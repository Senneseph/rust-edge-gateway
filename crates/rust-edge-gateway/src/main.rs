//! Rust Edge Gateway - Main entry point
//!
//! This is the main server that:
//! - Routes HTTP requests to worker processes
//! - Manages worker lifecycles
//! - Serves the admin UI
//! - Handles configuration and persistence

mod config;
mod db;
mod router;
mod worker;
mod api;
mod compiler;
mod openapi;
mod services;

use anyhow::Result;
use axum::{
    Router,
    routing::{get, post, put, delete},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::AppConfig;
use crate::db::Database;
use crate::worker::WorkerManager;

/// Shared application state
pub struct AppState {
    pub config: AppConfig,
    pub db: Database,
    pub workers: RwLock<WorkerManager>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,rust_edge_gateway=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Rust Edge Gateway");

    // Load configuration
    let config = AppConfig::from_env();
    tracing::info!("Configuration loaded: {:?}", config);

    // Initialize database
    let db = Database::new(&config.data_dir)?;
    db.migrate()?;
    tracing::info!("Database initialized");

    // Initialize worker manager
    let workers = WorkerManager::new(&config);

    // Create shared state
    let state = Arc::new(AppState {
        config: config.clone(),
        db,
        workers: RwLock::new(workers),
    });

    // Build admin API router
    let admin_api = Router::new()
        // Domains
        .route("/domains", get(api::list_domains))
        .route("/domains", post(api::create_domain))
        .route("/domains/{id}", get(api::get_domain))
        .route("/domains/{id}", put(api::update_domain))
        .route("/domains/{id}", delete(api::delete_domain))
        .route("/domains/{id}/collections", get(api::list_domain_collections))
        // Collections
        .route("/collections", get(api::list_collections))
        .route("/collections", post(api::create_collection))
        .route("/collections/{id}", get(api::get_collection))
        .route("/collections/{id}", put(api::update_collection))
        .route("/collections/{id}", delete(api::delete_collection))
        // Services
        .route("/services", get(api::list_services))
        .route("/services", post(api::create_service))
        .route("/services/{id}", get(api::get_service))
        .route("/services/{id}", put(api::update_service))
        .route("/services/{id}", delete(api::delete_service))
        .route("/services/{id}/test", post(api::test_service))
        // Endpoints
        .route("/endpoints", get(api::list_endpoints))
        .route("/endpoints", post(api::create_endpoint))
        .route("/endpoints/{id}", get(api::get_endpoint))
        .route("/endpoints/{id}", put(api::update_endpoint))
        .route("/endpoints/{id}", delete(api::delete_endpoint))
        .route("/endpoints/{id}/code", get(api::get_endpoint_code))
        .route("/endpoints/{id}/code", put(api::update_endpoint_code))
        .route("/endpoints/{id}/compile", post(api::compile_endpoint))
        .route("/endpoints/{id}/start", post(api::start_endpoint))
        .route("/endpoints/{id}/stop", post(api::stop_endpoint))
        // Import
        .route("/import/openapi", post(api::import_openapi))
        // System
        .route("/health", get(api::health_check))
        .route("/stats", get(api::get_stats));

    // Build admin router (serves static files + API)
    let admin_router = Router::new()
        .nest("/api", admin_api)
        .fallback_service(ServeDir::new(&config.static_dir));

    // Build main gateway router
    let gateway_router = router::create_gateway_router(state.clone());

    // Start admin server on port 8081
    let admin_addr = format!("0.0.0.0:{}", config.admin_port);
    let admin_listener = tokio::net::TcpListener::bind(&admin_addr).await?;
    tracing::info!("Admin UI listening on {}", admin_addr);

    let admin_state = state.clone();
    let admin_handle = tokio::spawn(async move {
        let app = admin_router
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http())
            .with_state(admin_state);
        
        axum::serve(admin_listener, app).await
    });

    // Start gateway server on port 8080
    let gateway_addr = format!("0.0.0.0:{}", config.gateway_port);
    let gateway_listener = tokio::net::TcpListener::bind(&gateway_addr).await?;
    tracing::info!("Gateway listening on {}", gateway_addr);

    let gateway_app = gateway_router
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let gateway_handle = tokio::spawn(async move {
        axum::serve(gateway_listener, gateway_app).await
    });

    // Wait for both servers
    tokio::select! {
        res = admin_handle => tracing::error!("Admin server exited: {:?}", res),
        res = gateway_handle => tracing::error!("Gateway server exited: {:?}", res),
    }

    Ok(())
}

