# Admin API Reference

The Rust Edge Gateway Admin API allows programmatic management of endpoints.

**Base URL:** `https://$env:TARGET_DOMAIN/api`

## Authentication

Include your API key in the header:

```
Authorization: Bearer YOUR_API_KEY
```

## Endpoints

### List Endpoints

```http
GET /endpoints
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "uuid",
      "name": "hello-world",
      "domain": "api.example.com",
      "path": "/hello",
      "method": "GET",
      "compiled": true,
      "enabled": true
    }
  ]
}
```

### Create Endpoint

```http
POST /endpoints
Content-Type: application/json

{
  "name": "hello-world",
  "domain": "api.example.com",
  "path": "/hello",
  "method": "GET",
  "code": "use edge_hive_sdk::prelude::*;\n\npub fn handle(req: Request) -> Response {\n    Response::ok(\"Hello!\")\n}"
}
```

### Get Endpoint

```http
GET /endpoints/{id}
```

### Update Endpoint

```http
PUT /endpoints/{id}
Content-Type: application/json

{
  "name": "updated-name",
  "domain": "new.domain.com",
  "enabled": true
}
```

### Delete Endpoint

```http
DELETE /endpoints/{id}
```

### Get Endpoint Code

```http
GET /endpoints/{id}/code
```

**Response:**
```json
{
  "success": true,
  "data": "use edge_hive_sdk::prelude::*;\n\npub fn handle..."
}
```

### Update Endpoint Code

```http
PUT /endpoints/{id}/code
Content-Type: application/json

{
  "code": "use edge_hive_sdk::prelude::*;\n\npub fn handle(req: Request) -> Response {\n    Response::ok(\"Updated!\")\n}"
}
```

### Compile Endpoint

```http
POST /endpoints/{id}/compile
```

Compiles the handler code into a binary. Returns compilation status.

**Response:**
```json
{
  "success": true,
  "data": "Compiled to /app/handlers/uuid/target/release/uuid"
}
```

### Start Endpoint

```http
POST /endpoints/{id}/start
```

Starts the worker process for this endpoint.

### Stop Endpoint

```http
POST /endpoints/{id}/stop
```

Stops the worker process.

## System

### Health Check

```http
GET /health
```

**Response:**
```json
{
  "success": true,
  "data": "healthy"
}
```

### Statistics

```http
GET /stats
```

**Response:**
```json
{
  "success": true,
  "data": {
    "endpoint_count": 5,
    "active_workers": 3
  }
}
```

