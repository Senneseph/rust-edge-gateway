# Error Handling

The SDK provides a `HandlerError` enum for structured error handling.

## HandlerError

```rust
pub enum HandlerError {
    IpcError(String),
    SerializationError(serde_json::Error),
    DatabaseError(String),
    RedisError(String),
    ServiceUnavailable(String),
    ValidationError(String),
    NotFound(String),
    Unauthorized(String),
    Internal(String),
}
```

## Error Variants

| Variant | Status Code | Use Case |
|---------|-------------|----------|
| `ValidationError` | 400 | Invalid input, missing fields |
| `Unauthorized` | 401 | Missing or invalid authentication |
| `NotFound` | 404 | Resource not found |
| `ServiceUnavailable` | 503 | Backend service down |
| `IpcError` | 500 | Internal IPC communication error |
| `SerializationError` | 500 | JSON serialization failed |
| `DatabaseError` | 500 | Database operation failed |
| `RedisError` | 500 | Redis operation failed |
| `Internal` | 500 | General internal error |

## Methods

### `status_code()`

Get the HTTP status code for this error:

```rust
let err = HandlerError::NotFound("User not found".to_string());
assert_eq!(err.status_code(), 404);
```

### `to_response()`

Convert the error to an HTTP Response:

```rust
let err = HandlerError::ValidationError("Invalid email".to_string());
let response = err.to_response();
// Response with status 400 and body: {"error": "Validation error: Invalid email"}
```

## Usage Patterns

### Result-Based Handlers

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    match process_request(&req) {
        Ok(data) => Response::ok(data),
        Err(e) => e.to_response(),
    }
}

fn process_request(req: &Request) -> Result<JsonValue, HandlerError> {
    // Validate input
    let body: CreateUser = req.json()
        .map_err(|e| HandlerError::ValidationError(e.to_string()))?;
    
    if body.email.is_empty() {
        return Err(HandlerError::ValidationError("Email is required".into()));
    }
    
    // Check authorization
    let token = req.header("Authorization")
        .ok_or_else(|| HandlerError::Unauthorized("Missing token".into()))?;
    
    // Simulate lookup
    if body.email == "notfound@example.com" {
        return Err(HandlerError::NotFound("User not found".into()));
    }
    
    Ok(json!({"id": "123", "email": body.email}))
}

#[derive(Deserialize)]
struct CreateUser {
    email: String,
}

handler_loop!(handle);
```

### Custom Error Types

You can define your own error types and convert to `HandlerError`:

```rust
enum AppError {
    UserNotFound(String),
    InvalidEmail,
    DatabaseDown,
    RateLimited,
}

impl From<AppError> for HandlerError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::UserNotFound(id) => 
                HandlerError::NotFound(format!("User {} not found", id)),
            AppError::InvalidEmail => 
                HandlerError::ValidationError("Invalid email format".into()),
            AppError::DatabaseDown => 
                HandlerError::ServiceUnavailable("Database unavailable".into()),
            AppError::RateLimited => 
                HandlerError::Internal("Rate limit exceeded".into()),
        }
    }
}

fn process(req: &Request) -> Result<JsonValue, AppError> {
    // Your logic returning AppError variants
    Err(AppError::InvalidEmail)
}

fn handle(req: Request) -> Response {
    match process(&req) {
        Ok(data) => Response::ok(data),
        Err(e) => HandlerError::from(e).to_response(),
    }
}
```

### Early Returns with ?

Use the `?` operator for clean error propagation:

```rust
fn process_request(req: &Request) -> Result<JsonValue, HandlerError> {
    // Each ? will return early if Err
    let input: InputData = req.json()
        .map_err(|e| HandlerError::ValidationError(e.to_string()))?;
    
    let user = find_user(&input.user_id)
        .ok_or_else(|| HandlerError::NotFound("User not found".into()))?;
    
    let result = update_user(&user)
        .map_err(|e| HandlerError::DatabaseError(e.to_string()))?;
    
    Ok(json!({"updated": true, "user": result}))
}
```

### Logging Errors

Always log errors for debugging:

```rust
fn handle(req: Request) -> Response {
    match process(&req) {
        Ok(data) => Response::ok(data),
        Err(e) => {
            // Log to stderr (captured by gateway)
            eprintln!("[{}] Error: {}", req.request_id, e);
            e.to_response()
        }
    }
}
```

