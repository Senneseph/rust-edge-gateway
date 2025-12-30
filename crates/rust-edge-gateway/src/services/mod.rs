//! Service Connectors
//!
//! This module provides connectors for various backend services:
//! - SQLite - Embedded database
//! - MinIO - S3-compatible object storage
//! - MySQL - MySQL/MariaDB database
//! - PostgreSQL - PostgreSQL database
//! - Redis - In-memory data store
//! - Memcached - Distributed memory caching
//! - MongoDB - Document database
//! - FTP/SFTP - File transfer services
//! - Email - SMTP email sending

use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

use crate::api::ServiceType;

/// Common trait for all service connectors
pub trait ServiceConnector: Send + Sync {
    /// Get the service type
    fn service_type(&self) -> ServiceType;
    
    /// Test the connection
    fn test_connection(&self) -> Result<()>;
    
    /// Get connection info (sanitized, no passwords)
    fn connection_info(&self) -> Value;
}

/// Service registry for managing active service connections
pub struct ServiceRegistry {
    services: std::collections::HashMap<String, Arc<dyn ServiceConnector>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: std::collections::HashMap::new(),
        }
    }
    
    /// Register a service connector
    pub fn register(&mut self, id: String, connector: Arc<dyn ServiceConnector>) {
        self.services.insert(id, connector);
    }
    
    /// Get a service connector by ID
    pub fn get(&self, id: &str) -> Option<Arc<dyn ServiceConnector>> {
        self.services.get(id).cloned()
    }
    
    /// Remove a service connector
    pub fn remove(&mut self, id: &str) -> Option<Arc<dyn ServiceConnector>> {
        self.services.remove(id)
    }
    
    /// List all registered service IDs
    pub fn list(&self) -> Vec<String> {
        self.services.keys().cloned().collect()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a service connector from config
///
/// NOTE: Per the Service Provider Architecture, connectors should be dynamically
/// loaded rather than baked into the gateway. This function returns an error
/// indicating the service type is not yet implemented.
pub fn create_connector(service_type: ServiceType, _config: &Value) -> Result<Arc<dyn ServiceConnector>> {
    anyhow::bail!(
        "Service type {:?} is not yet implemented. Service Providers should be loaded dynamically.",
        service_type
    )
}

