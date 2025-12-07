//! Admin API endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

// ============================================================================
// Domain - Top-level organization (e.g., "api.example.com")
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub id: String,
    pub name: String,
    pub host: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDomainRequest {
    pub name: String,
    pub host: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDomainRequest {
    pub name: Option<String>,
    pub host: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

// ============================================================================
// Collection - Group endpoints within a domain (e.g., "Pet Store", "Users")
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub domain_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub base_path: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCollectionRequest {
    pub domain_id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub base_path: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCollectionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub base_path: Option<String>,
    pub enabled: Option<bool>,
}

// ============================================================================
// Service - Backend service connections (databases, caches, storage)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub service_type: ServiceType,
    pub config: serde_json::Value,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Sqlite,
    Minio,
    Mysql,
    Postgres,
    Redis,
    Memcached,
    Mongodb,
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceType::Sqlite => write!(f, "sqlite"),
            ServiceType::Minio => write!(f, "minio"),
            ServiceType::Mysql => write!(f, "mysql"),
            ServiceType::Postgres => write!(f, "postgres"),
            ServiceType::Redis => write!(f, "redis"),
            ServiceType::Memcached => write!(f, "memcached"),
            ServiceType::Mongodb => write!(f, "mongodb"),
        }
    }
}

