//! Get Pet by ID Handler
//!
//! Works with any storage backend: SQLite, PostgreSQL, MySQL, MinIO, or FTP

use rust_edge_gateway_sdk::prelude::*;

fn get_storage() -> Storage {
    Storage::database("petstore", "pets")
}

fn handle(req: Request) -> Response {
    let storage = get_storage();
    
    // Get pet ID from path parameter
    let pet_id = match req.path_param("petId") {
        Some(id) => id.clone(),
        None => return Response::bad_request("Missing petId parameter"),
    };
    
    match storage.get(&pet_id) {
        Ok(Some(pet)) => Response::ok(pet),
        Ok(None) => Response::not_found(),
        Err(e) => e.to_response(),
    }
}

handler_loop!(handle);

