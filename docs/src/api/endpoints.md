# Endpoints API

Endpoints are the core resource - each represents a route with handler code.

## List Endpoints

```bash
GET /api/endpoints
```

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `domain` | string | Filter by domain hostname |
| `collection_id` | string | Filter by collection UUID |
| `enabled` | bool | Filter by enabled status |

**Response:**

```json
{
  "ok": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "getPets",
      "path": "/pets",
      "method": "GET",
      "domain": "api.example.com",
      "collection_id": "collection-uuid",
      "description": "List all pets",
      "code": "use rust_edge_gateway_sdk::prelude::*;\n...",
      "enabled": true,
      "status": "running",
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    }
  ]
}
```

## Create Endpoint

```bash
POST /api/endpoints
Content-Type: application/json

{
  "name": "getPets",
  "path": "/pets",
  "method": "GET",
  "domain": "api.example.com",
  "collection_id": "collection-uuid",
  "description": "List all pets",
  "code": "use rust_edge_gateway_sdk::prelude::*;\n\nfn handle(req: Request) -> Response {\n    Response::ok(json!({\"pets\": []}))\n}\n\nhandler_loop!(handle);",
  "enabled": true
}
```

**Request Body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Endpoint name (for display) |
| `path` | string | Yes | URL path pattern (e.g., `/pets/{id}`) |
| `method` | string | Yes | HTTP method (GET, POST, PUT, DELETE, PATCH) |
| `domain` | string | Yes | Domain hostname or `*` for all |
| `collection_id` | string | No | Parent collection UUID |
| `description` | string | No | Description of the endpoint |
| `code` | string | No | Rust handler code |
| `enabled` | bool | No | Whether endpoint is active (default: true) |

**Response:**

```json
{
  "ok": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "getPets",
    "path": "/pets",
    "method": "GET",
    "domain": "api.example.com",
    "collection_id": "collection-uuid",
    "description": "List all pets",
    "code": "...",
    "enabled": true,
    "status": "created",
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
}
```

## Get Endpoint

```bash
GET /api/endpoints/{id}
```

## Update Endpoint

```bash
PUT /api/endpoints/{id}
Content-Type: application/json

{
  "name": "Updated Name",
  "code": "// new code...",
  "enabled": false
}
```

## Delete Endpoint

```bash
DELETE /api/endpoints/{id}
```

## Compile Endpoint

Compile the handler code into an executable.

```bash
POST /api/endpoints/{id}/compile
```

**Response (Success):**

```json
{
  "ok": true,
  "data": {
    "status": "compiled",
    "message": "Compilation successful"
  }
}
```

**Response (Failure):**

```json
{
  "ok": false,
  "error": "error[E0308]: mismatched types\n  --> src/main.rs:5:5\n..."
}
```

## Start Endpoint

Start the worker process for this endpoint.

```bash
POST /api/endpoints/{id}/start
```

**Response:**

```json
{
  "ok": true,
  "data": {
    "status": "running",
    "pid": 12345
  }
}
```

## Stop Endpoint

Stop the worker process.

```bash
POST /api/endpoints/{id}/stop
```

**Response:**

```json
{
  "ok": true,
  "data": {
    "status": "stopped"
  }
}
```

## Endpoint Status Values

| Status | Description |
|--------|-------------|
| `created` | Endpoint defined, not yet compiled |
| `compiled` | Code compiled successfully |
| `running` | Worker process is active |
| `stopped` | Worker stopped, can be restarted |
| `error` | Compilation or runtime error |

## Bind Service to Endpoint

Associate a service with an endpoint.

```bash
POST /api/endpoints/{id}/services
Content-Type: application/json

{
  "service_id": "service-uuid",
  "pool_id": "main-db"
}
```

**Request Body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `service_id` | string | Yes | Service UUID to bind |
| `pool_id` | string | Yes | Identifier used in handler code |

## Unbind Service

```bash
DELETE /api/endpoints/{endpoint_id}/services/{service_id}
```

