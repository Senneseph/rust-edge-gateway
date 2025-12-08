# Response

The `Response` struct represents an outgoing HTTP response.

## Quick Reference

| Method | Status | Use Case |
|--------|--------|----------|
| `ok(body)` | 200 | Successful GET/PUT |
| `created(body)` | 201 | Successful POST |
| `accepted(body)` | 202 | Async operation started |
| `no_content()` | 204 | Successful DELETE |
| `bad_request(msg)` | 400 | Invalid input |
| `unauthorized(msg)` | 401 | Missing/invalid auth |
| `forbidden(msg)` | 403 | Not authorized |
| `not_found()` | 404 | Resource not found |
| `conflict(msg)` | 409 | Resource conflict |
| `internal_error(msg)` | 500 | Server error |
| `service_unavailable(msg)` | 503 | Backend down |

## Definition

```rust
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}
```

## Constructor Methods

### `new(status: u16)`

Create a response with just a status code (no body).

```rust
Response::new(204)  // 204 No Content
Response::new(301).with_header("Location", "/new-path")
```

### `ok<T: Serialize>(body: T)`

Create a 200 OK response with JSON body.

```rust
Response::ok(json!({"message": "Success"}))
Response::ok(my_struct)  // If my_struct implements Serialize
```

### `json<T: Serialize>(status: u16, body: T)`

Create a JSON response with a custom status code.

```rust
Response::json(201, json!({"id": "new-id"}))
Response::json(400, json!({"error": "Invalid input"}))
```

### `text(status: u16, body: impl Into<String>)`

Create a plain text response.

```rust
Response::text(200, "Hello, World!")
Response::text(500, "Internal Server Error")
```

### `html(status: u16, body: impl Into<String>)`

Create an HTML response.

```rust
Response::html(200, "<html><body><h1>Hello!</h1></body></html>")
```

### `binary(status: u16, data: impl AsRef<[u8]>, content_type: impl Into<String>)`

Create a binary response for files, images, etc.

```rust
// Serve an image
Response::binary(200, image_bytes, "image/png")

// Serve a PDF with download prompt
Response::binary(200, pdf_bytes, "application/pdf")
    .with_header("Content-Disposition", "attachment; filename=\"report.pdf\"")

// Inline image
Response::binary(200, jpeg_bytes, "image/jpeg")
    .with_header("Content-Disposition", "inline; filename=\"photo.jpg\"")
```

### `created<T: Serialize>(body: T)`

Create a 201 Created response.

```rust
Response::created(json!({"id": "12345", "name": "New Resource"}))
```

### `accepted<T: Serialize>(body: T)`

Create a 202 Accepted response (for async operations).

```rust
Response::accepted(json!({"job_id": "abc123", "status": "processing"}))
```

### `no_content()`

Create a 204 No Content response.

```rust
Response::no_content()  // Used for DELETE
```

### `redirect(status: u16, location: impl Into<String>)`

Create a redirect response.

```rust
Response::redirect(302, "https://example.com/new-location")
Response::redirect(301, "/permanent-new-path")
```

## Error Response Helpers

### `bad_request(message: impl Into<String>)`

400 Bad Request.

```rust
Response::bad_request("Missing required field: email")
```

### `unauthorized(message: impl Into<String>)`

401 Unauthorized.

```rust
Response::unauthorized("Invalid or expired token")
```

### `forbidden(message: impl Into<String>)`

403 Forbidden.

```rust
Response::forbidden("You don't have permission to access this resource")
```

### `not_found()` / `not_found_msg(message)`

404 Not Found.

```rust
Response::not_found()
Response::not_found_msg("User with ID 123 not found")
```

### `conflict(message: impl Into<String>)`

409 Conflict.

```rust
Response::conflict("A user with this email already exists")
```

### `internal_error(message: impl Into<String>)`

500 Internal Server Error.

```rust
Response::internal_error("Database connection failed")
```

### `service_unavailable(message: impl Into<String>)`

503 Service Unavailable.

```rust
Response::service_unavailable("Database is currently unavailable")
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

### `with_cors(origin: impl Into<String>)`

Add CORS headers for cross-origin requests.

```rust
Response::ok(data).with_cors("*")
Response::ok(data).with_cors("https://myapp.com")
```

### `with_cache(max_age_seconds: u32)`

Add caching headers.

```rust
Response::ok(data).with_cache(3600)  // Cache for 1 hour
Response::ok(data).with_cache(0)     // No cache
```

## Common Patterns

### RESTful API

```rust
// GET /items - List items
Response::ok(json!({"items": items}))

// GET /items/{id} - Get single item
Response::ok(item)  // or Response::not_found()

// POST /items - Create item
Response::created(json!({"id": new_id, ...item}))

// PUT /items/{id} - Update item
Response::ok(updated_item)

// DELETE /items/{id} - Delete item
Response::no_content()
```

### Error Handling with Result

```rust
fn handle(req: Request) -> Result<Response, HandlerError> {
    let user = find_user(&req)?;  // Returns 404 if not found
    Ok(Response::ok(user))
}

handler_loop_result!(handle);  // Errors auto-convert to Response
```

### File Uploads Response

```rust
fn handle_upload(req: Request) -> Result<Response, HandlerError> {
    let multipart = req.multipart()?;
    let file = multipart.require_file("document")?;

    // Process file...
    let saved_id = save_to_storage(&file.data)?;

    Ok(Response::created(json!({
        "id": saved_id,
        "filename": file.filename,
        "size": file.data.len(),
        "content_type": file.content_type
    })))
}
```

### Serving Files

```rust
fn serve_image(req: Request) -> Result<Response, HandlerError> {
    let id = req.require_path_param::<String>("id")?;
    let image_data = load_image(&id)?;

    Ok(Response::binary(200, image_data, "image/png")
        .with_cache(86400)  // Cache for 1 day
        .with_header("Content-Disposition", format!("inline; filename=\"{}.png\"", id)))
}
```

### Custom Content Types

```rust
// XML
Response::new(200)
    .with_header("Content-Type", "application/xml")
    .with_body("<root><item>Value</item></root>")

// CSV with download
Response::new(200)
    .with_header("Content-Type", "text/csv")
    .with_header("Content-Disposition", "attachment; filename=\"data.csv\"")
    .with_body("id,name\n1,Alice\n2,Bob")
```

### CORS Preflight

```rust
fn handle(req: Request) -> Response {
    if req.is_method("OPTIONS") {
        return Response::no_content()
            .with_cors("*")
            .with_header("Access-Control-Max-Age", "86400");
    }

    // Handle actual request...
    Response::ok(data).with_cors("*")
}

