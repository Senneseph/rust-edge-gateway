# Service Actors

Service Actors provide thread-safe access to backend services using the actor pattern.

## Actor Pattern

Each service runs as an independent actor:

```
┌──────────────────────────────────────────────────────────────┐
│                      Service Actor                            │
├──────────────────────────────────────────────────────────────┤
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐    │
│  │   Inbox     │────▶│   Actor     │────▶│  Backend    │    │
│  │  (Channel)  │     │   Loop      │     │  (Pool)     │    │
│  └─────────────┘     └─────────────┘     └─────────────┘    │
│         ▲                   │                                │
│         │                   ▼                                │
│  ┌─────────────┐     ┌─────────────┐                        │
│  │  Handlers   │◀────│  Response   │                        │
│  │  (Callers)  │     │  Channel    │                        │
│  └─────────────┘     └─────────────┘                        │
└──────────────────────────────────────────────────────────────┘
```

## How It Works

1. **Handler sends a command** to the actor's inbox channel
2. **Actor receives the command** in its event loop
3. **Actor executes the operation** using its connection pool
4. **Actor sends the result** back via a oneshot channel
5. **Handler receives the result** and continues

## Actor Types

### Database Actor

Manages SQL database connections:

```rust
pub enum DatabaseCommand {
    Query {
        sql: String,
        params: Vec<Value>,
        reply: oneshot::Sender<Result<Vec<Row>>>,
    },
    Execute {
        sql: String,
        params: Vec<Value>,
        reply: oneshot::Sender<Result<u64>>,
    },
}
```

### Cache Actor

Manages Redis/Memcached connections:

```rust
pub enum CacheCommand {
    Get {
        key: String,
        reply: oneshot::Sender<Result<Option<String>>>,
    },
    Set {
        key: String,
        value: String,
        ttl: Option<u64>,
        reply: oneshot::Sender<Result<()>>,
    },
    Delete {
        key: String,
        reply: oneshot::Sender<Result<bool>>,
    },
}
```

### MinIO/Storage Actor

Manages object storage (S3/MinIO). This is a fully implemented service actor:

```rust
pub enum MinioCommand {
    GetObject {
        key: String,
        reply: oneshot::Sender<Result<Vec<u8>, String>>,
    },
    PutObject {
        key: String,
        data: Vec<u8>,
        content_type: Option<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    DeleteObject {
        key: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    ListObjects {
        prefix: Option<String>,
        reply: oneshot::Sender<Result<Vec<ObjectInfo>, String>>,
    },
}
```

#### MinIO Actor Implementation

The actor runs as an async task with an S3 bucket connection:

```rust
pub struct MinioHandle {
    sender: mpsc::Sender<MinioCommand>,
    bucket_name: String,
}

impl MinioHandle {
    pub async fn spawn(config: &MinioConfig) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel(100);
        let bucket = create_s3_bucket(config)?;

        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    MinioCommand::GetObject { key, reply } => {
                        let result = bucket.get_object(&key).await;
                        let _ = reply.send(result.map(|r| r.to_vec()));
                    }
                    MinioCommand::PutObject { key, data, content_type, reply } => {
                        let ct = content_type.as_deref().unwrap_or("application/octet-stream");
                        let result = bucket.put_object_with_content_type(&key, &data, ct).await;
                        let _ = reply.send(result.map(|_| ()));
                    }
                    // ... other commands
                }
            }
        });

        Ok(MinioHandle { sender: tx, bucket_name: config.bucket.clone() })
    }
}
```

#### Using the MinIO Actor

Handlers communicate with the actor via async message passing:

```rust
// Get an object
let (tx, rx) = oneshot::channel();
minio_handle.sender.send(MinioCommand::GetObject {
    key: "uploads/file.txt".to_string(),
    reply: tx,
}).await?;
let data = rx.await??;

// List objects
let (tx, rx) = oneshot::channel();
minio_handle.sender.send(MinioCommand::ListObjects {
    prefix: Some("uploads/".to_string()),
    reply: tx,
}).await?;
let objects = rx.await??;
```

## Actor Handle

Handlers interact with actors through handles:

```rust
pub struct ActorHandle<C> {
    sender: mpsc::Sender<C>,
}

impl<C> ActorHandle<C> {
    pub async fn send(&self, command: C) -> Result<()> {
        self.sender.send(command).await?;
        Ok(())
    }
}
```

## Benefits

### Thread Safety

Actors own their resources exclusively:
- No shared mutable state
- No locks needed
- No data races possible

### Isolation

Actor failures are contained:
- A crashed actor doesn't crash handlers
- Actors can be restarted independently
- Errors are returned as `Result` values

### Backpressure

Channel buffers provide natural backpressure:
- If an actor is overloaded, senders wait
- Prevents resource exhaustion
- Configurable buffer sizes

### Connection Pooling

Actors manage connection pools:
- Connections are reused across requests
- Pool size is configurable
- Automatic reconnection on failure

## Configuration

Actors are configured via the Admin UI or API. First create the service configuration:

```json
{
  "name": "my-storage",
  "service_type": "minio",
  "config": {
    "endpoint": "minio:9000",
    "access_key": "minioadmin",
    "secret_key": "minioadmin",
    "bucket": "my-bucket",
    "use_ssl": false,
    "region": "us-east-1"
  }
}
```

Then activate the service actor:

```bash
POST /api/services/{id}/activate
```

## Lifecycle

1. **Service created** - Configuration stored in database
2. **Service activated** - Actor task spawns, connects to backend
3. **Requests arrive** - Handlers send commands to actor via channel
4. **Actor processes** - Executes operations, returns results via oneshot
5. **Service deactivated** - Actor completes in-flight ops, shuts down
6. **Gateway stops** - All actors gracefully shut down

Actors can be activated/deactivated at runtime without restarting the gateway.

## REST Endpoints for MinIO

Once a MinIO service is activated, built-in handlers expose REST endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/minio/objects` | GET | List objects (with optional `?prefix=`) |
| `/api/minio/objects` | POST | Upload file (multipart form) |
| `/api/minio/objects/{key}` | GET | Download file |
| `/api/minio/objects/{key}` | DELETE | Delete file |

These handlers communicate with the MinIO actor via message passing, ensuring thread-safe access to the S3 bucket.

