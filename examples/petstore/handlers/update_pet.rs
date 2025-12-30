//! Update Pet Handler
//!
//! Works with any storage backend: SQLite, PostgreSQL, MySQL, MinIO, or FTP

use rust_edge_gateway_sdk::prelude::*;

fn get_storage() -> Storage {
    Storage::database("petstore", "pets")
}

#[derive(Deserialize)]
struct UpdatePetRequest {
    name: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    status: Option<String>,
}

fn handle(req: Request) -> Response {
    let storage = get_storage();
    
    // Get pet ID from path parameter
    let pet_id = match req.path_param("petId") {
        Some(id) => id.clone(),
        None => return Response::bad_request("Missing petId parameter"),
    };
    
    // Get existing pet
    let mut pet = match storage.get(&pet_id) {
        Ok(Some(p)) => p,
        Ok(None) => return Response::not_found(),
        Err(e) => return e.to_response(),
    };
    
    // Parse update request
    let input: UpdatePetRequest = match req.json() {
        Ok(data) => data,
        Err(e) => return Response::bad_request(format!("Invalid JSON: {}", e)),
    };
    
    // Apply updates
    if let Some(name) = input.name {
        pet["name"] = json!(name);
    }
    if let Some(category) = input.category {
        pet["category"] = json!(category);
    }
    if let Some(tags) = input.tags {
        pet["tags"] = json!(tags);
    }
    if let Some(status) = input.status {
        pet["status"] = json!(status);
    }
    
    // Update timestamp
    pet["updated_at"] = json!(chrono_now());
    
    match storage.update(&pet_id, &pet) {
        Ok(()) => Response::ok(pet),
        Err(e) => e.to_response(),
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}Z", secs)
}

handler_loop!(handle);

