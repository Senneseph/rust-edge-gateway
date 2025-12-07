# Response

The `Response` struct represents an outgoing HTTP response.

## Definition

```rust
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}
```

## Fields

| Field | Type | Description |
|-------|------|-------------|
| `status` | `u16` | HTTP status code (200, 404, 500, etc.) |
| `headers` | `HashMap<String, String>` | Response headers |
| `body` | `Option<String>` | Response body content |

## Constructor Methods

### `new(status: u16)`

Create a response with just a status code (no body).

```rust
Response::new(204)  // 204 No Content
```

### `ok<T: Serialize>(body: T)`

Create a 200 OK response with JSON body.

```rust
Response::ok(json!({"message": "Success"}))
Response::ok(json!({"users": users}))
Response::ok(my_struct)  // If my_struct implements Serialize
```

### `json<T: Serialize>(status: u16, body: T)`

Create a JSON response with a custom status code.

```rust
Response::json(201, json!({"id": "new-id"}))
Response::json(400, json!({"error": "Invalid input"}))
Response::json(403, json!({"error": "Forbidden"}))
```

### `text(status: u16, body: impl Into<String>)`

Create a plain text response.

```rust
Response::text(200, "Hello, World!")
Response::text(500, "Internal Server Error")
```

### `created<T: Serialize>(body: T)`

Create a 201 Created response with JSON body.

```rust
Response::created(json!({
    "id": "12345",
    "name": "New Resource",
}))
```

### `no_content()`

Create a 204 No Content response.

```rust
Response::no_content()  // Used for DELETE or updates with no return data
```

## Error Response Helpers

### `not_found()`

Create a 404 Not Found response.

```rust
Response::not_found()
// Returns: {"error": "Not Found"}
```

### `bad_request(message: impl Into<String>)`

Create a 400 Bad Request response.

```rust
Response::bad_request("Missing required field: email")
Response::bad_request(format!("Invalid value for field: {}", field))
```

### `internal_error(message: impl Into<String>)`

Create a 500 Internal Server Error response.

```rust
Response::internal_error("Database connection failed")
```

## Builder Methods

### `with_header(key, value)`

Add a header to the response.

```rust
Response::ok(json!({"data": "value"}))
    .with_header("Cache-Control", "max-age=3600")
    .with_header("X-Custom-Header", "custom-value")
```

### `with_body(body: impl Into<String>)`

Set or replace the response body.

```rust
Response::new(200)
    .with_header("Content-Type", "text/html")
    .with_body("<html><body>Hello!</body></html>")
```

## Common Patterns

### RESTful Responses

```rust
// GET - Return resource
Response::ok(json!({"id": 1, "name": "Item"}))

// POST - Return created resource
Response::created(json!({"id": 1, "name": "New Item"}))

// PUT - Return updated resource
Response::ok(json!({"id": 1, "name": "Updated Item"}))

// DELETE - No content
Response::no_content()
```

### Error Handling Pattern

```rust
fn handle(req: Request) -> Response {
    match process(&req) {
        Ok(data) => Response::ok(data),
        Err(e) => match e {
            AppError::NotFound => Response::not_found(),
            AppError::Validation(msg) => Response::bad_request(msg),
            AppError::Unauthorized => Response::json(401, json!({"error": "Unauthorized"})),
            AppError::Internal(msg) => Response::internal_error(msg),
        }
    }
}
```

### Custom Content Types

```rust
// HTML
Response::new(200)
    .with_header("Content-Type", "text/html; charset=utf-8")
    .with_body("<h1>Hello</h1>")

// XML
Response::new(200)
    .with_header("Content-Type", "application/xml")
    .with_body("<root><item>Value</item></root>")

// CSV
Response::new(200)
    .with_header("Content-Type", "text/csv")
    .with_header("Content-Disposition", "attachment; filename=\"data.csv\"")
    .with_body("id,name\n1,Alice\n2,Bob")
```

### Redirect

```rust
// Temporary redirect (307)
Response::new(307)
    .with_header("Location", "https://example.com/new-path")

// Permanent redirect (308)
Response::new(308)
    .with_header("Location", "https://example.com/permanent-path")
```

### CORS Headers

```rust
Response::ok(json!({"data": "value"}))
    .with_header("Access-Control-Allow-Origin", "*")
    .with_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
    .with_header("Access-Control-Allow-Headers", "Content-Type, Authorization")
```

