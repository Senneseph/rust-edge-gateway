//! Gateway router - routes HTTP requests to handler libraries (v2 architecture)
//!
//! Uses dynamic library loading with graceful draining for zero-downtime deployments.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, get},
    Router,
};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::AppState;

/// Create the gateway router that handles all incoming requests
///
/// The gateway does NOT require API key authentication - it just routes requests
/// to compiled handlers. Any authentication/authorization is the responsibility
/// of the individual handlers themselves.
pub fn create_gateway_router(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_check))
        .route("/{*path}", any(handle_gateway_request))
        .route("/", any(handle_gateway_request))
}

/// Health check endpoint for the gateway
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Handle an incoming gateway request using v2 handler registry
async fn handle_gateway_request(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let request_id = Uuid::new_v4().to_string();

    // Extract domain from Host header (strip port if present)
    let host = request.headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");
    let domain = host.split(':').next().unwrap_or(host);

    tracing::debug!(
        request_id = %request_id,
        domain = %domain,
        method = %method,
        path = %path,
        "Incoming request"
    );

    // Find the endpoint for this request (with path parameter extraction)
    let (endpoint, path_params) = match state.db.find_endpoint(domain, &path, &method) {
        Ok(Some((e, params))) => (e, params),
        Ok(None) => {
            tracing::debug!("No endpoint found for {} {} {}", domain, method, path);
            return (StatusCode::NOT_FOUND, "Not Found").into_response();
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error").into_response();
        }
    };

    // Check if endpoint is compiled
    if !endpoint.compiled {
        return (StatusCode::SERVICE_UNAVAILABLE, "Endpoint not compiled").into_response();
    }

    // Build the SDK request
    let query: std::collections::HashMap<String, String> = request.uri()
        .query()
        .map(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let headers: std::collections::HashMap<String, String> = request.headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Get body
    let body_bytes = match axum::body::to_bytes(request.into_body(), 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to read body: {}", e);
            return (StatusCode::BAD_REQUEST, "Failed to read body").into_response();
        }
    };

    let body = if body_bytes.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&body_bytes).to_string())
    };

    let sdk_request = rust_edge_gateway_sdk::Request {
        method: method.clone(),
        path: path.clone(),
        query,
        headers,
        body,
        params: path_params,
        client_ip: None, // TODO: extract from X-Forwarded-For
        request_id: request_id.clone(),
    };

    // Execute via v2 handler registry with timeout and graceful draining support
    let timeout = Duration::from_secs(state.config.handler_timeout_secs);
    let ctx = state.create_sdk_context().await;

    let response = state.handler_registry.execute_with_timeout(
        &endpoint.id,
        &ctx,
        sdk_request,
        timeout,
    ).await;

    match response {
        Ok(sdk_response) => {
            let mut builder = Response::builder()
                .status(StatusCode::from_u16(sdk_response.status).unwrap_or(StatusCode::OK));

            for (key, value) in sdk_response.headers {
                builder = builder.header(&key, &value);
            }

            match builder.body(Body::from(sdk_response.body.unwrap_or_default())) {
                Ok(response) => response,
                Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response").into_response(),
            }
        }
        Err(e) => {
            let error_msg = e.to_string();

            // Check if handler is draining (return 503 for graceful handling)
            if error_msg.contains("draining") {
                tracing::info!(request_id = %request_id, "Handler is draining, returning 503");
                return (StatusCode::SERVICE_UNAVAILABLE, "Handler updating, please retry").into_response();
            }

            tracing::error!(request_id = %request_id, "Handler error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Handler error: {}", e)).into_response()
        }
    }
}
