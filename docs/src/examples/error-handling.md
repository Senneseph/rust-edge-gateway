# Error Handling

Robust error handling patterns for handlers.

## Basic Pattern

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    match process_request(&req) {
        Ok(data) => Response::ok(data),
        Err(e) => e.to_response(),
    }
}

fn process_request(req: &Request) -> Result<JsonValue, HandlerError> {
    let input: CreateUser = req.json()
        .map_err(|e| HandlerError::ValidationError(e.to_string()))?;
    
    // Process and return result
    Ok(json!({"id": "123", "name": input.name}))
}

#[derive(Deserialize)]
struct CreateUser {
    name: String,
}

handler_loop!(handle);
```

## Error Types and Status Codes

```rust
fn process(req: &Request) -> Result<JsonValue, HandlerError> {
    // 400 Bad Request - Invalid input
    if req.body.is_none() {
        return Err(HandlerError::ValidationError("Body required".into()));
    }
    
    // 401 Unauthorized - Missing/invalid auth
    if req.header("Authorization").is_none() {
        return Err(HandlerError::Unauthorized("Token required".into()));
    }
    
    // 404 Not Found - Resource doesn't exist
    let user = find_user("123");
    if user.is_none() {
        return Err(HandlerError::NotFound("User not found".into()));
    }
    
    // 503 Service Unavailable - Backend down
    if !database_available() {
        return Err(HandlerError::ServiceUnavailable("Database down".into()));
    }
    
    // 500 Internal Error - Unexpected error
    if something_broke() {
        return Err(HandlerError::Internal("Unexpected error".into()));
    }
    
    Ok(json!({"status": "ok"}))
}
```

## Input Validation

```rust
#[derive(Deserialize)]
struct RegisterUser {
    email: String,
    password: String,
    name: String,
}

fn validate_input(input: &RegisterUser) -> Result<(), HandlerError> {
    // Email validation
    if !input.email.contains('@') {
        return Err(HandlerError::ValidationError(
            "Invalid email format".into()
        ));
    }
    
    // Password validation
    if input.password.len() < 8 {
        return Err(HandlerError::ValidationError(
            "Password must be at least 8 characters".into()
        ));
    }
    
    // Name validation
    if input.name.trim().is_empty() {
        return Err(HandlerError::ValidationError(
            "Name is required".into()
        ));
    }
    
    Ok(())
}

fn handle(req: Request) -> Response {
    let input: RegisterUser = match req.json() {
        Ok(i) => i,
        Err(e) => return Response::bad_request(format!("Invalid JSON: {}", e)),
    };
    
    if let Err(e) = validate_input(&input) {
        return e.to_response();
    }
    
    Response::created(json!({"email": input.email}))
}
```

## Custom Error Type

```rust
enum AppError {
    UserNotFound(String),
    EmailTaken(String),
    InvalidCredentials,
    RateLimited,
    DatabaseError(String),
}

impl From<AppError> for HandlerError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::UserNotFound(id) => 
                HandlerError::NotFound(format!("User {} not found", id)),
            AppError::EmailTaken(email) => 
                HandlerError::ValidationError(format!("Email {} already registered", email)),
            AppError::InvalidCredentials => 
                HandlerError::Unauthorized("Invalid email or password".into()),
            AppError::RateLimited => 
                HandlerError::Internal("Rate limit exceeded".into()),
            AppError::DatabaseError(msg) => 
                HandlerError::DatabaseError(msg),
        }
    }
}

fn process(req: &Request) -> Result<JsonValue, AppError> {
    // Business logic with custom errors
    Err(AppError::InvalidCredentials)
}

fn handle(req: Request) -> Response {
    match process(&req) {
        Ok(data) => Response::ok(data),
        Err(e) => HandlerError::from(e).to_response(),
    }
}
```

## Logging Errors

Always log errors for debugging:

```rust
fn handle(req: Request) -> Response {
    match process(&req) {
        Ok(data) => Response::ok(data),
        Err(e) => {
            // Log with request ID for tracing
            eprintln!("[{}] Error: {}", req.request_id, e);
            
            // Log stack trace for internal errors
            if matches!(e, HandlerError::Internal(_) | HandlerError::DatabaseError(_)) {
                eprintln!("[{}] Request path: {}", req.request_id, req.path);
                eprintln!("[{}] Request body: {:?}", req.request_id, req.body);
            }
            
            e.to_response()
        }
    }
}
```

## Graceful Degradation

Handle service failures gracefully:

```rust
fn handle(req: Request) -> Response {
    let redis = RedisPool { pool_id: "cache".to_string() };
    let db = DbPool { pool_id: "main".to_string() };
    
    // Try cache first
    let cached = redis.get("data:key");
    
    match cached {
        Ok(Some(data)) => {
            return Response::ok(json!({"source": "cache", "data": data}));
        }
        Ok(None) => { /* Cache miss, continue */ }
        Err(e) => {
            // Log but don't fail - Redis being down shouldn't break the app
            eprintln!("Redis error (non-fatal): {}", e);
        }
    }
    
    // Fallback to database
    match db.query("SELECT * FROM data", &[]) {
        Ok(result) => Response::ok(json!({"source": "db", "data": result.rows})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            Response::json(503, json!({
                "error": "Service temporarily unavailable",
                "retry_after": 5,
            }))
        }
    }
}
```

