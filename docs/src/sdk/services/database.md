# Database Service

Connect to SQL databases (PostgreSQL, MySQL, SQLite) from your handlers.

## Configuration

### PostgreSQL

```json
{
  "service_type": "postgres",
  "config": {
    "host": "localhost",
    "port": 5432,
    "database": "myapp",
    "username": "app_user",
    "password": "secret",
    "ssl_mode": "prefer",
    "pool_size": 10
  }
}
```

### MySQL

```json
{
  "service_type": "mysql",
  "config": {
    "host": "localhost",
    "port": 3306,
    "database": "myapp",
    "username": "app_user",
    "password": "secret",
    "use_ssl": false,
    "pool_size": 10
  }
}
```

### SQLite

```json
{
  "service_type": "sqlite",
  "config": {
    "path": "/data/app.db",
    "create_if_missing": true
  }
}
```

## Usage

### Basic Query

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    let db = DbPool { pool_id: "main".to_string() };
    
    let result = db.query("SELECT * FROM users WHERE active = ?", &["true"]);
    
    match result {
        Ok(data) => Response::ok(json!({"users": data.rows})),
        Err(e) => Response::internal_error(e.to_string()),
    }
}
```

### Query with Parameters

Always use parameterized queries to prevent SQL injection:

```rust
// GOOD - parameterized
db.query("SELECT * FROM users WHERE id = ?", &[&user_id])

// BAD - string concatenation (SQL injection risk!)
// db.query(&format!("SELECT * FROM users WHERE id = {}", user_id), &[])
```

### Insert, Update, Delete

Use `execute` for statements that modify data:

```rust
fn create_user(db: &DbPool, name: &str, email: &str) -> Result<u64, HandlerError> {
    db.execute(
        "INSERT INTO users (name, email) VALUES (?, ?)",
        &[name, email]
    )
}

fn update_user(db: &DbPool, id: &str, name: &str) -> Result<u64, HandlerError> {
    db.execute(
        "UPDATE users SET name = ? WHERE id = ?",
        &[name, id]
    )
}

fn delete_user(db: &DbPool, id: &str) -> Result<u64, HandlerError> {
    db.execute("DELETE FROM users WHERE id = ?", &[id])
}
```

### Working with Results

The `DbResult` contains rows as JSON objects:

```rust
let result = db.query("SELECT id, name, email FROM users", &[])?;

// result.rows is Vec<HashMap<String, Value>>
for row in &result.rows {
    let id = row.get("id");
    let name = row.get("name");
    println!("User: {:?} - {:?}", id, name);
}

// Or serialize the whole result
Response::ok(json!({
    "users": result.rows,
    "count": result.rows.len(),
}))
```

### Typed Results

Parse rows into your own structs:

```rust
#[derive(Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
}

fn get_user(db: &DbPool, id: &str) -> Result<Option<User>, HandlerError> {
    let result = db.query("SELECT * FROM users WHERE id = ?", &[id])?;
    
    if let Some(row) = result.rows.first() {
        let user: User = serde_json::from_value(row.clone().into())
            .map_err(|e| HandlerError::Internal(e.to_string()))?;
        Ok(Some(user))
    } else {
        Ok(None)
    }
}
```

## Error Handling

```rust
fn handle(req: Request) -> Response {
    let db = DbPool { pool_id: "main".to_string() };
    
    let result = db.query("SELECT * FROM users", &[]);
    
    match result {
        Ok(data) => Response::ok(json!({"users": data.rows})),
        Err(HandlerError::DatabaseError(msg)) => {
            eprintln!("Database error: {}", msg);
            Response::internal_error("Database temporarily unavailable")
        }
        Err(HandlerError::ServiceUnavailable(msg)) => {
            Response::json(503, json!({"error": "Service unavailable", "retry_after": 5}))
        }
        Err(e) => e.to_response(),
    }
}
```

## Best Practices

1. **Always use parameterized queries** - Never concatenate user input into SQL
2. **Handle connection errors gracefully** - Services may be temporarily unavailable
3. **Use appropriate pool sizes** - Match your concurrency needs
4. **Keep queries simple** - Complex logic is better in your handler code
5. **Log errors** - Use `eprintln!` for debugging database issues

