# Storage Abstraction

A unified interface for storing data across different backends.

## Overview

The `Storage` type provides a single API that works with:

- **Database backends** - SQLite, PostgreSQL, MySQL
- **Object storage** - MinIO, S3-compatible storage
- **File storage** - FTP, FTPS, SFTP

This allows you to write handler code once and deploy it with different backends.

## Creating a Storage Instance

```rust
use rust_edge_gateway_sdk::prelude::*;

// Database storage (SQLite, PostgreSQL, MySQL)
let db_storage = Storage::database("my-db-pool", "my_table");

// Object storage (MinIO, S3)
let obj_storage = Storage::object_storage("my-minio-pool", "data/items");

// File storage (FTP, SFTP)
let file_storage = Storage::file_storage("my-ftp-pool", "/data/items");
```

## Storage Operations

### Get by ID

```rust
let storage = Storage::database("pool", "users");

match storage.get("user-123") {
    Ok(Some(user)) => {
        // user is a JsonValue
        println!("Found: {}", user["name"]);
    }
    Ok(None) => {
        println!("Not found");
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

### List Records

```rust
// List all
let all_items = storage.list(None)?;

// List with filter (interpreted based on backend)
// For databases: WHERE status = ?
// For files: filter by filename pattern
let filtered = storage.list(Some("active"))?;
```

### Create Record

```rust
let item = json!({
    "name": "Widget",
    "price": 29.99,
    "in_stock": true
});

storage.create("item-001", &item)?;
```

### Update Record

```rust
let updated = json!({
    "name": "Widget Pro",
    "price": 39.99,
    "in_stock": true
});

storage.update("item-001", &updated)?;
```

### Delete Record

```rust
match storage.delete("item-001")? {
    true => println!("Deleted"),
    false => println!("Not found"),
}
```

## Backend Behavior

### Database (SQLite, PostgreSQL, MySQL)

- Records stored as table rows
- `table_name` specifies the table
- Filter parameter used in `WHERE status = ?`
- Requires table to exist with proper schema

### Object Storage (MinIO, S3)

- Records stored as JSON files
- Path: `{base_path}/{id}.json`
- Filter parameter passed to listing API
- Automatically creates bucket if needed

### File Storage (FTP, SFTP)

- Records stored as JSON files
- Path: `{base_path}/{id}.json`
- Filter parameter filters file listing
- Directory must exist on server

## Complete Example

```rust
use rust_edge_gateway_sdk::prelude::*;

fn get_storage() -> Storage {
    // Change this to switch backends:
    // Storage::database("pool", "items")
    // Storage::object_storage("minio", "items")
    Storage::file_storage("ftp", "/data/items")
}

fn handle(req: Request) -> Response {
    let storage = get_storage();
    
    match req.method.as_str() {
        "GET" => {
            if let Some(id) = req.path_param("id") {
                // Get single item
                match storage.get(id) {
                    Ok(Some(item)) => Response::ok(item),
                    Ok(None) => Response::not_found(),
                    Err(e) => e.to_response(),
                }
            } else {
                // List all items
                match storage.list(req.query_param("status").map(|s| s.as_str())) {
                    Ok(items) => Response::ok(json!({"items": items})),
                    Err(e) => e.to_response(),
                }
            }
        }
        "POST" => {
            let data: JsonValue = req.json().unwrap();
            let id = format!("item-{}", generate_id());
            match storage.create(&id, &data) {
                Ok(()) => Response::created(json!({"id": id})),
                Err(e) => e.to_response(),
            }
        }
        _ => Response::json(405, json!({"error": "Method not allowed"}))
    }
}

handler_loop!(handle);
```

## See Also

- [Pet Store Demo](../examples/petstore.md) - Complete example using Storage
- [Database Services](./database.md) - Direct database access
- [FTP Services](./ftp.md) - FTP/SFTP configuration

