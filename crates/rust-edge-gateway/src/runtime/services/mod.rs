//! Actor-based services
//!
//! This module provides long-lived service actors with connection pooling:
//! - Database (generic SQL interface)
//! - Cache (Redis-like key-value)
//! - Storage (S3-compatible object storage)
//! - Email (SMTP)

pub mod database;
pub mod cache;
pub mod storage;

use std::sync::Arc;
use serde::{Deserialize, Serialize};

pub use database::{Database, DatabaseConfig, DatabaseCommand};
pub use cache::{Cache, CacheConfig, CacheCommand};
pub use storage::{ObjectStore, StorageConfig, StorageCommand};

/// Container for all available services
/// 
/// This is passed to handlers via Context and provides access to
/// pre-established, long-lived service connections.
#[derive(Clone, Default)]
pub struct Services {
    /// Database service (MySQL, PostgreSQL, SQLite)
    pub db: Option<Database>,
    
    /// Cache service (Redis, Memcached)
    pub cache: Option<Cache>,
    
    /// Object storage service (MinIO, S3)
    pub storage: Option<ObjectStore>,
}

impl Services {
    /// Create a new empty services container
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Builder pattern: add database service
    pub fn with_db(mut self, db: Database) -> Self {
        self.db = Some(db);
        self
    }
    
    /// Builder pattern: add cache service
    pub fn with_cache(mut self, cache: Cache) -> Self {
        self.cache = Some(cache);
        self
    }
    
    /// Builder pattern: add storage service
    pub fn with_storage(mut self, storage: ObjectStore) -> Self {
        self.storage = Some(storage);
        self
    }
    
    /// Get database or return error
    pub fn require_db(&self) -> Result<&Database, ServiceError> {
        self.db.as_ref()
            .ok_or(ServiceError::NotConfigured("database"))
    }
    
    /// Get cache or return error
    pub fn require_cache(&self) -> Result<&Cache, ServiceError> {
        self.cache.as_ref()
            .ok_or(ServiceError::NotConfigured("cache"))
    }
    
    /// Get storage or return error
    pub fn require_storage(&self) -> Result<&ObjectStore, ServiceError> {
        self.storage.as_ref()
            .ok_or(ServiceError::NotConfigured("storage"))
    }
}

impl std::fmt::Debug for Services {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Services")
            .field("db", &self.db.is_some())
            .field("cache", &self.cache.is_some())
            .field("storage", &self.storage.is_some())
            .finish()
    }
}

/// Errors related to service operations
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Service not configured: {0}")]
    NotConfigured(&'static str),
    
    #[error("Service unavailable: {0}")]
    Unavailable(String),
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Query failed: {0}")]
    QueryFailed(String),
    
    #[error("Operation timed out")]
    Timeout,
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Service type enumeration for configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Sqlite,
    Mysql,
    Postgres,
    Redis,
    Memcached,
    Minio,
    S3,
    Smtp,
}

/// Generic service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service_type: ServiceType,
    pub name: String,
    #[serde(flatten)]
    pub config: serde_json::Value,
}

/// Service manager for starting and stopping services
pub struct ServiceManager {
    services: Services,
    configs: Vec<ServiceConfig>,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            services: Services::new(),
            configs: Vec::new(),
        }
    }
    
    /// Start all configured services
    pub async fn start_services(&mut self, configs: Vec<ServiceConfig>) -> Result<Services, ServiceError> {
        let mut services = Services::new();
        
        for config in configs {
            match config.service_type {
                ServiceType::Sqlite | ServiceType::Mysql | ServiceType::Postgres => {
                    let db_config: DatabaseConfig = serde_json::from_value(config.config.clone())
                        .map_err(|e| ServiceError::InvalidConfig(e.to_string()))?;
                    let db = Database::start(db_config).await?;
                    services.db = Some(db);
                }
                ServiceType::Redis | ServiceType::Memcached => {
                    let cache_config: CacheConfig = serde_json::from_value(config.config.clone())
                        .map_err(|e| ServiceError::InvalidConfig(e.to_string()))?;
                    let cache = Cache::start(cache_config).await?;
                    services.cache = Some(cache);
                }
                ServiceType::Minio | ServiceType::S3 => {
                    let storage_config: StorageConfig = serde_json::from_value(config.config.clone())
                        .map_err(|e| ServiceError::InvalidConfig(e.to_string()))?;
                    let storage = ObjectStore::start(storage_config).await?;
                    services.storage = Some(storage);
                }
                ServiceType::Smtp => {
                    // Email service - TODO
                    tracing::warn!("SMTP service not yet implemented");
                }
            }
        }
        
        self.services = services.clone();
        Ok(services)
    }
    
    /// Get current services
    pub fn services(&self) -> &Services {
        &self.services
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}