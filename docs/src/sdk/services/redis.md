# Redis Service

Use Redis for caching, sessions, and fast key-value storage.

## Configuration

```json
{
  "service_type": "redis",
  "config": {
    "host": "localhost",
    "port": 6379,
    "password": null,
    "database": 0,
    "use_tls": false,
    "pool_size": 10
  }
}
```

### Configuration Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | string | required | Redis server hostname |
| `port` | u16 | 6379 | Redis server port |
| `password` | string | null | Redis password (optional) |
| `database` | u8 | 0 | Redis database number (0-15) |
| `use_tls` | bool | false | Enable TLS encryption |
| `pool_size` | u32 | 10 | Connection pool size |
| `username` | string | null | Username for Redis 6+ ACL |

## Usage

### Basic Operations

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    let redis = RedisPool { pool_id: "cache".to_string() };
    
    // Get a value
    match redis.get("my-key") {
        Ok(Some(value)) => Response::ok(json!({"value": value})),
        Ok(None) => Response::not_found(),
        Err(e) => Response::internal_error(e.to_string()),
    }
}
```

### Set Values

```rust
// Set without expiration
redis.set("key", "value")?;

// Set with expiration (seconds)
redis.setex("session:abc123", &session_data, 3600)?; // 1 hour
redis.setex("rate:user:123", "1", 60)?; // 1 minute
```

### Caching Pattern

```rust
fn handle(req: Request) -> Response {
    let redis = RedisPool { pool_id: "cache".to_string() };
    let db = DbPool { pool_id: "main".to_string() };
    
    let user_id = req.path_param("id").unwrap();
    let cache_key = format!("user:{}", user_id);
    
    // Try cache first
    if let Ok(Some(cached)) = redis.get(&cache_key) {
        return Response::ok(json!({
            "source": "cache",
            "user": serde_json::from_str::<JsonValue>(&cached).unwrap()
        }));
    }
    
    // Cache miss - fetch from database
    let result = db.query("SELECT * FROM users WHERE id = ?", &[user_id])?;
    
    if let Some(user) = result.rows.first() {
        // Cache for 5 minutes
        let user_json = serde_json::to_string(user).unwrap();
        let _ = redis.setex(&cache_key, &user_json, 300);
        
        return Response::ok(json!({
            "source": "database",
            "user": user
        }));
    }
    
    Response::not_found()
}
```

### Session Management

```rust
fn get_session(redis: &RedisPool, session_id: &str) -> Result<Option<Session>, HandlerError> {
    let key = format!("session:{}", session_id);
    
    match redis.get(&key)? {
        Some(data) => {
            let session: Session = serde_json::from_str(&data)
                .map_err(|e| HandlerError::Internal(e.to_string()))?;
            Ok(Some(session))
        }
        None => Ok(None),
    }
}

fn save_session(redis: &RedisPool, session_id: &str, session: &Session) -> Result<(), HandlerError> {
    let key = format!("session:{}", session_id);
    let data = serde_json::to_string(session)
        .map_err(|e| HandlerError::Internal(e.to_string()))?;
    
    // Sessions expire in 24 hours
    redis.setex(&key, &data, 86400)
}

#[derive(Serialize, Deserialize)]
struct Session {
    user_id: String,
    created_at: String,
    data: JsonValue,
}
```

### Rate Limiting

```rust
fn check_rate_limit(redis: &RedisPool, client_ip: &str) -> Result<bool, HandlerError> {
    let key = format!("rate:{}", client_ip);
    
    match redis.get(&key)? {
        Some(count) => {
            let count: u32 = count.parse().unwrap_or(0);
            if count >= 100 {
                return Ok(false); // Rate limited
            }
            // Note: This is a simplified example
            // Real implementation would use INCR command
            redis.setex(&key, &(count + 1).to_string(), 60)?;
            Ok(true)
        }
        None => {
            redis.setex(&key, "1", 60)?;
            Ok(true)
        }
    }
}

fn handle(req: Request) -> Response {
    let redis = RedisPool { pool_id: "cache".to_string() };
    
    let client_ip = req.client_ip.as_deref().unwrap_or("unknown");
    
    match check_rate_limit(&redis, client_ip) {
        Ok(true) => {
            // Process request normally
            Response::ok(json!({"status": "ok"}))
        }
        Ok(false) => {
            Response::json(429, json!({"error": "Too many requests"}))
                .with_header("Retry-After", "60")
        }
        Err(e) => {
            // If Redis is down, allow the request (fail open)
            eprintln!("Rate limit check failed: {}", e);
            Response::ok(json!({"status": "ok"}))
        }
    }
}
```

## Error Handling

```rust
match redis.get("key") {
    Ok(Some(value)) => { /* use value */ }
    Ok(None) => { /* key doesn't exist */ }
    Err(HandlerError::RedisError(msg)) => {
        eprintln!("Redis error: {}", msg);
        // Fallback behavior
    }
    Err(HandlerError::ServiceUnavailable(_)) => {
        // Redis is down - decide on fallback
    }
    Err(e) => { /* other error */ }
}
```

## Best Practices

1. **Use meaningful key prefixes** - `user:123`, `session:abc`, `cache:posts:1`
2. **Always set expiration for cache keys** - Prevents unbounded memory growth
3. **Handle Redis unavailability** - Decide on fail-open vs fail-closed
4. **Don't store large values** - Redis works best with small, fast lookups
5. **Use JSON for structured data** - Easy to serialize/deserialize

