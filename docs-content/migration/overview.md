# Migrating to Rust Edge Gateway

This guide helps you migrate existing API services to Rust Edge Gateway.

## Migration Steps

### 1. Identify Endpoints

List all endpoints in your existing service:

```
GET  /users
POST /users
GET  /users/{id}
PUT  /users/{id}
DELETE /users/{id}
```

### 2. Create Handler Files

For each endpoint (or group of related endpoints), create a handler:

```rust
use edge_hive_sdk::prelude::*;

pub fn handle(req: Request) -> Response {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/users") => list_users(req),
        ("POST", "/users") => create_user(req),
        ("GET", path) if path.starts_with("/users/") => get_user(req),
        ("PUT", path) if path.starts_with("/users/") => update_user(req),
        ("DELETE", path) if path.starts_with("/users/") => delete_user(req),
        _ => Response::not_found(),
    }
}

fn list_users(req: Request) -> Response {
    // TODO: Implement
    Response::ok(json!({"users": []}))
}

fn create_user(req: Request) -> Response {
    // TODO: Implement
    Response::created(json!({"id": 1}))
}

fn get_user(req: Request) -> Response {
    let id = req.path.strip_prefix("/users/").unwrap_or("0");
    // TODO: Implement
    Response::ok(json!({"id": id}))
}

fn update_user(req: Request) -> Response {
    let id = req.path.strip_prefix("/users/").unwrap_or("0");
    // TODO: Implement
    Response::ok(json!({"id": id, "updated": true}))
}

fn delete_user(req: Request) -> Response {
    Response::no_content()
}
```

### 3. Configure Services

If your handler needs a database:

1. Open the endpoint in the admin UI
2. Add service configuration (coming soon)
3. Update handler to use injected services

### 4. Test Locally

Use curl to test your endpoint:

```bash
# Create endpoint via API
curl -X POST http://localhost:9081/api/endpoints \
  -H "Content-Type: application/json" \
  -d '{"name": "users-api", "domain": "localhost", "path": "/users", "method": "GET"}'

# Upload code
curl -X PUT http://localhost:9081/api/endpoints/{id}/code \
  -H "Content-Type: application/json" \
  -d '{"code": "..."}'

# Compile
curl -X POST http://localhost:9081/api/endpoints/{id}/compile

# Start
curl -X POST http://localhost:9081/api/endpoints/{id}/start

# Test
curl http://localhost:9080/users
```

### 5. Update DNS

Point your domain to the Rust Edge Gateway server:

```
api.example.com  A  $DEPLOY_SERVER_IP
```

### 6. Deploy

Your endpoint is now live on Rust Edge Gateway!

## Common Patterns

### Express.js to Rust Edge Gateway

**Before (Express):**
```javascript
app.get('/api/status', (req, res) => {
  res.json({ status: 'ok', time: new Date() });
});
```

**After (Rust Edge Gateway):**
```rust
use edge_hive_sdk::prelude::*;
use chrono::Utc;

pub fn handle(_req: Request) -> Response {
    Response::ok(json!({
        "status": "ok",
        "time": Utc::now().to_rfc3339()
    }))
}
```

### NestJS Controller to Rust Edge Gateway

**Before (NestJS):**
```typescript
@Controller('items')
export class ItemsController {
  @Get()
  findAll(): Item[] {
    return this.itemsService.findAll();
  }
}
```

**After (Rust Edge Gateway):**
```rust
use edge_hive_sdk::prelude::*;

pub fn handle(req: Request, db: DbPool) -> Response {
    let items = db.query("SELECT * FROM items", &[])?;
    Response::ok(json!({"items": items.rows}))
}
```

