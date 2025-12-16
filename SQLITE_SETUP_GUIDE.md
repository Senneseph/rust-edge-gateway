# SQLite REG Service Setup Guide

This guide explains how to set up and use the long-lived SQLite container service with your Rust Edge Gateway handlers.

## Overview

The Rust Edge Gateway now includes support for handlers to connect to a containerized SQLite database service. This allows:

- **Persistent Data**: Data stored in SQLite persists across handler invocations
- **Shared State**: Multiple handlers can access the same database
- **Network Access**: Handlers can query the database via HTTP API
- **Easy Scaling**: Database is a separate service that can be scaled independently

## Architecture

```
┌─────────────────────────────────────────────────────┐
│         Rust Edge Gateway Container                 │
│                                                     │
│  ┌──────────────────────────────────────────────┐  │
│  │  Handler Process (a-icon-sqlite-test)        │  │
│  │                                              │  │
│  │  - Receives HTTP requests                    │  │
│  │  - Makes HTTP calls to SQLite service        │  │
│  │  - Returns JSON responses                    │  │
│  └──────────────────────────────────────────────┘  │
│           │                                         │
│           │ HTTP (port 8282)                       │
│           ↓                                         │
└───────────┼──────────────────────────────────────────┘
            │
    ┌───────┴──────────┐
    │ Docker Network   │
    │                  │
    ↓                  │
┌─────────────────┐    │
│  live-sqlite    │    │
│  Container      │    │
│                 │    │
│ - HTTP Server   │◄───┘
│ - SQLite DB     │
│ - Port 8080     │
│   (exposed as   │
│    8282)        │
└─────────────────┘
```

## Prerequisites

- Docker and Docker Compose
- The `live-sqlite` container image (ghcr.io/getchomp/sqlite-http)
- Rust Edge Gateway (version 0.1.0+)

## Deployment Steps

### 1. Update Docker Compose

Both `docker-compose.yml` and `docker-compose.prod.yml` have been updated to include the `live-sqlite` service.

**Key Features:**
- Automatic health checks
- Persistent volume (`sqlite_data`) for database files
- Network isolation (services communicate via Docker network)
- Configurable via environment variables

### 2. Configure Environment Variables

The gateway passes environment variables to handlers that need to connect to SQLite:

```bash
# In .env or docker-compose.yml
SQLITE_SERVICE_HOST=live-sqlite    # Container name (resolved via Docker DNS)
SQLITE_SERVICE_PORT=8080           # Internal port (8282 for external)
```

**For DigitalOcean Deployment:**
```bash
SQLITE_SERVICE_HOST=live-sqlite
SQLITE_SERVICE_PORT=8080
```

### 3. Start the Services

```bash
# Development
docker-compose up -d

# Production
docker-compose -f docker-compose.prod.yml up -d
```

### 4. Verify the Setup

```bash
# Check if live-sqlite is running
docker ps | grep live-sqlite

# Check health
curl http://localhost:8282/health

# Or inside the gateway container:
docker exec rust-edge-gateway curl -s http://live-sqlite:8080/health
```

## Creating a Handler

### Basic Handler Structure

```rust
use rust_edge_gateway_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct QueryRequest {
    query: String,
}

async fn handle(req: Request) -> Response {
    match req.path.as_str() {
        "/health" => Response::ok(json!({"status": "ok"})),
        "/query" => {
            if req.method == "POST" {
                handle_query(req).await
            } else {
                Response::method_not_allowed()
            }
        }
        _ => Response::not_found()
    }
}

async fn handle_query(req: Request) -> Response {
    let query_req: QueryRequest = match req.json() {
        Ok(q) => q,
        Err(e) => return Response::bad_request(json!({"error": e.to_string()})),
    };

    match execute_sqlite_query(&query_req.query).await {
        Ok(data) => Response::ok(json!({"data": data})),
        Err(e) => Response::internal_error(json!({"error": e})),
    }
}

async fn execute_sqlite_query(sql: &str) -> Result<serde_json::Value, String> {
    let sqlite_host = std::env::var("SQLITE_SERVICE_HOST")
        .unwrap_or_else(|_| "localhost".to_string());
    let sqlite_port = std::env::var("SQLITE_SERVICE_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8282);

    let url = format!("http://{}:{}/query", sqlite_host, sqlite_port);
    let client = reqwest::Client::new();
    let body = json!({"sql": sql, "params": []});

    client.post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}
```

### Handler Dependencies

Your `Cargo.toml` needs these dependencies:

```toml
[dependencies]
rust-edge-gateway-sdk = { path = "../../crates/rust-edge-gateway-sdk", features = ["async"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### Handler Macros

```rust
// For async handlers with Result type
#[handler_loop_async_result!(handle)]
async fn handle(req: Request) -> Result<Response, HandlerError> {
    // Your handler code
}

// For regular async handlers
#[handler_loop_async!(handle)]
async fn handle(req: Request) -> Response {
    // Your handler code
}

// For sync handlers
#[handler_loop!(handle)]
fn handle(req: Request) -> Response {
    // Your handler code
}
```

## SQLite Service API

The `live-sqlite` container exposes an HTTP API at port 8080 (internally):

### Query Endpoint

```
POST /query
Content-Type: application/json

{
  "sql": "SELECT * FROM users WHERE id = ?1",
  "params": ["123"]
}
```

**Response:**
```json
[
  {
    "id": 123,
    "name": "John Doe",
    "email": "john@example.com"
  }
]
```

### Execute Endpoint

```
POST /execute
Content-Type: application/json

