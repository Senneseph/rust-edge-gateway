# JSON API

Build a RESTful JSON API endpoint.

## Code

```rust
use rust_edge_gateway_sdk::prelude::*;

#[derive(Deserialize)]
struct CreateItem {
    name: String,
    description: Option<String>,
    price: f64,
}

#[derive(Serialize)]
struct Item {
    id: String,
    name: String,
    description: Option<String>,
    price: f64,
    created_at: String,
}

fn handle(req: Request) -> Response {
    // Parse request body
    let input: CreateItem = match req.json() {
        Ok(data) => data,
        Err(e) => return Response::bad_request(format!("Invalid JSON: {}", e)),
    };
    
    // Validate
    if input.name.is_empty() {
        return Response::bad_request("Name is required");
    }
    
    if input.price < 0.0 {
        return Response::bad_request("Price must be non-negative");
    }
    
    // Create item (in real app, save to database)
    let item = Item {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        description: input.description,
        price: input.price,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    
    // Return 201 Created
    Response::created(item)
}

handler_loop!(handle);
```

## Endpoint Configuration

| Setting | Value |
|---------|-------|
| Path | `/items` |
| Method | `POST` |
| Domain | `*` |

## Test

```bash
curl -X POST http://localhost:9080/items \
  -H "Content-Type: application/json" \
  -d '{"name": "Widget", "description": "A useful widget", "price": 19.99}'
```

## Response

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Widget",
  "description": "A useful widget",
  "price": 19.99,
  "created_at": "2024-01-15T10:30:00.000Z"
}
```

## Full CRUD Example

For a complete CRUD API, create multiple endpoints:

### GET /items - List Items

```rust
fn handle(_req: Request) -> Response {
    // In real app, fetch from database
    let items = vec![
        json!({"id": "1", "name": "Item 1"}),
        json!({"id": "2", "name": "Item 2"}),
    ];
    
    Response::ok(json!({
        "items": items,
        "count": items.len(),
    }))
}
```

### GET /items/{id} - Get Item

```rust
fn handle(req: Request) -> Response {
    let id = match req.path_param("id") {
        Some(id) => id,
        None => return Response::bad_request("Missing ID"),
    };
    
    // In real app, fetch from database
    if id == "1" {
        Response::ok(json!({
            "id": "1",
            "name": "Item 1",
            "price": 9.99,
        }))
    } else {
        Response::not_found()
    }
}
```

### PUT /items/{id} - Update Item

```rust
#[derive(Deserialize)]
struct UpdateItem {
    name: Option<String>,
    price: Option<f64>,
}

fn handle(req: Request) -> Response {
    let id = match req.path_param("id") {
        Some(id) => id.clone(),
        None => return Response::bad_request("Missing ID"),
    };
    
    let update: UpdateItem = match req.json() {
        Ok(data) => data,
        Err(e) => return Response::bad_request(format!("Invalid JSON: {}", e)),
    };
    
    // In real app, update in database
    Response::ok(json!({
        "id": id,
        "name": update.name.unwrap_or("Unchanged".to_string()),
        "updated": true,
    }))
}
```

### DELETE /items/{id} - Delete Item

```rust
fn handle(req: Request) -> Response {
    let id = match req.path_param("id") {
        Some(id) => id,
        None => return Response::bad_request("Missing ID"),
    };
    
    // In real app, delete from database
    eprintln!("Deleted item: {}", id);
    
    Response::no_content()
}
```

