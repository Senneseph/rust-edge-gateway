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
    middleware::Next,
};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::AppState;

/// Create the gateway router that handles all incoming requests
pub fn create_gateway_router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_check))
        .route("/{*path}", any(handle_gateway_request))
        .route("/", any(handle_gateway_request))
        .layer(axum::middleware::from_fn_with_state(state.clone(), api_key_middleware))
}

/// Health check endpoint for the gateway
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// API key middleware for gateway requests
pub async fn api_key_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    // Skip API key check for health check endpoint only
    let path = request.uri().path();
    if path == "/health" {
        return Ok(next.run(request).await);
    }

    // Get headers from request
    let headers = request.headers();

    // Check for API key in headers
    let auth_header = headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Extract API key from Bearer auth
    if !auth_header.starts_with("Bearer ") {
        return Err((StatusCode::UNAUTHORIZED, "API key required".to_string()));
    }

    let api_key_str = &auth_header[7..];

    // Check rate limit for this API key
    if let Err(retry_after) = state.api_key_rate_limiter.check(api_key_str) {
        tracing::warn!(api_key = %api_key_str, "API key rate limited");
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            format!("Too many requests. Try again in {} seconds", retry_after.as_secs())
        ));
    }

    // Validate API key
    let admin_db = crate::db_admin::AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    let key = admin_db.get_api_key_by_value(api_key_str)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid API key".to_string()))?;

    // Check if API key is enabled
    if !key.enabled {
        return Err((StatusCode::UNAUTHORIZED, "API key is disabled".to_string()));
    }

    // Check if API key has expired
    if let Some(expires_at) = key.expires_at {
        if chrono::Utc::now() > expires_at {
            return Err((StatusCode::UNAUTHORIZED, "API key has expired".to_string()));
        }
    }

    // Continue to the next middleware/handler
    Ok(next.run(request).await)
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
