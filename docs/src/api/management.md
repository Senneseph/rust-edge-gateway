# Management API

The Rust Edge Gateway provides a REST API for managing endpoints, domains, collections, and services.

## Base URL

```
http://localhost:9081/api
```

In production, access via your admin domain:
```
https://rust-edge-gateway.yourdomain.com/api
```

## Response Format

All API responses follow this format:

```json
{
  "ok": true,
  "data": { ... }
}
```

Or for errors:

```json
{
  "ok": false,
  "error": "Error message"
}
```

## Authentication

Currently, the API is open. Future versions will support authentication.

## Rate Limiting

No rate limiting is currently applied to the management API.

## Endpoints Overview

| Resource | Endpoints |
|----------|-----------|
| [Domains](./domains.md) | `/api/domains/*` |
| [Collections](./collections.md) | `/api/collections/*` |
| [Services](./services.md) | `/api/services/*` |
| [Endpoints](./endpoints.md) | `/api/endpoints/*` |
| Import | `/api/import/*` |
| System | `/api/health`, `/api/stats` |

## System Endpoints

### Health Check

Check if the gateway is running.

```bash
GET /api/health
```

**Response:**

```json
{
  "ok": true,
  "data": {
    "status": "healthy",
    "version": "0.1.0"
  }
}
```

### Statistics

Get gateway statistics.

```bash
GET /api/stats
```

**Response:**

```json
{
  "ok": true,
  "data": {
    "endpoints_total": 10,
    "endpoints_running": 8,
    "requests_handled": 1234,
    "uptime_seconds": 3600
  }
}
```

## Import Endpoints

### Import OpenAPI Spec

Create endpoints from an OpenAPI 3.x specification.

```bash
POST /api/import/openapi
Content-Type: application/json

{
  "spec": "openapi: 3.0.0\ninfo:\n  title: Pet Store\n...",
  "domain": "api.example.com",
  "domain_id": "uuid-of-domain",
  "create_collection": true
}
```

**Request Body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `spec` | string | Yes | OpenAPI YAML or JSON content |
| `domain` | string | Yes | Domain to associate endpoints with |
| `domain_id` | string | No* | Domain UUID (*required if create_collection is true) |
| `collection_id` | string | No | Existing collection to add endpoints to |
| `create_collection` | bool | No | Create new collection from spec info |

**Response:**

```json
{
  "ok": true,
  "data": {
    "collection": {
      "id": "uuid",
      "name": "Pet Store",
      "base_path": "/v1"
    },
    "endpoints_created": 5,
    "endpoints": [
      {"id": "uuid", "name": "getPets", "path": "/pets", "method": "GET"},
      {"id": "uuid", "name": "createPet", "path": "/pets", "method": "POST"}
    ]
  }
}
```

### Import Bundle (ZIP)

Upload a ZIP file containing an OpenAPI spec and handler code files.

```bash
POST /api/import/bundle?domain=api.example.com&create_collection=true&domain_id=uuid&compile=true&start=true
Content-Type: multipart/form-data

# Form field: bundle (or file, zip) = your-bundle.zip
```

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `domain` | string | Yes | Domain to associate endpoints with |
| `domain_id` | string | No* | Domain UUID (*required if create_collection is true) |
| `collection_id` | string | No | Existing collection to add endpoints to |
| `create_collection` | bool | No | Create new collection from spec info |
| `compile` | bool | No | Compile handlers after import |
| `start` | bool | No | Start handlers after compilation (requires compile=true) |

**Bundle Structure:**

```
bundle.zip
├── openapi.yaml          # OpenAPI spec (or openapi.json, api.yaml, spec.yaml)
├── bundle.yaml           # Optional manifest with dependencies
└── handlers/             # Handler files (can also be at root or in src/)
    ├── get_pets.rs       # Matches operationId "getPets" or "get_pets"
    ├── create_pet.rs     # Matches operationId "createPet" or "create_pet"
    └── get_pet_by_id.rs  # Matches operationId "getPetById" or "get_pet_by_id"
```

**Bundle Manifest (bundle.yaml):**

The optional `bundle.yaml` file can specify dependencies shared by all handlers:

```yaml
bundle:
  name: my-api
  version: 1.0.0

dependencies:
  regex: "1.10"
  chrono:
    version: "0.4"
    features:
      - serde
  uuid:
    version: "1.0"
    features:
      - v4
      - serde

routes:
  - method: GET
    path: /pets
    handler: get_pets
  - method: POST
    path: /pets
    handler: create_pet
```

Handler files are matched to OpenAPI operations by normalizing names:
- `getPet.rs` → matches operationId `getPet` or `get_pet`
- `list_all_pets.rs` → matches operationId `listAllPets` or `list_all_pets`

**Response:**

```json
{
  "ok": true,
  "data": {
    "collection": {"id": "uuid", "name": "Pet Store"},
    "endpoints_created": 5,
    "endpoints_updated": 0,
    "handlers_matched": 5,
    "compiled": 5,
    "started": 5,
    "endpoints": [...],
    "errors": []
  }
}
```

**Example with curl (API Key Authentication):**

```bash
curl -X POST "http://localhost:8081/api/import/bundle?domain=api.example.com&create_collection=true&domain_id=abc123&compile=true&start=true" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -F "bundle=@my-api.zip"
```

**Example with curl (Session Authentication - Admin UI):**

```bash
curl -X POST "http://localhost:8081/api/admin/import/bundle?domain=api.example.com&create_collection=true&domain_id=abc123&compile=true&start=true" \
  -H "Cookie: session_id=YOUR_SESSION_ID" \
  -F "bundle=@my-api.zip"
```

**Note:** Import endpoints are available at two paths:
- `/api/import/*` - Requires API key with `import:write`, `import:*`, or both `endpoints:write` and `services:write` permissions
- `/api/admin/import/*` - Requires session authentication (for Admin UI)

## Common Patterns

### List with Filters

Most list endpoints support query parameters:

```bash
GET /api/endpoints?domain=api.example.com&enabled=true
```

### Pagination

List endpoints will support pagination in future versions:

```bash
GET /api/endpoints?page=1&limit=20
```

### Error Handling

Always check the `ok` field in responses:

```javascript
const response = await fetch('/api/endpoints');
const data = await response.json();

if (data.ok) {
  console.log('Endpoints:', data.data);
} else {
  console.error('Error:', data.error);
}
```

