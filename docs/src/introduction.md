# Rust Edge Gateway

**Rust Edge Gateway** is a high-performance API gateway that lets you write request handlers in Rust. Your handlers are compiled to native binaries and run as isolated worker processes, providing:

- 🚀 **Native Performance** - Handlers compile to optimized native code
- 🔒 **Isolation** - Each handler runs in its own process
- 🔄 **Hot Reload** - Update handlers without restarting the gateway
- 🛠️ **Simple SDK** - Easy-to-use Request/Response API
- 📦 **Service Integration** - Connect to databases, Redis, and more

## How It Works

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Client    │────▶│  Edge Gateway    │────▶│  Your Handler   │
│  (Browser,  │     │  (Routes &       │     │  (Compiled      │
│   API, etc) │◀────│   Manages)       │◀────│   Rust Binary)  │
└─────────────┘     └──────────────────┘     └─────────────────┘
                            │
                            ▼
                    ┌───────────────┐
                    │   Services    │
                    │  (DB, Redis,  │
                    │   MinIO, etc) │
                    └───────────────┘
```

1. **Gateway receives request** - The gateway matches the incoming request to an endpoint
2. **Handler is invoked** - The compiled handler binary receives the request via IPC
3. **Handler processes** - Your code runs, optionally using injected services
4. **Response returned** - The handler sends the response back through the gateway

## Getting Started

The fastest way to get started is to:

1. Access the Admin UI at `/admin/`
2. Create a new endpoint
3. Write your handler code
4. Compile and test

See the [Quick Start](./getting-started/quick-start.md) guide for detailed instructions.

## SDK Overview

Your handler code uses the `rust-edge-gateway-sdk` crate:

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    Response::ok(json!({
        "message": "Hello, World!",
        "path": req.path,
        "method": req.method,
    }))
}

handler_loop!(handle);
```

The SDK provides:

- **[Request](./sdk/request.md)** - Access HTTP method, path, headers, body, query params
- **[Response](./sdk/response.md)** - Build HTTP responses with JSON, text, or custom content
- **[HandlerError](./sdk/errors.md)** - Structured error handling with HTTP status codes
- **[Services](./sdk/services.md)** - Database, Redis, and other service integrations

## Architecture

Rust Edge Gateway uses a worker process model:

- **Main Gateway** - Axum-based HTTP server handling routing
- **Worker Processes** - Your compiled handlers as standalone binaries
- **IPC Protocol** - Length-prefixed JSON over stdin/stdout
- **Service Connectors** - Pooled connections to backends (DB, Redis, etc.)

This architecture provides:

- **Security** - Handlers can't directly access the gateway's memory
- **Stability** - A crashed handler doesn't bring down the gateway
- **Scalability** - Multiple worker instances can handle concurrent requests

