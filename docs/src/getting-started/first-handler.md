# Your First Handler

This guide explains the structure of a handler and how to work with requests and responses.

## Handler Structure

Every handler follows the same pattern:

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    // Your logic here
    Response::ok(json!({"status": "success"}))
}

handler_loop!(handle);
```

### The Prelude

The `prelude` module imports everything you typically need:

```rust
use rust_edge_gateway_sdk::prelude::*;

// This imports:
// - Request, Response types
// - serde::{Deserialize, Serialize}
// - serde_json::{json, Value as JsonValue}
// - read_request, send_response IPC functions
// - HandlerError for error handling
```

### The Handler Function

Your handler function receives a `Request` and returns a `Response`:

```rust
fn handle(req: Request) -> Response {
    // Access request data
    let method = &req.method;  // "GET", "POST", etc.
    let path = &req.path;      // "/users/123"
    
    // Return a response
    Response::ok(json!({"received": path}))
}
```

### The Handler Loop Macro

The `handler_loop!` macro sets up the main function and IPC loop:

```rust
handler_loop!(handle);

// This expands to:
fn main() {
    loop {
        match read_request() {
            Ok(req) => {
                let response = handle(req);
                send_response(response).unwrap();
            }
            Err(_) => break,
        }
    }
}
```

## Working with Requests

### Accessing the Body

For POST/PUT requests, parse the JSON body:

```rust
#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

fn handle(req: Request) -> Response {
    // Parse JSON body
    let user: CreateUser = match req.json() {
        Ok(u) => u,
        Err(e) => return Response::bad_request(format!("Invalid JSON: {}", e)),
    };
    
    Response::created(json!({
        "id": "new-user-id",
        "name": user.name,
        "email": user.email,
    }))
}
```

### Path Parameters

Extract dynamic path segments (e.g., `/users/{id}`):

```rust
fn handle(req: Request) -> Response {
    let user_id = req.path_param("id")
        .ok_or_else(|| "Missing user ID")?;
    
    Response::ok(json!({"user_id": user_id}))
}
```

### Query Parameters

Access query string values (e.g., `?page=1&limit=10`):

```rust
fn handle(req: Request) -> Response {
    let page = req.query_param("page")
        .map(|s| s.parse::<u32>().unwrap_or(1))
        .unwrap_or(1);
    
    let limit = req.query_param("limit")
        .map(|s| s.parse::<u32>().unwrap_or(10))
        .unwrap_or(10);
    
    Response::ok(json!({
        "page": page,
        "limit": limit,
    }))
}
```

### Headers

Access HTTP headers (case-insensitive):

```rust
fn handle(req: Request) -> Response {
    let auth = req.header("Authorization");
    let content_type = req.header("Content-Type");
    
    if auth.is_none() {
        return Response::json(401, json!({"error": "Unauthorized"}));
    }
    
    Response::ok(json!({"authenticated": true}))
}
```

## Working with Responses

### JSON Responses

The most common response type:

```rust
// 200 OK with JSON
Response::ok(json!({"status": "success"}))

// 201 Created
Response::created(json!({"id": "123"}))

// Custom status with JSON
Response::json(418, json!({"error": "I'm a teapot"}))
```

### Error Responses

Built-in error response helpers:

```rust
Response::bad_request("Invalid input")      // 400
Response::not_found()                        // 404
Response::internal_error("Something broke")  // 500
```

### Custom Headers

Add headers to any response:

```rust
Response::ok(json!({"data": "value"}))
    .with_header("X-Custom-Header", "custom-value")
    .with_header("Cache-Control", "max-age=3600")
```

### Text Responses

For non-JSON responses:

```rust
Response::text(200, "Hello, World!")
Response::text(200, "<html><body>Hello</body></html>")
    .with_header("Content-Type", "text/html")
```

## Next Steps

- [Handler Lifecycle](./lifecycle.md) - Compilation and process management
- [Error Handling](../sdk/errors.md) - Structured error handling
- [Examples](../examples/hello-world.md) - More code examples

