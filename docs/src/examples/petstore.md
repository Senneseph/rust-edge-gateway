# Pet Store Demo

A complete REST API example demonstrating the same Pet Store API working with multiple storage backends.

## Overview

The Pet Store demo shows how Rust Edge Gateway's `Storage` abstraction allows the **same handler code** to work with:

- **SQLite** - Embedded database (no external dependencies)
- **PostgreSQL** - Full-featured relational database
- **MySQL** - Popular relational database
- **MinIO** - Object storage (pets stored as JSON files)
- **FTP/SFTP** - File transfer (pets stored as JSON files)

## Quick Start

```bash
cd examples/petstore

# SQLite (default - no external dependencies)
./setup.sh sqlite

# PostgreSQL
./setup.sh postgres

# MySQL
./setup.sh mysql

# MinIO (object storage)
./setup.sh minio

# FTP/SFTP (file storage)
./setup.sh ftp
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/pets` | List all pets (optional `?status=` filter) |
| POST | `/pets` | Create a new pet |
| GET | `/pets/{petId}` | Get a pet by ID |
| PUT | `/pets/{petId}` | Update a pet |
| DELETE | `/pets/{petId}` | Delete a pet |

## Storage Abstraction

The key to multi-backend support is the `Storage` type:

```rust
use rust_edge_gateway_sdk::prelude::*;

fn get_storage() -> Storage {
    // Choose one:
    Storage::database("petstore", "pets")           // SQLite/PostgreSQL/MySQL
    Storage::object_storage("petstore", "pets")     // MinIO/S3
    Storage::file_storage("petstore", "pets")       // FTP/SFTP
}

fn handle(req: Request) -> Response {
    let storage = get_storage();
    
    match storage.list(None) {
        Ok(pets) => Response::ok(json!({"pets": pets})),
        Err(e) => e.to_response(),
    }
}
```

## Storage API

All storage backends implement the same interface:

```rust
// Get a record by ID
storage.get("pet-123") -> Result<Option<JsonValue>, HandlerError>

// List all records (with optional filter)
storage.list(Some("available")) -> Result<Vec<JsonValue>, HandlerError>

// Create a new record
storage.create("pet-123", &pet_json) -> Result<(), HandlerError>

// Update an existing record
storage.update("pet-123", &pet_json) -> Result<(), HandlerError>

// Delete a record
storage.delete("pet-123") -> Result<bool, HandlerError>
```

## Database Schema

For SQL backends, use the provided schema:

```sql
CREATE TABLE IF NOT EXISTS pets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT,
    tags TEXT,  -- JSON array as string
    status TEXT NOT NULL DEFAULT 'available',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pets_status ON pets(status);
```

## File Storage Format

For MinIO and FTP backends, pets are stored as individual JSON files:

```
/pets/
  pet-001.json
  pet-002.json
  pet-003.json
```

Each file contains:

```json
{
  "id": "pet-001",
  "name": "Buddy",
  "category": "dog",
  "tags": ["friendly", "trained"],
  "status": "available",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

## Handler Code

See the handler implementations in `examples/petstore/handlers/`:

- `list_pets.rs` - List with optional status filter
- `get_pet.rs` - Get by ID
- `create_pet.rs` - Create with validation
- `update_pet.rs` - Partial update support
- `delete_pet.rs` - Delete by ID

## Testing

```bash
# Create a pet
curl -X POST http://petstore.example.com/pets \
  -H "Content-Type: application/json" \
  -d '{"name": "Buddy", "category": "dog", "status": "available"}'

# List all pets
curl http://petstore.example.com/pets

# Filter by status
curl http://petstore.example.com/pets?status=available

# Get a specific pet
curl http://petstore.example.com/pets/pet-001

# Update a pet
curl -X PUT http://petstore.example.com/pets/pet-001 \
  -H "Content-Type: application/json" \
  -d '{"status": "sold"}'

# Delete a pet
curl -X DELETE http://petstore.example.com/pets/pet-001
```

