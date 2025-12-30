use rust_edge_gateway_sdk::prelude::*;

async fn handle(req: Request) -> Result<Response, HandlerError> {
    Ok(crate::handler::handle(req).await)
}

#[cfg_attr(debug_assertions, tokio::main)]
fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    
    loop {
        match read_request() {
            Ok(req) => {
                let response = match rt.block_on(handle(req)) {
                    Ok(resp) => resp,
                    Err(e) => e.to_response(),
                };
                if let Err(e) = send_response(response) {
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

