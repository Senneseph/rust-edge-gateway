# Collections API

Collections group related endpoints within a domain (e.g., "Pet Store", "User Management").

## List Collections

```bash
GET /api/collections
```

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `domain_id` | string | Filter by domain UUID |

**Response:**

```json
{
  "ok": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
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

## Create Collection

```bash
POST /api/collections
Content-Type: application/json

{
  "domain_id": "domain-uuid",
  "name": "Pet Store",
  "description": "Pet management endpoints",
  "base_path": "/pets",
  "enabled": true
}
```

**Request Body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `domain_id` | string | Yes | Parent domain UUID |
| `name` | string | Yes | Display name |
| `description` | string | No | Optional description |
| `base_path` | string | No | Common path prefix for endpoints |
| `enabled` | bool | No | Whether collection is active (default: true) |

**Response:**

```json
{
  "ok": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "domain_id": "domain-uuid",
    "name": "Pet Store",
    "description": "Pet management endpoints",
    "base_path": "/pets",
    "enabled": true,
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
}
```

## Get Collection

```bash
GET /api/collections/{id}
```

**Response:**

```json
{
  "ok": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "domain_id": "domain-uuid",
    "name": "Pet Store",
    "description": "Pet management endpoints",
    "base_path": "/pets",
    "enabled": true,
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
}
```

## Update Collection

```bash
PUT /api/collections/{id}
Content-Type: application/json

{
  "name": "Updated Name",
  "description": "Updated description",
  "base_path": "/v2/pets",
  "enabled": false
}
```

**Request Body:**

All fields are optional. Only provided fields will be updated.

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Display name |
| `description` | string | Description |
| `base_path` | string | Path prefix |
| `enabled` | bool | Active status |

**Response:**

```json
{
  "ok": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "domain_id": "domain-uuid",
    "name": "Updated Name",
    "description": "Updated description",
    "base_path": "/v2/pets",
    "enabled": false,
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T11:00:00Z"
  }
}
```

## Delete Collection

```bash
DELETE /api/collections/{id}
```

**Response:**

```json
{
  "ok": true,
  "data": null
}
```

**Note:** Deleting a collection will also delete all associated endpoints.

## Get Collection Endpoints

List all endpoints in a collection.

```bash
GET /api/collections/{id}/endpoints
```

**Response:**

```json
{
  "ok": true,
  "data": [
    {
      "id": "endpoint-uuid",
      "name": "getPets",
      "path": "/pets",
      "method": "GET",
      "domain": "api.example.com",
      "collection_id": "collection-uuid",
      "description": "List all pets",
      "enabled": true
    }
  ]
}
```

