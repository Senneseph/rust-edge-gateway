# Quick Start: SQLite with Rust Edge Gateway

## TL;DR - Getting Started in 5 Minutes

### Step 1: Start the Services

```bash
cd ~/Projects/rust-edge-gateway

# Development
docker-compose up -d

# Or Production
docker-compose -f docker-compose.prod.yml up -d
```

### Step 2: Test the Connection

```bash
# Verify SQLite service is running
curl http://localhost:8282/health

# From inside the gateway container
docker exec rust-edge-gateway curl http://live-sqlite:8080/health
```

### Step 3: Build and Deploy the Test Handler

The test handler is already created at `handlers/a-icon-sqlite-test/`

```bash
# Build the handler
cd handlers/a-icon-sqlite-test
cargo build --release

# The binary will be at: handlers/a-icon-sqlite-test/target/release/handler_a_icon_sqlite_test
```

### Step 4: Register the Handler via Admin UI

Open `http://localhost:9081/admin/` and:

1. Create a new Domain: `a-icon.local`
2. Create a Collection: `sqlite-tests`
3. Create an Endpoint:
   - Name: `sqlite-test`
   - Path: `/sqlite-test`
   - Method: `GET`
   - Compiled: Yes (after building)

### Step 5: Test the Handler

```bash
# Check handler health
curl http://localhost:9080/sqlite-test/health

# Test SQLite connection
curl http://localhost:9080/sqlite-test/test-connection

# Create a test table
curl http://localhost:9080/sqlite-test/create-table

# Insert test data
curl http://localhost:9080/sqlite-test/insert-data

# Execute a custom query
curl -X POST http://localhost:9080/sqlite-test/query \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM test_table;"}'
```

## Handler Template

Copy this to create a new handler:

```rust
// src/handler.rs
use rust_edge_gateway_sdk::prelude::*;

pub async fn handle(req: Request) -> Response {
    // Get SQLite service location from environment
    let sqlite_host = std::env::var("SQLITE_SERVICE_HOST")
        .unwrap_or_else(|_| "localhost".to_string());
    let sqlite_port = std::env::var("SQLITE_SERVICE_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8282);

    let base_url = format!("http://{}:{}", sqlite_host, sqlite_port);

    // Execute a query
    let body = json!({
        "sql": "SELECT sqlite_version();",
        "params": []
    });

    match reqwest::Client::new()
        .post(format!("{}/query", base_url))
        .json(&body)
        .send()
        .await
    {
        Ok(response) => {
            match response.json().await {
                Ok(data) => Response::ok(json!({"data": data})),
                Err(e) => Response::internal_error(json!({"error": e.to_string()})),
            }
        }
        Err(e) => Response::internal_error(json!({"error": e.to_string()})),
    }
}
```

```rust
// src/main.rs
use rust_edge_gateway_sdk::prelude::*;

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    
    loop {
        match read_request() {
            Ok(req) => {
                let response = rt.block_on(crate::handler::handle(req));
                if let Err(e) = send_response(response) {
                    eprintln!("Failed to send response: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to read request: {}", e);
                break;
            }
        }
    }
}
```

## Cargo.toml

```toml
[package]
name = "handler_my_handler"
version = "0.1.0"
edition = "2021"

[dependencies]
rust-edge-gateway-sdk = { path = "../../crates/rust-edge-gateway-sdk", features = ["async"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

## SQLite API Endpoints

All endpoints are on the SQLite service container (default: `http://live-sqlite:8080` internally, `http://localhost:8282` externally)

### Query (SELECT)

```bash
POST /query
Content-Type: application/json

{
  "sql": "SELECT id, name FROM users WHERE id = ?1",
  "params": ["123"]
}
```

### Execute (INSERT, UPDATE, DELETE)

```bash
POST /execute
Content-Type: application/json

{
  "sql": "INSERT INTO users (name, email) VALUES (?1, ?2)",
  "params": ["John", "john@example.com"]
}
```

### Health Check

```bash
GET /health
```

## Environment Variables in Handlers

```rust
// SQLite service configuration
let host = std::env::var("SQLITE_SERVICE_HOST").unwrap_or("localhost".to_string());
let port: u16 = std::env::var("SQLITE_SERVICE_PORT")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(8282);

let base_url = format!("http://{}:{}", host, port);
```

## Common Queries

### Create Table

```sql
CREATE TABLE IF NOT EXISTS icons (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    data BLOB,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
)
```

### Insert Data

```sql
INSERT INTO icons (name, data) VALUES (?1, ?2)
```

### Select Data

```sql
SELECT id, name, created_at FROM icons ORDER BY created_at DESC LIMIT 100
```

### Update Data

```sql
UPDATE icons SET name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2
```

### Delete Data

```sql
DELETE FROM icons WHERE id = ?1
```

## Debugging

### Check if SQLite container is running

```bash
docker ps | grep live-sqlite
```

### Check health

```bash
curl http://localhost:8282/health
docker logs live-sqlite
```

### Check database file

```bash
docker exec live-sqlite ls -la /data/
```

### Connect to database directly (inside container)

```bash
docker exec -it live-sqlite sqlite3 /data/app.db
```

### View handler logs

```bash
docker logs rust-edge-gateway
```

## Next Steps

1. ✅ Services running - `docker-compose up -d`
2. ✅ Test handler built and deployed
3. Create your own handlers following the template above
4. Register handlers in the Admin UI
5. Test via HTTP requests
6. Deploy to DigitalOcean when ready

## For a-icon.com Project

The handlers are located in:
- `handlers/a-icon-sqlite-test/` - Test handler

Create new handlers here and register them in the Admin UI. All handlers automatically get:
- `SQLITE_SERVICE_HOST=live-sqlite`
- `SQLITE_SERVICE_PORT=8080`

These environment variables are passed to all containers.

## Need Help?

- See full guide: `SQLITE_SETUP_GUIDE.md`
- Check test handler: `handlers/a-icon-sqlite-test/src/handler.rs`
- Rust Edge Gateway docs: `docs/`
