# Handler Macros

The SDK provides several macros for running handler loops with different patterns.

## Quick Reference

| Macro | Handler Signature | Use Case |
|-------|-------------------|----------|
| `handler_loop!` | `fn(Request) -> Response` | Simple sync handlers |
| `handler_loop_result!` | `fn(Request) -> Result<Response, HandlerError>` | Sync with error handling |
| `handler_loop_async!` | `async fn(Request) -> Response` | Async handlers |
| `handler_loop_async_result!` | `async fn(Request) -> Result<Response, HandlerError>` | Async with error handling |

## Sync Handlers

### `handler_loop!`

For simple synchronous handlers that return a `Response` directly.

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    Response::ok(json!({"path": req.path, "method": req.method}))
}

handler_loop!(handle);
```

### `handler_loop_result!`

For handlers that return `Result<Response, HandlerError>`. Errors are automatically converted to HTTP responses.

```rust
use rust_edge_gateway_sdk::prelude::*;

#[derive(Deserialize)]
struct CreateItem {
    name: String,
    price: f64,
}

fn handle(req: Request) -> Result<Response, HandlerError> {
    // These all use ? operator - errors become HTTP responses
    let auth = req.require_header("Authorization")?;
    let item: CreateItem = req.json()?;
    let id: i64 = req.require_path_param("id")?;
    
    if item.price < 0.0 {
        return Err(HandlerError::ValidationError("Price cannot be negative".into()));
    }
    
    Ok(Response::created(json!({"id": id, "item": item.name})))
}

handler_loop_result!(handle);
```

## Async Handlers

Async handlers require the `async` feature to be enabled.

### Cargo.toml

```toml
[dependencies]
rust-edge-gateway-sdk = { git = "https://github.com/user/rust-edge-gateway", features = ["async"] }
tokio = { version = "1", features = ["full"] }
```

### `handler_loop_async!`

For async handlers that return a `Response` directly. A Tokio runtime is created automatically.

```rust
use rust_edge_gateway_sdk::prelude::*;

async fn handle(req: Request) -> Response {
    let data = fetch_from_api().await;
    Response::ok(data)
}

handler_loop_async!(handle);
```

### `handler_loop_async_result!`

For async handlers with Result-based error handling. Combines async support with automatic error conversion.

```rust
use rust_edge_gateway_sdk::prelude::*;

async fn handle(req: Request) -> Result<Response, HandlerError> {
    let data: CreateItem = req.json()?;
    
    // Async database call
    let id = database.insert(&data).await
        .map_err(|e| HandlerError::DatabaseError(e.to_string()))?;
    
    // Async file upload
    let url = s3.upload(&data.file).await
        .map_err(|e| HandlerError::StorageError(e.to_string()))?;
    
    Ok(Response::created(json!({
        "id": id,
        "file_url": url
    })))
}

handler_loop_async_result!(handle);
```

## Runtime Management

**Important:** The async macros create a single Tokio runtime that persists across all requests. This is more efficient than creating a runtime per request.

```rust
// DON'T do this - creates runtime per request (inefficient):
fn handle(req: Request) -> Response {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // async work
    })
}

// DO this - runtime is created once by the macro:
async fn handle(req: Request) -> Response {
    // async work directly
}
handler_loop_async!(handle);
```

## Choosing the Right Macro

| Your handler needs... | Use this macro |
|-----------------------|----------------|
| Simple sync logic | `handler_loop!` |
| Sync with `?` operator | `handler_loop_result!` |
| Async operations (DB, HTTP, files) | `handler_loop_async!` |
| Async with `?` operator | `handler_loop_async_result!` |

## Example: Complete CRUD Handler

```rust
use rust_edge_gateway_sdk::prelude::*;

async fn handle(req: Request) -> Result<Response, HandlerError> {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/items") => list_items().await,
        ("POST", "/items") => create_item(&req).await,
        ("GET", _) if req.path.starts_with("/items/") => get_item(&req).await,
        ("DELETE", _) if req.path.starts_with("/items/") => delete_item(&req).await,
        _ => Err(HandlerError::MethodNotAllowed("Use GET, POST, or DELETE".into())),
    }
}

async fn list_items() -> Result<Response, HandlerError> {
    let items = db::get_all_items().await
        .map_err(|e| HandlerError::DatabaseError(e.to_string()))?;
    Ok(Response::ok(json!({"items": items})))
}

async fn create_item(req: &Request) -> Result<Response, HandlerError> {
    let item: NewItem = req.json()?;
    let id = db::insert_item(&item).await
        .map_err(|e| HandlerError::DatabaseError(e.to_string()))?;
    Ok(Response::created(json!({"id": id})))
}

handler_loop_async_result!(handle);
```

