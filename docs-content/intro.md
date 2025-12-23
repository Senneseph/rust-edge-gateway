# Rust Edge Gateway Platform

Welcome to Rust Edge Gateway - a Rust-powered serverless edge platform that provides high-performance API endpoints with minimal resource usage.

## What is Rust Edge Gateway?

Rust Edge Gateway is a lightweight alternative to AWS Lambda that:

- **Eliminates cold starts** - Workers stay warm and ready
- **Uses minimal memory** - Native Rust binaries with sub-megabyte footprint
- **Maintains persistent connections** - DB pools live across requests
- **Provides simple deployment** - Just write a handler function

## Quick Start

### 1. Access the Admin UI

Navigate to `https://$env:TARGET_DOMAIN/admin/` and log in with your API key.

### 2. Create an Endpoint

1. Click **+ New Endpoint**
2. Fill in:
   - **Name**: `hello-world`
   - **Domain**: `api.example.com`
   - **Path**: `/hello`
   - **Method**: `GET`

### 3. Write Your Handler

```rust
use edge_hive_sdk::prelude::*;

pub fn handle(req: Request) -> Response {
    Response::ok(json!({
        "message": "Hello, World!",
        "path": req.path
    }))
}
```

### 4. Compile and Start

Click **Compile**, then **Start**. Your endpoint is now live!

### 5. Test It

```bash
curl https://api.example.com/hello
```

## Next Steps

- [Handler Guide](./handlers/getting-started.md) - Learn to write handlers
- [Services](./services/overview.md) - Database, Redis, and more
- [API Reference](./api/endpoints.md) - Admin API documentation

