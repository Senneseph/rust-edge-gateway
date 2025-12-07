# Services API

Services represent backend connections (databases, caches, storage) that handlers can use.

## List Services

```bash
GET /api/services
```

**Response:**

```json
{
  "ok": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Main Database",
      "service_type": "postgres",
      "config": {
        "host": "db.example.com",
        "port": 5432,
        "database": "myapp"
      },
      "enabled": true,
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    }
  ]
}
```

## Create Service

```bash
POST /api/services
Content-Type: application/json

{
  "name": "Main Database",
  "service_type": "postgres",
  "config": {
    "host": "db.example.com",
    "port": 5432,
    "database": "myapp",
    "username": "app_user",
    "password": "secret"
  },
  "enabled": true
}
```

**Request Body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Display name |
| `service_type` | string | Yes | Type of service (see below) |
| `config` | object | Yes | Service-specific configuration |
| `enabled` | bool | No | Whether service is active (default: true) |

**Service Types:**

| Type | Description |
|------|-------------|
| `sqlite` | SQLite embedded database |
| `postgres` | PostgreSQL database |
| `mysql` | MySQL database |
| `redis` | Redis cache/store |
| `mongodb` | MongoDB document database |
| `minio` | MinIO/S3 object storage |
| `memcached` | Memcached cache |

**Response:**

```json
{
  "ok": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Main Database",
    "service_type": "postgres",
    "config": { ... },
    "enabled": true,
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
}
```

## Get Service

```bash
GET /api/services/{id}
```

## Update Service

```bash
PUT /api/services/{id}
Content-Type: application/json

{
  "name": "Updated Name",
  "config": {
    "host": "new-db.example.com"
  },
  "enabled": false
}
```

## Delete Service

```bash
DELETE /api/services/{id}
```

## Test Service Connection

Test if the service is reachable and properly configured.

```bash
POST /api/services/{id}/test
```

**Response (Success):**

```json
{
  "ok": true,
  "data": {
    "connected": true,
    "latency_ms": 5,
    "message": "Connection successful"
  }
}
```

**Response (Failure):**

```json
{
  "ok": true,
  "data": {
    "connected": false,
    "error": "Connection refused"
  }
}
```

## Service Configuration Examples

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

### Redis

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

### MinIO

```json
{
  "service_type": "minio",
  "config": {
    "endpoint": "minio.example.com:9000",
    "access_key": "minioadmin",
    "secret_key": "minioadmin",
    "use_ssl": true,
    "bucket": "uploads"
  }
}
```

