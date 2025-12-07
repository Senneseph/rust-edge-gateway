# Services

Rust Edge Gateway can connect your handlers to backend services like databases, Redis, and object storage.

## Overview

Services are configured in the gateway admin UI and made available to handlers. Your handler code uses typed service handles to interact with backends.

## Available Service Types

| Service | Description | Use Cases |
|---------|-------------|-----------|
| **SQLite** | Embedded SQL database | Local data, caching, simple apps |
| **PostgreSQL** | Advanced relational database | Complex queries, transactions |
| **MySQL** | Popular relational database | Web applications, compatibility |
| **Redis** | In-memory data store | Caching, sessions, pub/sub |
| **MongoDB** | Document database | Flexible schemas, JSON data |
| **MinIO** | S3-compatible object storage | File uploads, media storage |
| **Memcached** | Distributed caching | High-speed key-value caching |

## Configuring Services

### Via Admin UI

1. Go to **Services** in the admin panel
2. Click **Create Service**
3. Select service type and configure connection
4. Test the connection
5. Bind to endpoints

### Via API

```bash
curl -X POST http://localhost:9081/api/services \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Main Database",
    "service_type": "postgres",
    "config": {
      "host": "db.example.com",
      "port": 5432,
      "database": "myapp",
      "username": "app_user",
      "password": "secret"
    }
  }'
```

## Using Services in Handlers

Services are accessed through typed handles in your handler code.

### Database Example

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    let db = DbPool { pool_id: "main-db".to_string() };
    
    // Query
    let result = db.query(
        "SELECT id, name FROM users WHERE active = ?",
        &["true"]
    );
    
    match result {
        Ok(data) => Response::ok(json!({"users": data.rows})),
        Err(e) => Response::internal_error(e.to_string()),
    }
}
```

### Redis Example

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    let redis = RedisPool { pool_id: "cache".to_string() };
    
    // Try cache first
    if let Ok(Some(cached)) = redis.get("user:123") {
        return Response::ok(json!({"source": "cache", "data": cached}));
    }
    
    // Cache miss - fetch and store
    let data = fetch_from_db();
    let _ = redis.setex("user:123", &data, 300); // Cache for 5 minutes
    
    Response::ok(json!({"source": "db", "data": data}))
}
```

## Service Handles

### DbPool

For SQL databases (PostgreSQL, MySQL, SQLite):

```rust
pub struct DbPool {
    pub pool_id: String,
}

impl DbPool {
    /// Execute a query, returns rows
    fn query(&self, sql: &str, params: &[&str]) -> Result<DbResult, HandlerError>;
    
    /// Execute a statement (INSERT, UPDATE, DELETE)
    fn execute(&self, sql: &str, params: &[&str]) -> Result<u64, HandlerError>;
}
```

### RedisPool

For Redis:

```rust
pub struct RedisPool {
    pub pool_id: String,
}

impl RedisPool {
    /// Get a value
    fn get(&self, key: &str) -> Result<Option<String>, HandlerError>;
    
    /// Set a value
    fn set(&self, key: &str, value: &str) -> Result<(), HandlerError>;
    
    /// Set with expiration (seconds)
    fn setex(&self, key: &str, value: &str, seconds: u64) -> Result<(), HandlerError>;
}
```

## Binding Services to Endpoints

Services must be bound to endpoints before they can be used:

1. Create the service in the admin UI
2. Open the endpoint configuration
3. Add the service binding with a pool ID
4. The pool ID is used in your handler code

This allows the same endpoint code to use different service instances in different environments (dev, staging, prod).

## Next Steps

- [Database Service Details](./services/database.md)
- [Redis Service Details](./services/redis.md)

