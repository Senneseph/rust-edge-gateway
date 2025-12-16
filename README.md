# Rust Edge Gateway

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.86+-orange.svg)](https://www.rust-lang.org)

**Rust Edge Gateway** is a high-performance API gateway that lets you write request handlers in Rust. Your handlers are compiled to native binaries and run as isolated worker processes.

## ✨ Features

- 🚀 **Native Performance** - Handlers compile to optimized native code
- 🔒 **Process Isolation** - Each handler runs in its own process
- 🔄 **Hot Reload** - Update handlers without restarting the gateway
- 🛠️ **Simple SDK** - Easy-to-use Request/Response API with async support
- 📦 **Service Integration** - Connect to databases, Redis, S3/MinIO, and more
- 📋 **OpenAPI Import** - Import existing API specs and generate handler stubs
- 🎯 **Multi-Domain** - Host multiple APIs on different domains
- 🗄️ **Long-Lived Services** - Container-based SQLite and other persistent services

## 📚 Documentation

Full documentation is available at **[docs.rust-edge-gateway.iffuso.com](https://docs.rust-edge-gateway.iffuso.com)**

**New:** See the [SQLite Setup Guide](./SQLITE_SETUP_GUIDE.md) for using the long-lived SQLite service with your handlers.

## 🚀 Quick Start

### Option 1: Docker (Recommended)

```bash
# Clone the repository
git clone https://github.com/Senneseph/Rust-Edge-Gateway.git
cd Rust-Edge-Gateway

# Start the gateway (includes live-sqlite container)
docker-compose up -d

# Access the Admin UI
open http://localhost:9081/admin/
```

### Option 2: Build from Source

```bash
# Prerequisites: Rust 1.86+
cargo build --release --bin rust-edge-gateway

# Run the gateway
./target/release/rust-edge-gateway
```

### Option 3: Docker Production Image

```bash
docker-compose -f docker-compose.prod.yml up -d
```

## ⚙️ Configuration

Configure via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_EDGE_GATEWAY_DATA_DIR` | `./data` | SQLite database location |
| `RUST_EDGE_GATEWAY_HANDLERS_DIR` | `./handlers` | Compiled handlers location |
| `RUST_EDGE_GATEWAY_STATIC_DIR` | `./static` | Admin UI static files |
| `RUST_EDGE_GATEWAY_GATEWAY_PORT` | `8080` | Gateway port (API traffic) |
| `RUST_EDGE_GATEWAY_ADMIN_PORT` | `8081` | Admin UI/API port |
| `RUST_EDGE_GATEWAY_ADMIN_API_KEY` | *(none)* | Optional API key for admin |
| `RUST_EDGE_GATEWAY_HANDLER_TIMEOUT_SECS` | `30` | Handler request timeout |
| `RUST_EDGE_GATEWAY_HANDLER_MAX_MEMORY_MB` | `64` | Handler memory limit |
| `RUST_LOG` | `info` | Log level |
| `SQLITE_SERVICE_HOST` | `live-sqlite` | SQLite service hostname |
| `SQLITE_SERVICE_PORT` | `8080` | SQLite service port (internal) |

## 🔌 Management API

The gateway exposes a REST API on the admin port (default: 8081):

```bash

# Health check
curl http://localhost:8081/api/health

# List endpoints
curl http://localhost:8081/api/endpoints

# Create a domain
curl -X POST http://localhost:8081/api/domains \
  -H "Content-Type: application/json" \
  -d '{"name": "api.example.com"}'

# Create an endpoint
curl -X POST http://localhost:8081/api/endpoints \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hello-world",
    "domain_id": "<domain-uuid>",
    "path": "/hello",
    "method": "GET"
  }'

# Import an OpenAPI spec
curl -X POST "http://localhost:8081/api/import/openapi?domain=api.example.com" \
  -H "Content-Type: application/x-yaml" \
  --data-binary @openapi.yaml

# Import a bundle (OpenAPI + handlers)
curl -X POST "http://localhost:8081/api/import/bundle?domain=api.example.com&compile=true" \
  -F "bundle=@my-api.zip"
```

**Full API Reference:** See the [OpenAPI spec](docs/src/api/openapi.yaml) or the [Management API docs](https://docs.rust-edge-gateway.iffuso.com/api/management.html).

## 🦀 Writing Handlers

Handlers are Rust functions that receive a `Request` and return a `Response`:

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    Response::ok(json!({
        "message": "Hello, World!",
        "path": req.path,
        "method": req.method
    }))
}

handler_loop!(handle);
```

### Handler Macros

| Macro | Signature | Use Case |
|-------|-----------|----------|
| `handler_loop!` | `fn(Request) -> Response` | Simple sync handlers |
| `handler_loop_result!` | `fn(Request) -> Result<Response, HandlerError>` | Sync with `?` operator |
| `handler_loop_async!` | `async fn(Request) -> Response` | Async handlers |
| `handler_loop_async_result!` | `async fn(Request) -> Result<Response, HandlerError>` | Async with `?` operator |

### Async Handler Example

```rust
use rust_edge_gateway_sdk::prelude::*;

async fn handle(req: Request) -> Result<Response, HandlerError> {
    let id: i64 = req.require_path_param("id")?;
    let data: CreateItem = req.json()?;
    
    // Async database call
    let result = db.insert(&data).await
        .map_err(|e| HandlerError::DatabaseError(e.to_string()))?;
    
    Ok(Response::created(json!({"id": result.id})))
}

handler_loop_async_result!(handle);
```

### Request API

```rust
// Path and query parameters
let id: i64 = req.require_path_param("id")?;
let page: i32 = req.query_param_as("page").unwrap_or(1);

// Headers
let auth = req.require_header("Authorization")?;
let content_type = req.content_type();

// Body parsing
let data: MyStruct = req.json()?;            // Parse JSON body
let bytes = req.body_bytes();                 // Raw bytes
let form = req.multipart()?;                  // Multipart form data
```

### Response API

```rust
Response::ok(json!({"status": "ok"}))         // 200 OK with JSON
Response::created(json!({"id": 123}))         // 201 Created
Response::no_content()                        // 204 No Content
Response::bad_request("Invalid input")        // 400 Bad Request
Response::unauthorized("Missing token")       // 401 Unauthorized
Response::not_found()                         // 404 Not Found
Response::internal_error("Something broke")   // 500 Internal Server Error

Response::binary(bytes, "image/png")          // Binary response
Response::html("<h1>Hello</h1>")              // HTML response
Response::redirect("/new-location")           // 302 Redirect
```

### Error Handling

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Result<Response, HandlerError> {
    // These return HandlerError on failure, auto-converted to HTTP responses
    let id: i64 = req.require_path_param("id")?;
    let data: MyInput = req.json()?;

    if data.value < 0 {
        return Err(HandlerError::ValidationError("Value must be positive".into()));
    }

    Ok(Response::ok(json!({"processed": true})))
}

handler_loop_result!(handle);
```

## 📦 Bundle Format

Deploy complete APIs as ZIP files:

```
my-api.zip
├── openapi.yaml          # OpenAPI specification
└── handlers/
    ├── get_users.rs      # Matches operationId "getUsers"
    ├── create_user.rs    # Matches operationId "createUser"
    └── get_user_by_id.rs # Matches operationId "getUserById"
```

```bash
curl -X POST "http://localhost:8081/api/import/bundle?domain=api.example.com&compile=true&start=true" \
  -F "bundle=@my-api.zip"
```

## 🏗️ Architecture

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Client    │────▶│  Edge Gateway    │────▶│  Your Handler   │
│  (HTTP)     │     │  (Axum Router)   │     │  (Native Binary)│
│             │◀────│                  │◀────│                 │
└─────────────┘     └──────────────────┘     └─────────────────┘
                            │
                    ┌───────┴───────┐
                    │   Services    │
                    │  (DB, Redis,  │
                    │   S3, etc.)   │
                    └───────────────┘
```

- **Gateway Process**: Axum-based HTTP server handling routing and load balancing
- **Worker Processes**: Compiled handler binaries, one per endpoint
- **IPC Protocol**: Length-prefixed JSON over stdin/stdout
- **Service Connectors**: Pooled connections to backend services

## 🧪 Development

```bash
# Run tests
cargo test

# Build documentation
cd docs && mdbook build

# Development mode with hot reload
cargo watch -x "run --bin rust-edge-gateway"
```

## 📁 Project Structure

```
rust-edge-gateway/
├── crates/
│   ├── rust-edge-gateway/      # Main gateway binary
│   └── rust-edge-gateway-sdk/  # Handler SDK
├── docs/                       # mdBook documentation
├── static/admin/               # Admin UI
├── examples/                   # Example APIs
├── deploy/                     # Deployment configs
└── terraform/                  # Infrastructure as code
```

## 🤝 Contributing

Contributions are welcome! Please read the documentation and open an issue or PR.

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

