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

## Methods

### `json<T>()`

Parse the body as JSON into a typed struct.

```rust
#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

fn handle(req: Request) -> Response {
    match req.json::<CreateUser>() {
        Ok(user) => Response::ok(json!({"name": user.name})),
        Err(e) => Response::bad_request(format!("Invalid JSON: {}", e)),
    }
}
```

### `query_param(key: &str)`

Get a query parameter by name.

```rust
fn handle(req: Request) -> Response {
    // URL: /search?q=rust&page=2
    let query = req.query_param("q").unwrap_or(&"".to_string());
    let page = req.query_param("page")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(1);
    
    Response::ok(json!({
        "query": query,
        "page": page,
    }))
}
```

### `path_param(key: &str)`

Get a path parameter extracted from the route pattern.

```rust
// Endpoint path: /users/{id}/posts/{post_id}
// Request: /users/123/posts/456

fn handle(req: Request) -> Response {
    let user_id = req.path_param("id");       // Some("123")
    let post_id = req.path_param("post_id");  // Some("456")
    
    Response::ok(json!({
        "user_id": user_id,
        "post_id": post_id,
    }))
}
```

### `header(key: &str)`

Get a header value (case-insensitive lookup).

```rust
fn handle(req: Request) -> Response {
    // These all work:
    let auth = req.header("Authorization");
    let auth = req.header("authorization");
    let auth = req.header("AUTHORIZATION");
    
    let content_type = req.header("Content-Type");
    let user_agent = req.header("User-Agent");
    
    Response::ok(json!({
        "has_auth": auth.is_some(),
    }))
}
```

### `is_method(method: &str)`

Check if the request method matches (case-insensitive).

```rust
fn handle(req: Request) -> Response {
    if req.is_method("GET") {
        return get_resource(&req);
    } else if req.is_method("POST") {
        return create_resource(&req);
    }
    
    Response::json(405, json!({"error": "Method not allowed"}))
}
```

## Examples

### Complete Request Handler

```rust
use rust_edge_gateway_sdk::prelude::*;

#[derive(Deserialize)]
struct UpdateProfile {
    name: Option<String>,
    bio: Option<String>,
}

fn handle(req: Request) -> Response {
    // Log request info
    eprintln!("[{}] {} {} from {:?}", 
        req.request_id, 
        req.method, 
        req.path,
        req.client_ip
    );
    
    // Check authentication
    let token = match req.header("Authorization") {
        Some(t) => t,
        None => return Response::json(401, json!({"error": "Unauthorized"})),
    };
    
    // Get path parameter
    let user_id = match req.path_param("id") {
        Some(id) => id.clone(),
        None => return Response::bad_request("Missing user ID"),
    };
    
    // Parse body
    let update: UpdateProfile = match req.json() {
        Ok(u) => u,
        Err(e) => return Response::bad_request(format!("Invalid JSON: {}", e)),
    };
    
    // Process request...
    Response::ok(json!({
        "user_id": user_id,
        "updated": true,
    }))
}

handler_loop!(handle);
```

