//! Auto-generated handler wrapper
use edge_hive_sdk::prelude::*;

mod handler;

fn main() {
    loop {
        match edge_hive_sdk::ipc::read_request() {
            Ok(req) => {
                let response = handler::handle(req);
                if let Err(e) = edge_hive_sdk::ipc::send_response(response) {
                    eprintln!("Failed to send response: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to read request: {}", e);
                break;
            }
        }
    }
}
