# Error Handling

The SDK provides a `HandlerError` enum for structured error handling with automatic HTTP status code mapping.

## Quick Reference

| Variant | Status | Use Case |
|---------|--------|----------|
| `BadRequest(msg)` | 400 | Invalid input, malformed JSON |
| `ValidationError(msg)` | 400 | Semantic validation failures |
| `Unauthorized(msg)` | 401 | Missing or invalid auth |
| `Forbidden(msg)` | 403 | Authenticated but not authorized |
| `NotFound` / `NotFoundMessage(msg)` | 404 | Resource not found |
| `MethodNotAllowed(msg)` | 405 | Wrong HTTP method |
| `Conflict(msg)` | 409 | Resource conflict (duplicate) |
| `PayloadTooLarge(msg)` | 413 | Request body too large |
| `Internal(msg)` / `InternalError(msg)` | 500 | Server error |
| `DatabaseError(msg)` | 500 | Database operation failed |
| `StorageError(msg)` | 500 | Storage operation failed |
| `ServiceUnavailable(msg)` | 503 | Backend service down |

## HandlerError Definition

```rust
pub enum HandlerError {
    // 4xx Client Errors
    BadRequest(String),
    ValidationError(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound,
    NotFoundMessage(String),
    MethodNotAllowed(String),
    Conflict(String),
    PayloadTooLarge(String),

    // 5xx Server Errors
    IpcError(String),
    SerializationError(serde_json::Error),
    DatabaseError(String),
    RedisError(String),
    StorageError(String),
    InternalError(String),
    Internal(String),
    ServiceUnavailable(String),
}
```

## Key Features

### Automatic Response Conversion

`HandlerError` implements `From<HandlerError> for Response`, so you can use `.into()`:

```rust
fn handle(req: Request) -> Response {
    match process(&req) {
        Ok(data) => Response::ok(data),
        Err(e) => e.into(),  // Automatically converts to Response
    }
}
```

### Use with handler_loop_result!

The `handler_loop_result!` macro automatically converts errors to responses:

```rust
fn handle(req: Request) -> Result<Response, HandlerError> {
    let data: MyInput = req.json()?;  // BadRequest on parse failure
    let id: i64 = req.require_path_param("id")?;  // BadRequest if missing
    let auth = req.require_header("Authorization")?;  // BadRequest if missing

    Ok(Response::ok(json!({"id": id, "data": data})))
}

handler_loop_result!(handle);  // Errors auto-convert to HTTP responses
```

## Methods

### `status_code() -> u16`

Get the HTTP status code for this error:

```rust
let err = HandlerError::NotFound;
assert_eq!(err.status_code(), 404);

let err = HandlerError::BadRequest("Invalid input".into());
assert_eq!(err.status_code(), 400);
```

### `to_response() -> Response`

Convert the error to an HTTP Response:

```rust
let err = HandlerError::ValidationError("Invalid email".to_string());
let response = err.to_response();
// Response { status: 400, body: {"error": "Validation error: Invalid email"} }
```

## Usage Patterns

### Clean Result-Based Handlers

```rust
use rust_edge_gateway_sdk::prelude::*;

#[derive(Deserialize)]
struct CreateUser {
    email: String,
    name: String,
}

fn handle(req: Request) -> Result<Response, HandlerError> {
    // Parse body - returns BadRequest on failure
    let body: CreateUser = req.json()?;

    // Validate
    if body.email.is_empty() {
        return Err(HandlerError::ValidationError("Email is required".into()));
    }

    // Check authentication
    let token = req.require_header("Authorization")?;

    // Get typed path parameter
    let user_id: i64 = req.require_path_param("id")?;

    // Simulate database operation
    let user = find_user(user_id)
        .ok_or(HandlerError::NotFound)?;

    Ok(Response::ok(user))
}

handler_loop_result!(handle);
```

### Converting External Errors

Map external library errors to `HandlerError`:

```rust
fn save_to_database(data: &MyData) -> Result<i64, HandlerError> {
    let conn = get_connection()
        .map_err(|e| HandlerError::DatabaseError(e.to_string()))?;

    let id = conn.insert(data)
        .map_err(|e| HandlerError::DatabaseError(e.to_string()))?;

    Ok(id)
}
```

### Custom Error Types

Define domain-specific errors and convert to `HandlerError`:

```rust
enum AppError {
    UserNotFound(String),
    DuplicateEmail,
    InvalidCredentials,
    RateLimited,
}

impl From<AppError> for HandlerError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::UserNotFound(id) =>
                HandlerError::NotFoundMessage(format!("User {} not found", id)),
            AppError::DuplicateEmail =>
                HandlerError::Conflict("Email already registered".into()),
            AppError::InvalidCredentials =>
                HandlerError::Unauthorized("Invalid email or password".into()),
            AppError::RateLimited =>
                HandlerError::ServiceUnavailable("Rate limit exceeded, try again later".into()),
        }
    }
}

fn handle(req: Request) -> Result<Response, HandlerError> {
    let result = business_logic(&req)
        .map_err(HandlerError::from)?;  // Convert AppError to HandlerError
    Ok(Response::ok(result))
}
```

### Async Error Handling

Works the same with async handlers:

```rust
async fn handle(req: Request) -> Result<Response, HandlerError> {
    let data: CreateItem = req.json()?;

    let id = database.insert(&data).await
        .map_err(|e| HandlerError::DatabaseError(e.to_string()))?;

    let uploaded = s3.put_object(&data.file).await
        .map_err(|e| HandlerError::StorageError(e.to_string()))?;

    Ok(Response::created(json!({
        "id": id,
        "file_url": uploaded.url
    })))
}

handler_loop_async_result!(handle);
```

### Logging Errors

Always log errors for debugging (logs go to stderr, captured by gateway):

```rust
fn handle(req: Request) -> Result<Response, HandlerError> {
    match process(&req) {
        Ok(data) => Ok(Response::ok(data)),
        Err(e) => {
            eprintln!("[{}] Error: {} ({})",
                req.request_id,
                e,
                e.status_code()
            );
            Err(e)  // Will be converted to Response by handler_loop_result!
        }
    }
}
```

## Best Practices

1. **Use `handler_loop_result!`** - Simplifies error handling with automatic conversion
2. **Use specific error variants** - `BadRequest` vs `ValidationError` vs `Unauthorized`
3. **Always log errors** - Use `eprintln!` for debugging
4. **Convert early** - Map external errors to `HandlerError` at the boundary
5. **Include context** - Error messages should help debugging

