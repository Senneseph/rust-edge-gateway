# Endpoint Handler Examples

This directory contains example Endpoint Handlers that demonstrate how to build
request handlers using the Rust Edge Gateway SDK.

## Architecture

Endpoint Handlers are user-written code that processes HTTP requests. They:

1. Receive a `Context` and `Request` from the gateway
2. Access Service Providers (database, cache, storage) via the Context
3. Return a `Response`

```text
HTTP Request → Gateway → Endpoint Handler → Response
                  ↓
            Context provides access to:
            - MinIO (object storage)
            - SQLite/PostgreSQL (databases)
            - Redis (cache)
            - etc.
```

## Dependency Injection Pattern

Similar to Angular's Service Provider pattern, handlers declare their
dependencies through the Context. The gateway injects the configured
Service Providers at runtime:

```rust
pub fn handle(ctx: &Context, req: Request) -> Response {
    // Access MinIO Service Provider via Context
    let minio = ctx.minio();
    
    // Use the service
    let data = minio.get_object("bucket", "key").await?;
    
    Response::ok(data)
}
```

## Available Examples

### minio-storage

Demonstrates file upload, download, list, and delete operations using
the MinIO Service Provider.

- `handle_list` - List objects in a bucket
- `handle_get` - Download an object
- `handle_put` - Upload an object  
- `handle_delete` - Delete an object

## Deploying Handlers

1. Write your handler using the SDK
2. Deploy via the Admin API or UI
3. The gateway compiles and loads your handler
4. Requests to your endpoint route to your handler

See the [SDK documentation](../../docs/src/sdk/) for more details.

