# Handler Lifecycle

Understanding how handlers are compiled, started, and managed helps you write more reliable code.

## Endpoint States

An endpoint can be in one of these states:

| State | Description |
|-------|-------------|
| **Created** | Endpoint defined but code not yet compiled |
| **Compiled** | Code compiled successfully, ready to start |
| **Running** | Worker process is active and handling requests |
| **Stopped** | Worker process stopped, can be restarted |
| **Error** | Compilation or runtime error occurred |

## Compilation

When you click "Compile", the gateway:

1. **Creates a Cargo project** in the handlers directory
2. **Writes your code** to `src/main.rs`
3. **Generates Cargo.toml** with the SDK dependency
4. **Runs `cargo build --release`** to compile
5. **Stores the binary** for execution

### Generated Project Structure

```
handlers/
└── {endpoint-id}/
    ├── Cargo.toml
    ├── Cargo.lock
    ├── src/
    │   └── main.rs    # Your handler code
    └── target/
        └── release/
            └── handler  # Compiled binary
```

### Cargo.toml

The generated Cargo.toml includes the SDK:

```toml
[package]
name = "handler"
version = "0.1.0"
edition = "2021"

[dependencies]
rust-edge-gateway-sdk = { path = "../../crates/rust-edge-gateway-sdk" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

You can request additional dependencies by using them in your code - the gateway will detect common crates like `chrono`, `uuid`, `regex`, etc.

## Worker Processes

### Starting a Handler

When you start an endpoint:

1. Gateway spawns the compiled binary as a child process
2. IPC channels are established (stdin/stdout)
3. Worker enters its request loop
4. Status changes to "Running"

### Request Flow

```
┌─────────┐     ┌──────────┐     ┌────────┐
│ Gateway │────▶│  stdin   │────▶│ Worker │
│         │     │ (request)│     │        │
│         │◀────│  stdout  │◀────│        │
│         │     │(response)│     │        │
└─────────┘     └──────────┘     └────────┘
```

The IPC protocol uses length-prefixed JSON:
- 4 bytes: message length (big-endian u32)
- N bytes: JSON payload

### Worker Loop

Your handler runs in a loop:

```rust
loop {
    // 1. Read request from stdin
    let request = read_request()?;
    
    // 2. Call your handler function
    let response = handle(request);
    
    // 3. Write response to stdout
    send_response(response)?;
}
```

The loop exits when:
- stdin is closed (gateway stopped the worker)
- An IPC error occurs
- The process is killed

### Stopping a Handler

When you stop an endpoint:

1. Gateway closes the stdin pipe
2. Worker's read_request() returns an error
3. Worker exits cleanly
4. Gateway waits for process exit
5. Status changes to "Stopped"

## Hot Reload

Rust Edge Gateway supports hot reloading:

1. **Edit code** in the admin UI
2. **Compile** the new version
3. **Restart** the endpoint
   - Old worker finishes current request
   - New worker starts with updated code

No gateway restart required!

## Error Handling

### Compilation Errors

If compilation fails:
- Error message is captured and displayed
- Endpoint stays in previous state
- Previous binary (if any) remains available

### Runtime Errors

If your handler panics:
- Gateway detects the worker exit
- Error is logged
- Worker can be restarted
- Endpoint moves to "Error" state

### Graceful Error Handling

Always handle errors in your code:

```rust
fn handle(req: Request) -> Response {
    match process_request(&req) {
        Ok(data) => Response::ok(data),
        Err(e) => e.to_response(), // HandlerError -> Response
    }
}

fn process_request(req: &Request) -> Result<JsonValue, HandlerError> {
    let body: MyInput = req.json()
        .map_err(|e| HandlerError::ValidationError(e.to_string()))?;
    
    // ... process ...
    
    Ok(json!({"result": "success"}))
}
```

