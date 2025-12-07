# Hello World

The simplest possible handler.

## Code

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(_req: Request) -> Response {
    Response::ok(json!({
        "message": "Hello, World!"
    }))
}

handler_loop!(handle);
```

## Endpoint Configuration

| Setting | Value |
|---------|-------|
| Path | `/hello` |
| Method | `GET` |
| Domain | `*` |

## Test

```bash
curl http://localhost:9080/hello
```

## Response

```json
{
  "message": "Hello, World!"
}
```

## Variations

### With Request Info

```rust
fn handle(req: Request) -> Response {
    Response::ok(json!({
        "message": "Hello, World!",
        "method": req.method,
        "path": req.path,
        "request_id": req.request_id,
    }))
}
```

### Plain Text

```rust
fn handle(_req: Request) -> Response {
    Response::text(200, "Hello, World!")
}
```

### HTML

```rust
fn handle(_req: Request) -> Response {
    Response::new(200)
        .with_header("Content-Type", "text/html")
        .with_body("<h1>Hello, World!</h1>")
}
```

