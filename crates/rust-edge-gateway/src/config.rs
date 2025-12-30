//! Application configuration

use std::env;
use std::path::PathBuf;

/// Application configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Directory for SQLite database and other persistent data
    pub data_dir: PathBuf,
    
    /// Directory for compiled handler binaries
    pub handlers_dir: PathBuf,
    
    /// Directory for static admin UI files
    pub static_dir: PathBuf,
    
    /// Port for the main gateway (HTTP requests to handlers)
    pub gateway_port: u16,
    
    /// Port for the admin UI and API
    pub admin_port: u16,
    
    /// API key for admin authentication (simple auth for MVP)
    pub admin_api_key: Option<String>,

    /// Bootstrap API key to create on startup if it doesn't exist
    /// This allows setting an API key via environment variable
    pub bootstrap_api_key: Option<String>,

    /// Default admin password for initial setup
    pub default_admin_password: Option<String>,

    /// reCAPTCHA v3 site key for client-side integration
    pub recaptcha_site_key: Option<String>,

    /// reCAPTCHA v3 secret key for server-side verification
    pub recaptcha_secret_key: Option<String>,

    /// Handler request timeout in seconds
    pub handler_timeout_secs: u64,

    /// Maximum handler memory in MB (for monitoring)
    pub handler_max_memory_mb: u64,
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            data_dir: env::var("RUST_EDGE_GATEWAY_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./data")),

            handlers_dir: env::var("RUST_EDGE_GATEWAY_HANDLERS_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./handlers")),

            static_dir: env::var("RUST_EDGE_GATEWAY_STATIC_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./static")),

            gateway_port: env::var("RUST_EDGE_GATEWAY_GATEWAY_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080),

            admin_port: env::var("RUST_EDGE_GATEWAY_ADMIN_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8081),

            admin_api_key: env::var("RUST_EDGE_GATEWAY_ADMIN_API_KEY").ok(),

            // Bootstrap API key - will be created on startup with full permissions
            bootstrap_api_key: env::var("RUST_EDGE_GATEWAY_API_KEY").ok(),

            handler_timeout_secs: env::var("RUST_EDGE_GATEWAY_HANDLER_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),

            handler_max_memory_mb: env::var("RUST_EDGE_GATEWAY_HANDLER_MAX_MEMORY_MB")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(64),

            default_admin_password: env::var("DEFAULT_ADMIN_PASSWORD").ok(),

            recaptcha_site_key: env::var("RECAPTCHA_V3_SITE_KEY").ok(),
            recaptcha_secret_key: env::var("RECAPTCHA_V3_SECRET_KEY").ok(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