impl std::str::FromStr for ServiceType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sqlite" => Ok(ServiceType::Sqlite),
            "minio" => Ok(ServiceType::Minio),
            "mysql" => Ok(ServiceType::Mysql),
            "postgres" => Ok(ServiceType::Postgres),
            "redis" => Ok(ServiceType::Redis),
            "memcached" => Ok(ServiceType::Memcached),
            "mongodb" => Ok(ServiceType::Mongodb),
            _ => Err(format!("Unknown service type: {}", s)),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceRequest {
    pub name: String,
    pub service_type: ServiceType,
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServiceRequest {
    pub name: Option<String>,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

// ============================================================================
// Endpoint - API endpoints within collections
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<String>,
    pub name: String,
    pub domain: String,
    pub path: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub compiled: bool,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// Request to create a new endpoint
#[derive(Debug, Deserialize)]
pub struct CreateEndpointRequest {
    pub collection_id: Option<String>,
    pub name: String,
    pub domain: String,
    pub path: String,
    #[serde(default = "default_method")]
    pub method: String,
    pub description: Option<String>,
    pub code: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

/// Request to update an endpoint
#[derive(Debug, Deserialize)]
pub struct UpdateEndpointRequest {
    pub collection_id: Option<String>,
    pub name: Option<String>,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub method: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

/// Code update request
#[derive(Debug, Deserialize)]
pub struct UpdateCodeRequest {
    pub code: String,
}

/// API response wrapper
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self { success: false, data: None, error: Some(error.into()) }
    }
}

/// Health check endpoint
pub async fn health_check() -> Json<ApiResponse<&'static str>> {
    Json(ApiResponse::ok("healthy"))
}

/// Get system stats
#[derive(Serialize)]
pub struct Stats {
    pub endpoint_count: i64,
    pub active_workers: usize,
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Stats>> {
    let endpoint_count = state.db.endpoint_count().unwrap_or(0);
    let workers = state.workers.read().await;
    let active_workers = workers.active_count();

    Json(ApiResponse::ok(Stats { endpoint_count, active_workers }))
}

/// List all endpoints
pub async fn list_endpoints(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<Endpoint>>>, StatusCode> {
    match state.db.list_endpoints() {
        Ok(endpoints) => Ok(Json(ApiResponse::ok(endpoints))),
        Err(e) => {
            tracing::error!("Failed to list endpoints: {}", e);
            Ok(Json(ApiResponse::err(e.to_string())))
        }
    }
}

/// Create a new endpoint
pub async fn create_endpoint(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateEndpointRequest>,
) -> Result<Json<ApiResponse<Endpoint>>, StatusCode> {
    let endpoint = Endpoint {
        id: Uuid::new_v4().to_string(),
        collection_id: req.collection_id,
        name: req.name,
        domain: req.domain,
        path: req.path,
        method: req.method.to_uppercase(),
        description: req.description,
        code: req.code,
        compiled: false,
        enabled: false,
        created_at: None,
        updated_at: None,
    };

    match state.db.create_endpoint(&endpoint) {
        Ok(_) => Ok(Json(ApiResponse::ok(endpoint))),
        Err(e) => {
            tracing::error!("Failed to create endpoint: {}", e);
            Ok(Json(ApiResponse::err(e.to_string())))
        }
    }
}

/// Get an endpoint by ID
pub async fn get_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Endpoint>>, StatusCode> {
    match state.db.get_endpoint(&id) {
        Ok(Some(endpoint)) => Ok(Json(ApiResponse::ok(endpoint))),
        Ok(None) => Ok(Json(ApiResponse::err("Endpoint not found"))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Update an endpoint
pub async fn update_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateEndpointRequest>,
) -> Result<Json<ApiResponse<Endpoint>>, StatusCode> {
    let existing = match state.db.get_endpoint(&id) {
        Ok(Some(e)) => e,
        Ok(None) => return Ok(Json(ApiResponse::err("Endpoint not found"))),
        Err(e) => return Ok(Json(ApiResponse::err(e.to_string()))),
    };

    let updated = Endpoint {
        id: existing.id,
        collection_id: req.collection_id.or(existing.collection_id),
        name: req.name.unwrap_or(existing.name),
        domain: req.domain.unwrap_or(existing.domain),
        path: req.path.unwrap_or(existing.path),
        method: req.method.map(|m| m.to_uppercase()).unwrap_or(existing.method),
        description: req.description.or(existing.description),
        code: existing.code,
        compiled: existing.compiled,
        enabled: req.enabled.unwrap_or(existing.enabled),
        created_at: existing.created_at,
        updated_at: existing.updated_at,
    };

    match state.db.update_endpoint(&updated) {
        Ok(_) => Ok(Json(ApiResponse::ok(updated))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Delete an endpoint
pub async fn delete_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    // Stop the worker if running
    {
        let mut workers = state.workers.write().await;
        workers.stop_worker(&id);
    }

    match state.db.delete_endpoint(&id) {
        Ok(_) => Ok(Json(ApiResponse::ok(()))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Get endpoint code
pub async fn get_endpoint_code(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.db.get_endpoint(&id) {
        Ok(Some(endpoint)) => Ok(Json(ApiResponse::ok(endpoint.code.unwrap_or_default()))),
        Ok(None) => Ok(Json(ApiResponse::err("Endpoint not found"))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Update endpoint code
pub async fn update_endpoint_code(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCodeRequest>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.db.update_endpoint_code(&id, &req.code) {
        Ok(_) => Ok(Json(ApiResponse::ok(()))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Compile an endpoint
pub async fn compile_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let endpoint = match state.db.get_endpoint(&id) {
        Ok(Some(e)) => e,
        Ok(None) => return Ok(Json(ApiResponse::err("Endpoint not found"))),
        Err(e) => return Ok(Json(ApiResponse::err(e.to_string()))),
    };

    let code = match endpoint.code {
        Some(c) => c,
        None => return Ok(Json(ApiResponse::err("No code to compile"))),
    };

    // Compile the handler
    match crate::compiler::compile_handler(&state.config, &id, &code).await {
        Ok(binary_path) => {
            state.db.mark_compiled(&id, true).ok();
            Ok(Json(ApiResponse::ok(format!("Compiled to {}", binary_path))))
        }
        Err(e) => Ok(Json(ApiResponse::err(format!("Compilation failed: {}", e)))),
    }
}

/// Start an endpoint (spawn worker)
pub async fn start_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let endpoint = match state.db.get_endpoint(&id) {
        Ok(Some(e)) if e.compiled => e,
        Ok(Some(_)) => return Ok(Json(ApiResponse::err("Endpoint not compiled"))),
        Ok(None) => return Ok(Json(ApiResponse::err("Endpoint not found"))),
        Err(e) => return Ok(Json(ApiResponse::err(e.to_string()))),
    };

    let mut workers = state.workers.write().await;
    match workers.start_worker(&endpoint) {
        Ok(_) => {
            state.db.update_endpoint(&Endpoint { enabled: true, ..endpoint }).ok();
            Ok(Json(ApiResponse::ok(())))
        }
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Stop an endpoint (kill worker)
pub async fn stop_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let mut workers = state.workers.write().await;
    workers.stop_worker(&id);
    Ok(Json(ApiResponse::ok(())))
}

// ============================================================================
// Domain API Handlers
// ============================================================================

/// List all domains
pub async fn list_domains(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<Domain>>>, StatusCode> {
    match state.db.list_domains() {
        Ok(domains) => Ok(Json(ApiResponse::ok(domains))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Create a new domain
pub async fn create_domain(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDomainRequest>,
) -> Result<Json<ApiResponse<Domain>>, StatusCode> {
    let domain = Domain {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        host: req.host,
        description: req.description,
        enabled: true,
        created_at: None,
        updated_at: None,
    };

    match state.db.create_domain(&domain) {
        Ok(_) => Ok(Json(ApiResponse::ok(domain))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Get a domain by ID
pub async fn get_domain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Domain>>, StatusCode> {
    match state.db.get_domain(&id) {
        Ok(Some(domain)) => Ok(Json(ApiResponse::ok(domain))),
        Ok(None) => Ok(Json(ApiResponse::err("Domain not found"))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Update a domain
pub async fn update_domain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateDomainRequest>,
) -> Result<Json<ApiResponse<Domain>>, StatusCode> {
    let existing = match state.db.get_domain(&id) {
        Ok(Some(d)) => d,
        Ok(None) => return Ok(Json(ApiResponse::err("Domain not found"))),
        Err(e) => return Ok(Json(ApiResponse::err(e.to_string()))),
    };

    let updated = Domain {
        id: existing.id,
        name: req.name.unwrap_or(existing.name),
        host: req.host.unwrap_or(existing.host),
        description: req.description.or(existing.description),
        enabled: req.enabled.unwrap_or(existing.enabled),
        created_at: existing.created_at,
        updated_at: existing.updated_at,
    };

    match state.db.update_domain(&updated) {
        Ok(_) => Ok(Json(ApiResponse::ok(updated))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Delete a domain
pub async fn delete_domain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.db.delete_domain(&id) {
        Ok(_) => Ok(Json(ApiResponse::ok(()))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

// ============================================================================
// Collection API Handlers
// ============================================================================

/// List collections (optionally by domain)
pub async fn list_collections(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<Collection>>>, StatusCode> {
    match state.db.list_collections(None) {
        Ok(collections) => Ok(Json(ApiResponse::ok(collections))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// List collections for a domain
pub async fn list_domain_collections(
    State(state): State<Arc<AppState>>,
    Path(domain_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Collection>>>, StatusCode> {
    match state.db.list_collections(Some(&domain_id)) {
        Ok(collections) => Ok(Json(ApiResponse::ok(collections))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Create a new collection
pub async fn create_collection(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<ApiResponse<Collection>>, StatusCode> {
    let collection = Collection {
        id: Uuid::new_v4().to_string(),
        domain_id: req.domain_id,
        name: req.name,
        description: req.description,
        base_path: req.base_path,
        enabled: true,
        created_at: None,
        updated_at: None,
    };

    match state.db.create_collection(&collection) {
        Ok(_) => Ok(Json(ApiResponse::ok(collection))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Get a collection by ID
pub async fn get_collection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Collection>>, StatusCode> {
    match state.db.get_collection(&id) {
        Ok(Some(collection)) => Ok(Json(ApiResponse::ok(collection))),
        Ok(None) => Ok(Json(ApiResponse::err("Collection not found"))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Update a collection
pub async fn update_collection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCollectionRequest>,
) -> Result<Json<ApiResponse<Collection>>, StatusCode> {
    let existing = match state.db.get_collection(&id) {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(Json(ApiResponse::err("Collection not found"))),
        Err(e) => return Ok(Json(ApiResponse::err(e.to_string()))),
    };

    let updated = Collection {
        id: existing.id,
        domain_id: existing.domain_id,
        name: req.name.unwrap_or(existing.name),
        description: req.description.or(existing.description),
        base_path: req.base_path.unwrap_or(existing.base_path),
        enabled: req.enabled.unwrap_or(existing.enabled),
        created_at: existing.created_at,
        updated_at: existing.updated_at,
    };

    match state.db.update_collection(&updated) {
        Ok(_) => Ok(Json(ApiResponse::ok(updated))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Delete a collection
pub async fn delete_collection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.db.delete_collection(&id) {
        Ok(_) => Ok(Json(ApiResponse::ok(()))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}



// ============================================================================
// Service API Handlers
// ============================================================================

/// List all services
pub async fn list_services(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<Service>>>, StatusCode> {
    match state.db.list_services() {
        Ok(services) => Ok(Json(ApiResponse::ok(services))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Create a new service
pub async fn create_service(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateServiceRequest>,
) -> Result<Json<ApiResponse<Service>>, StatusCode> {
    let service = Service {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        service_type: req.service_type,
        config: req.config,
        enabled: true,
        created_at: None,
        updated_at: None,
    };

    match state.db.create_service(&service) {
        Ok(_) => Ok(Json(ApiResponse::ok(service))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Get a service by ID
pub async fn get_service(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Service>>, StatusCode> {
    match state.db.get_service(&id) {
        Ok(Some(service)) => Ok(Json(ApiResponse::ok(service))),
        Ok(None) => Ok(Json(ApiResponse::err("Service not found"))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Update a service
pub async fn update_service(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateServiceRequest>,
) -> Result<Json<ApiResponse<Service>>, StatusCode> {
    let existing = match state.db.get_service(&id) {
        Ok(Some(s)) => s,
        Ok(None) => return Ok(Json(ApiResponse::err("Service not found"))),
        Err(e) => return Ok(Json(ApiResponse::err(e.to_string()))),
    };

    let updated = Service {
        id: existing.id,
        name: req.name.unwrap_or(existing.name),
        service_type: existing.service_type,
        config: req.config.unwrap_or(existing.config),
        enabled: req.enabled.unwrap_or(existing.enabled),
        created_at: existing.created_at,
        updated_at: existing.updated_at,
    };

    match state.db.update_service(&updated) {
        Ok(_) => Ok(Json(ApiResponse::ok(updated))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}

/// Delete a service
pub async fn delete_service(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.db.delete_service(&id) {
        Ok(_) => Ok(Json(ApiResponse::ok(()))),
        Err(e) => Ok(Json(ApiResponse::err(e.to_string()))),
    }
}