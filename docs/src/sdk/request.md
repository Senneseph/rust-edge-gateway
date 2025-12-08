# Request

The `Request` struct represents an incoming HTTP request.

## Definition

```rust
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub params: HashMap<String, String>,
    pub client_ip: Option<String>,
    pub request_id: String,
}
```

## Fields

| Field | Type | Description |
|-------|------|-------------|
| `method` | `String` | HTTP method: GET, POST, PUT, DELETE, PATCH, etc. |
| `path` | `String` | Request path, e.g., `/users/123` |
| `query` | `HashMap<String, String>` | Query parameters from the URL |
| `headers` | `HashMap<String, String>` | HTTP headers |
| `body` | `Option<String>` | Request body (for POST, PUT, PATCH) |
| `params` | `HashMap<String, String>` | Path parameters extracted from the route |
| `client_ip` | `Option<String>` | Client's IP address |
| `request_id` | `String` | Unique identifier for request tracing |

## Methods Reference

### JSON Parsing

#### `json<T>() -> Result<T, HandlerError>`

Parse the body as JSON into a typed struct. Returns `HandlerError::BadRequest` on parse failure.

```rust
#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

fn handle(req: Request) -> Result<Response, HandlerError> {
    let user: CreateUser = req.json()?;  // Uses ? operator naturally
    Ok(Response::ok(json!({"name": user.name})))
}

handler_loop_result!(handle);
```

### Query Parameters

#### `query_param(key: &str) -> Option<&String>`

Get a query parameter as a string reference.

```rust
// URL: /search?q=rust
let query = req.query_param("q"); // Some(&"rust".to_string())
```

#### `query_param_as<T: FromStr>(key: &str) -> Option<T>`

Get a query parameter parsed as a specific type. Returns `None` if missing or can't be parsed.

```rust
// URL: /items?page=2&limit=10
let page: i64 = req.query_param_as("page").unwrap_or(1);
let limit: usize = req.query_param_as("limit").unwrap_or(20);
let active: bool = req.query_param_as("active").unwrap_or(false);
```

#### `require_query_param<T: FromStr>(key: &str) -> Result<T, HandlerError>`

Get a required query parameter. Returns `HandlerError::BadRequest` if missing or invalid.

```rust
fn handle(req: Request) -> Result<Response, HandlerError> {
    let page: i64 = req.require_query_param("page")?;
    let limit: usize = req.require_query_param("limit")?;
    Ok(Response::ok(json!({"page": page, "limit": limit})))
}
```

### Path Parameters

#### `path_param(key: &str) -> Option<&String>`

Get a path parameter extracted from the route pattern.

```rust
// Endpoint path: /users/{id}/posts/{post_id}
// Request: /users/123/posts/456

let user_id = req.path_param("id");       // Some(&"123".to_string())
let post_id = req.path_param("post_id");  // Some(&"456".to_string())
```

#### `path_param_as<T: FromStr>(key: &str) -> Option<T>`

Get a path parameter parsed as a specific type.

```rust
// Route: /users/{id}
let user_id: i64 = req.path_param_as("id").unwrap_or(0);
let uuid: Uuid = req.path_param_as("id").ok_or(HandlerError::BadRequest("Invalid UUID".into()))?;
```

#### `require_path_param<T: FromStr>(key: &str) -> Result<T, HandlerError>`

Get a required path parameter. Returns `HandlerError::BadRequest` if missing or invalid.

```rust
fn handle(req: Request) -> Result<Response, HandlerError> {
    let user_id: i64 = req.require_path_param("id")?;
    Ok(Response::ok(json!({"user_id": user_id})))
}
```

### Headers

#### `header(key: &str) -> Option<&String>`

Get a header value (case-insensitive lookup).

```rust
// All of these work:
let auth = req.header("Authorization");
let auth = req.header("authorization");
let auth = req.header("AUTHORIZATION");
```

#### `require_header(key: &str) -> Result<&String, HandlerError>`

Get a required header. Returns `HandlerError::BadRequest` if missing.

```rust
fn handle(req: Request) -> Result<Response, HandlerError> {
    let auth = req.require_header("Authorization")?;
    let api_key = req.require_header("X-API-Key")?;
    Ok(Response::ok(json!({"authenticated": true})))
}
```

### Request Inspection

#### `is_method(method: &str) -> bool`

Check if the request method matches (case-insensitive).

```rust
if req.is_method("POST") {
    // Handle POST
}
```

#### `content_type() -> Option<&String>`

Get the Content-Type header value.

```rust
if let Some(ct) = req.content_type() {
    eprintln!("Content-Type: {}", ct);
}
```

#### `is_json() -> bool`

Check if the request has a JSON content type.

```rust
if req.is_json() {
    let data: MyStruct = req.json()?;
}
```

#### `is_multipart() -> bool`

Check if the request has a multipart/form-data content type.

```rust
if req.is_multipart() {
    let multipart = req.multipart()?;
}
```

#### `body_bytes() -> Vec<u8>`

Get the raw body as bytes.

```rust
let raw_body = req.body_bytes();
```

### Multipart Form Data

#### `multipart() -> Result<MultipartData, HandlerError>`

Parse multipart/form-data body for file uploads.

```rust
fn handle(req: Request) -> Result<Response, HandlerError> {
    let multipart = req.multipart()?;

    // Get text fields
    let title = multipart.require_field("title")?;
    let description = multipart.field("description").unwrap_or(&"".to_string());

    // Get uploaded file
    let file = multipart.require_file("upload")?;
    eprintln!("Received file: {} ({} bytes)", file.filename, file.data.len());

    Ok(Response::ok(json!({
        "title": title,
        "filename": file.filename,
        "size": file.data.len()
    })))
}
```

## MultipartData

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `fields` | `HashMap<String, String>` | Text form fields |
| `files` | `HashMap<String, MultipartFile>` | Uploaded files |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `field(name)` | `Option<&String>` | Get a text field |
| `require_field(name)` | `Result<&String, HandlerError>` | Get required text field |
| `file(name)` | `Option<&MultipartFile>` | Get an uploaded file |
| `require_file(name)` | `Result<&MultipartFile, HandlerError>` | Get required file |

## MultipartFile

```rust
pub struct MultipartFile {
    pub filename: String,      // Original filename
    pub content_type: String,  // MIME type
    pub data: Vec<u8>,         // File content
}
```

## Complete Example

```rust
use rust_edge_gateway_sdk::prelude::*;

#[derive(Deserialize)]
struct UpdateProfile {
    name: Option<String>,
    bio: Option<String>,
}

fn handle(req: Request) -> Result<Response, HandlerError> {
    // Log request info
    eprintln!("[{}] {} {} from {:?}",
        req.request_id, req.method, req.path, req.client_ip);

    // Check authentication
    let token = req.require_header("Authorization")?;

    // Get path parameter (typed)
    let user_id: i64 = req.require_path_param("id")?;

    // Parse body
    let update: UpdateProfile = req.json()?;

    // Process request...
    Ok(Response::ok(json!({
        "user_id": user_id,
        "updated": true,
    })))
}

handler_loop_result!(handle);
```

