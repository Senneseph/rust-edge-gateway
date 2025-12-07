# Quick Start

This guide will help you create your first Rust Edge Gateway endpoint in under 5 minutes.

## Prerequisites

- Rust Edge Gateway running (either locally via Docker or deployed)
- Access to the Admin UI

## Step 1: Access the Admin UI

Navigate to your gateway's admin interface:

- **Local Development**: `http://localhost:9081/admin/`
- **Production**: `https://rust-edge-gateway.yourdomain.com/admin/`

## Step 2: Create an Endpoint

1. Click **"Create Endpoint"** or the **+** button
2. Fill in the endpoint details:

| Field | Example Value | Description |
|-------|---------------|-------------|
| **Name** | `hello-world` | Unique identifier for your endpoint |
| **Path** | `/hello` | The URL path to match |
| **Method** | `GET` | HTTP method (GET, POST, PUT, DELETE, etc.) |
| **Domain** | `*` | Domain to match (or `*` for all) |

3. Click **Save**

## Step 3: Write Handler Code

In the code editor, replace the default code with:

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    Response::ok(json!({
        "message": "Hello from Rust Edge Gateway!",
        "path": req.path,
        "method": req.method,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

handler_loop!(handle);
```

## Step 4: Compile

Click the **"Compile"** button. The gateway will:

1. Generate a Cargo project with your code
2. Compile it to a native binary
3. Report success or any compilation errors

You should see a success message like:
```
✓ Compiled successfully in 2.3s
```

## Step 5: Start the Endpoint

Click **"Start"** to activate the endpoint. The status should change to **Running**.

## Step 6: Test Your Endpoint

Make a request to your endpoint:

```bash
curl http://localhost:9080/hello
```

You should receive:

```json
{
  "message": "Hello from Rust Edge Gateway!",
  "path": "/hello",
  "method": "GET",
  "timestamp": "2024-01-15T10:30:00.000Z"
}
```

## What's Next?

- [Your First Handler](./first-handler.md) - Deeper dive into handler structure
- [Handler Lifecycle](./lifecycle.md) - Understand compilation and execution
- [Request API](../sdk/request.md) - Access headers, body, parameters
- [Response API](../sdk/response.md) - Build JSON, text, and custom responses
- [Examples](../examples/hello-world.md) - More code examples

## Troubleshooting

### Compilation Errors

Check the error message for:
- Missing dependencies (add to your handler's `use` statements)
- Syntax errors (Rust compiler messages are helpful!)
- Type mismatches

### Endpoint Not Responding

1. Check the endpoint is in **Running** status
2. Verify the path matches exactly (paths are case-sensitive)
3. Check the method matches your request
4. View endpoint logs in the admin UI

### Handler Crashes

View the logs to see panic messages or error output. Common causes:
- Unwrapping `None` or `Err` values
- Stack overflow from deep recursion
- Accessing invalid JSON fields