{
  "sql": "INSERT INTO users (name, email) VALUES (?1, ?2)",
  "params": ["Jane Doe", "jane@example.com"]
}
```

**Response:**
```json
{
  "rows_affected": 1
}
```

### Health Endpoint

```
GET /health
```

**Response:**
```
200 OK
```

## Testing the Setup

### 1. Register the Test Handler

Use the Admin UI or API:

```bash
curl -X POST http://localhost:8081/api/handlers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "a-icon-sqlite-test",
    "path": "/test-sqlite",
    "method": "GET"
  }'
```

### 2. Test Health Check

```bash
curl http://localhost:9080/test-sqlite/health
```

Expected response:
```json
{
  "status": "ok",
  "handler": "a-icon-sqlite-test",
  "version": "0.1.0"
}
```

### 3. Test SQLite Connection

```bash
curl http://localhost:9080/test-sqlite/test-connection
```

Expected response (if SQLite is running):
```json
{
  "success": true,
  "message": "SQLite service is healthy",
  "service": {
    "host": "live-sqlite",
    "port": 8080,
    "base_url": "http://live-sqlite:8080"
  }
}
```

### 4. Execute a Query

```bash
curl -X POST http://localhost:9080/test-sqlite/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "SELECT sqlite_version();"
  }'
```

Expected response:
```json
{
  "success": true,
  "message": "Query executed successfully",
  "data": [
    {
      "sqlite_version()": "3.x.x"
    }
  ]
}
```

### 5. Create Test Table

```bash
curl http://localhost:9080/test-sqlite/create-table
```

Expected response:
```json
{
  "success": true,
  "message": "Test table created successfully"
}
```

### 6. Insert Data

```bash
curl http://localhost:9080/test-sqlite/insert-data
```

Expected response:
```json
{
  "success": true,
  "message": "Data inserted successfully",
  "response": {
    "rows_affected": 1
  }
}
```

## Troubleshooting

### Connection Refused

**Symptom:** "Failed to connect to SQLite service"

**Solution:**
1. Verify `live-sqlite` container is running: `docker ps | grep live-sqlite`
2. Check health: `docker exec live-sqlite curl http://localhost:8080/health`
3. Verify network connectivity: `docker network ls` and `docker network inspect <network>`

### Database File Not Persisting

**Solution:**
1. Verify the volume exists: `docker volume ls | grep sqlite_data`
2. Check volume mount in container: `docker inspect live-sqlite | grep -A 10 Mounts`
3. Ensure data is written to `/data/app.db` inside the container

### Query Returns Empty Result

**Solutions:**
1. Verify the query syntax is correct SQLite syntax
2. Use parameterized queries with `?1`, `?2`, etc.
3. Check the database file exists: `docker exec live-sqlite ls -la /data/`

### Handlers Can't Find SQLite

**Solution:**
1. Verify environment variables are set: `docker exec rust-edge-gateway env | grep SQLITE`
2. Check handler logs: `docker exec rust-edge-gateway cat handler_logs.txt`
3. Use the test handler to debug: `curl http://localhost:9080/test-sqlite/test-connection`

## Production Considerations

### Security

1. **Network Isolation**: The SQLite service only listens on the internal Docker network
2. **Data Protection**: Use Docker volumes for persistent storage
3. **Backups**: Implement regular volume backups
4. **Access Control**: Implement authentication in your handlers if needed

### Performance

1. **Connection Pooling**: The HTTP API handles connection management
2. **Query Optimization**: Use indexes for frequently queried fields
3. **Caching**: Consider caching results in handlers for read-heavy workloads

### Monitoring

1. **Health Checks**: The container includes automatic health checks
2. **Logging**: Check container logs: `docker logs live-sqlite`
3. **Resource Usage**: Monitor CPU and memory: `docker stats live-sqlite`

## Example: a-icon.com Integration

Here's how to integrate SQLite with your a-icon.com Edgelets:

```rust
// In your handler for a-icon.com
async fn handle(req: Request) -> Response {
    match req.path.as_str() {
        "/api/icons" => list_icons().await,
        "/api/icons/:id" => get_icon(req).await,
        "/api/icons" if req.method == "POST" => create_icon(req).await,
        _ => Response::not_found(),
    }
}

async fn list_icons() -> Response {
    let sql = "SELECT id, name, data FROM icons ORDER BY created_at DESC LIMIT 100";
    
    match execute_query(sql).await {
        Ok(rows) => Response::ok(json!({"icons": rows})),
        Err(e) => Response::internal_error(json!({"error": e})),
    }
}

async fn create_icon(req: Request) -> Response {
    #[derive(Deserialize)]
    struct CreateIcon { name: String, data: String }
    
    let icon: CreateIcon = match req.json() {
        Ok(i) => i,
        Err(e) => return Response::bad_request(json!({"error": e.to_string()})),
    };

    let sql = "INSERT INTO icons (name, data, created_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)";
    
    match execute_query_with_params(sql, &[&icon.name, &icon.data]).await {
        Ok(_) => Response::created(json!({"success": true})),
        Err(e) => Response::internal_error(json!({"error": e})),
    }
}
```

## References

- [SQLite HTTP Server Documentation](https://github.com/getchomp/sqlite-http)
- [Rust Edge Gateway SDK](../../README.md)
- [Handler Development Guide](../../docs/src/sdk/README.md)
