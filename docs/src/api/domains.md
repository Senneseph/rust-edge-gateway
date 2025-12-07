# Domains API

Domains represent the top-level organization for your endpoints (e.g., `api.example.com`).

## List Domains

```bash
GET /api/domains
```

**Response:**

```json
{
  "ok": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Production API",
      "host": "api.example.com",
      "description": "Main production API",
      "enabled": true,
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    }
  ]
}
```

## Create Domain

```bash
POST /api/domains
Content-Type: application/json

{
  "name": "Production API",
  "host": "api.example.com",
  "description": "Main production API",
  "enabled": true
}
```

**Request Body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Display name for the domain |
| `host` | string | Yes | Hostname (e.g., `api.example.com`) |
| `description` | string | No | Optional description |
| `enabled` | bool | No | Whether domain is active (default: true) |

**Response:**

```json
{
  "ok": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Production API",
    "host": "api.example.com",
    "description": "Main production API",
    "enabled": true,
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
}
```

## Get Domain

```bash
GET /api/domains/{id}
```

**Response:**

```json
{
  "ok": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Production API",
    "host": "api.example.com",
    "description": "Main production API",
    "enabled": true,
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
}
```

## Update Domain

```bash
PUT /api/domains/{id}
Content-Type: application/json

{
  "name": "Updated API Name",
  "description": "Updated description",
  "enabled": false
}
```

**Request Body:**

All fields are optional. Only provided fields will be updated.

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Display name |
| `host` | string | Hostname |
| `description` | string | Description |
| `enabled` | bool | Active status |

**Response:**

```json
{
  "ok": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Updated API Name",
    "host": "api.example.com",
    "description": "Updated description",
    "enabled": false,
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T11:00:00Z"
  }
}
```

## Delete Domain

```bash
DELETE /api/domains/{id}
```

**Response:**

```json
{
  "ok": true,
  "data": null
}
```

**Note:** Deleting a domain will also delete all associated collections and endpoints.

## Get Domain Collections

List all collections belonging to a domain.

```bash
GET /api/domains/{id}/collections
```

**Response:**

```json
{
  "ok": true,
  "data": [
    {
      "id": "collection-uuid",
      "domain_id": "domain-uuid",
      "name": "Pet Store",
      "description": "Pet management endpoints",
      "base_path": "/pets",
      "enabled": true,
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    }
  ]
}
```

