//! Rust Edge Gateway - Main entry point
//!
//! This is the main server that:
//! - Routes HTTP requests to handler processes (v1) or dynamic libraries (v2)
//! - Manages worker/handler lifecycles
//! - Provides actor-based service runtime
//! - Serves the admin UI
//! - Handles configuration and persistence

mod config;
mod db;
mod db_admin; // Admin authentication database
mod router;
mod worker;
mod api;
mod compiler;
mod openapi;
mod bundle;
mod services;
mod runtime;  // New: Actor-based runtime
mod admin_auth; // New: Admin authentication
mod rate_limit; // Rate limiting for authentication
mod session; // Session management for admin UI

use anyhow::Result;
use axum::{
    Router,
    routing::{get, post, delete},
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
use crate::runtime::{
    Context,
    Services as RuntimeServices,
    HandlerRegistry,
    context::{RuntimeConfig, ContextBuilder},
};
use crate::admin_auth::{
    admin_login_page, create_admin_auth_router, endpoints_api_key_auth, services_api_key_auth,
    domains_api_key_auth, collections_api_key_auth, get_recaptcha_site_key,
};

/// Shared application state
pub struct AppState {
    pub config: AppConfig,
    pub db: Database,
    pub workers: RwLock<WorkerManager>,

    // New v2 runtime components
    pub runtime_services: RwLock<RuntimeServices>,
    pub handler_registry: HandlerRegistry,
    pub runtime_config: Arc<RuntimeConfig>,

    // Rate limiters for authentication
    pub login_rate_limiter: Arc<rate_limit::RateLimiter>,
    pub api_key_rate_limiter: Arc<rate_limit::RateLimiter>,

    // Session store for admin UI
    pub session_store: Arc<session::SessionStore>,
}

impl AppState {
    /// Create a Context for a request
    pub async fn create_context(&self) -> Context {
        let services = self.runtime_services.read().await.clone();
        ContextBuilder::new(services)
            .config(self.runtime_config.clone())
            .build()
    }

    /// Update runtime services (e.g., when activating a new service actor)
    pub async fn update_services<F>(&self, f: F)
    where
        F: FnOnce(&mut RuntimeServices),
    {
        let mut services = self.runtime_services.write().await;
        f(&mut services);
    }
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

    // Initialize admin database and create initial admin user if needed
    let admin_db = crate::db_admin::AdminDatabase::new(&config.data_dir)?;
    admin_db.migrate()?;
    
    // Create initial admin user if DEFAULT_ADMIN_PASSWORD is set and no admin exists
    if let Some(default_password) = &config.default_admin_password {
        if admin_db.get_admin_by_username("admin")?.is_none() {
            admin_db.create_initial_admin("admin", default_password)?;
            tracing::info!("Created initial admin user with password from DEFAULT_ADMIN_PASSWORD");
        } else {
            tracing::info!("Admin user already exists, skipping initial admin creation");
        }
    } else {
        tracing::warn!("DEFAULT_ADMIN_PASSWORD not set, skipping initial admin creation");
    }

    // Initialize worker manager (v1 - subprocess based)
    let workers = WorkerManager::new(&config);

    // Initialize v2 runtime components
    let runtime_services = RuntimeServices::new();
    let handler_registry = HandlerRegistry::new(config.handlers_dir.clone());
    let runtime_config = Arc::new(RuntimeConfig {
        handler_timeout_secs: config.handler_timeout_secs,
        max_body_size: 10 * 1024 * 1024, // 10MB
        debug: std::env::var("RUST_LOG").map(|v| v.contains("debug")).unwrap_or(false),
    });
    
    tracing::info!("Runtime initialized (v2 actor-based services ready)");

    // Initialize rate limiters
    // Login: 5 attempts per 15 minutes
    let login_rate_limiter = Arc::new(rate_limit::RateLimiter::new(
        5,
        std::time::Duration::from_secs(15 * 60),
    ));
    // API key validation: 100 attempts per minute (more lenient for legitimate API usage)
    let api_key_rate_limiter = Arc::new(rate_limit::RateLimiter::new(
        100,
        std::time::Duration::from_secs(60),
    ));

    // Session store: 24 hour session duration
    let session_store = Arc::new(session::SessionStore::new(
        std::time::Duration::from_secs(24 * 60 * 60),
    ));

    // Create shared state
    let state = Arc::new(AppState {
        config: config.clone(),
        db,
        workers: RwLock::new(workers),
        runtime_services: RwLock::new(runtime_services),
        handler_registry,
        runtime_config,
        login_rate_limiter,
        api_key_rate_limiter,
        session_store,
    });

    // ============================================================================
    // API Routes - Consolidated under /api
    // ============================================================================

    // Endpoints API - protected by API key with endpoints:* permissions
    let endpoints_api = Router::new()
        .route("/", get(api::list_endpoints).post(api::create_endpoint))
        .route("/{id}", get(api::get_endpoint).put(api::update_endpoint).delete(api::delete_endpoint))
        .route("/{id}/code", get(api::get_endpoint_code).put(api::update_endpoint_code))
        .route("/{id}/compile", post(api::compile_endpoint))
        .route("/{id}/start", post(api::start_endpoint))
        .route("/{id}/stop", post(api::stop_endpoint))
        .layer(axum::middleware::from_fn_with_state(state.clone(), endpoints_api_key_auth));

    // Services API - protected by API key with services:* permissions
    let services_api = Router::new()
        .route("/", get(api::list_services).post(api::create_service))
        .route("/{id}", get(api::get_service).put(api::update_service).delete(api::delete_service))
        .route("/{id}/test", post(api::test_service))
        .route("/{id}/activate", post(api::activate_service))
        .route("/{id}/deactivate", post(api::deactivate_service))
        .layer(axum::middleware::from_fn_with_state(state.clone(), services_api_key_auth));

    // Domains API - protected by API key with domains:* permissions
    let domains_api = Router::new()
        .route("/", get(api::list_domains).post(api::create_domain))
        .route("/{id}", get(api::get_domain).put(api::update_domain).delete(api::delete_domain))
        .route("/{id}/collections", get(api::list_domain_collections))
        .layer(axum::middleware::from_fn_with_state(state.clone(), domains_api_key_auth));

    // Collections API - protected by API key with collections:* permissions
    let collections_api = Router::new()
        .route("/", get(api::list_collections).post(api::create_collection))
        .route("/{id}", get(api::get_collection).put(api::update_collection).delete(api::delete_collection))
        .layer(axum::middleware::from_fn_with_state(state.clone(), collections_api_key_auth));

    // Admin API - protected by session authentication (for Admin UI only)
    // This includes API key management and system operations
    let admin_api = Router::new()
        // API Keys management
        .route("/api-keys", get(admin_auth::list_api_keys).post(admin_auth::create_api_key))
        .route("/api-keys/{id}/enable", post(admin_auth::enable_api_key))
        .route("/api-keys/{id}/disable", post(admin_auth::disable_api_key))
        .route("/api-keys/{id}", delete(admin_auth::delete_api_key))
        // reCAPTCHA site key (for static login page)
        .route("/recaptcha-site-key", get(admin_auth::get_recaptcha_site_key))
        // System stats and health (also available to admin UI)
        .route("/stats", get(api::get_stats))
        // Import operations (admin only - creates endpoints/services)
        .route("/import/openapi", post(api::import_openapi))
        .route("/import/bundle", post(api::import_bundle))
        .layer(axum::middleware::from_fn_with_state(state.clone(), session::session_auth));

    // Public API routes (no authentication required)
    let public_api = Router::new()
        .route("/health", get(api::health_check));

    // Create protected static file service for admin UI
    let protected_static = Router::new()
        .fallback_service(ServeDir::new(&config.static_dir))
        .layer(axum::middleware::from_fn_with_state(state.clone(), session::session_auth));

    // Build admin router (serves static files + API)
    // All API access is now under /api with proper sub-paths
    tracing::info!("Building admin router");
    let admin_router = Router::new()
        .route("/admin/login", get(admin_login_page))  // Public login page - no authentication required
        .nest("/auth", create_admin_auth_router())    // Public auth routes (login, password change)
        .nest("/api/endpoints", endpoints_api)        // API key auth: endpoints:*
        .nest("/api/services", services_api)          // API key auth: services:*
        .nest("/api/domains", domains_api)            // API key auth: domains:*
        .nest("/api/collections", collections_api)    // API key auth: collections:*
        .nest("/api/admin", admin_api)                // Session auth: Admin UI only
        .nest("/api", public_api)                     // Public: health check
        .fallback_service(protected_static.into_service());
    tracing::info!("Admin router built successfully");

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
