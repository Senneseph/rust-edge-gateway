//! Create Pet Handler
//!
//! Works with any storage backend: SQLite, PostgreSQL, MySQL, MinIO, or FTP

use rust_edge_gateway_sdk::prelude::*;

fn get_storage() -> Storage {
    Storage::database("petstore", "pets")
}

#[derive(Deserialize)]
struct CreatePetRequest {
    name: String,
    category: Option<String>,
    tags: Option<Vec<String>>,
    status: Option<String>,
}

fn handle(req: Request) -> Response {
    let storage = get_storage();
    
    // Parse request body
    let input: CreatePetRequest = match req.json() {
        Ok(data) => data,
        Err(e) => return Response::bad_request(format!("Invalid JSON: {}", e)),
    };
    
    // Generate a new ID
    let id = format!("pet-{}", uuid_v4());
    let now = chrono_now();
    
    // Build pet object
    let pet = json!({
        "id": id,
        "name": input.name,
        "category": input.category,
        "tags": input.tags.unwrap_or_default(),
        "status": input.status.unwrap_or_else(|| "available".to_string()),
        "created_at": now,
        "updated_at": now,
    });
    
    match storage.create(&id, &pet) {
        Ok(()) => Response::created(pet),
        Err(e) => e.to_response(),
    }
}

/// Simple UUID v4 generator (simplified for demo)
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:032x}", nanos)
}

/// Get current ISO 8601 timestamp
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Simplified - in production use chrono crate
    format!("{}Z", secs)
}

handler_loop!(handle);

