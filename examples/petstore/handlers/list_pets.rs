//! List Pets Handler
//!
//! Works with any storage backend: SQLite, PostgreSQL, MySQL, MinIO, or FTP

use rust_edge_gateway_sdk::prelude::*;

/// Storage configuration - set via environment or compile-time config
fn get_storage() -> Storage {
    // Default to database storage - can be changed to:
    // - Storage::database("petstore-db", "pets")
    // - Storage::object_storage("petstore-minio", "pets")
    // - Storage::file_storage("petstore-ftp", "pets")
    Storage::database("petstore", "pets")
}

fn handle(req: Request) -> Response {
    let storage = get_storage();
    
    // Get optional status filter from query params
    let status_filter = req.query_param("status").map(|s| s.as_str());
    
    match storage.list(status_filter) {
        Ok(pets) => Response::ok(json!({
            "pets": pets,
            "count": pets.len(),
        })),
        Err(e) => e.to_response(),
    }
}

handler_loop!(handle);

