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

#![allow(dead_code)]

pub mod sqlite;
pub mod minio;
pub mod mysql;
pub mod postgres;
pub mod redis;
pub mod memcached;
pub mod mongodb;

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
pub fn create_connector(service_type: ServiceType, config: &Value) -> Result<Arc<dyn ServiceConnector>> {
    match service_type {
        ServiceType::Sqlite => {
            let cfg: sqlite::SqliteConfig = serde_json::from_value(config.clone())?;
            Ok(Arc::new(sqlite::SqliteConnector::new(cfg)?))
        }
        ServiceType::Minio => {
            let cfg: minio::MinioConfig = serde_json::from_value(config.clone())?;
            Ok(Arc::new(minio::MinioConnector::new(cfg)))
        }
        ServiceType::Mysql => {
            let cfg: mysql::MysqlConfig = serde_json::from_value(config.clone())?;
            Ok(Arc::new(mysql::MysqlConnector::new(cfg)))
        }
        ServiceType::Postgres => {
            let cfg: postgres::PostgresConfig = serde_json::from_value(config.clone())?;
            Ok(Arc::new(postgres::PostgresConnector::new(cfg)))
        }
        ServiceType::Redis => {
            let cfg: redis::RedisConfig = serde_json::from_value(config.clone())?;
            Ok(Arc::new(redis::RedisConnector::new(cfg)))
        }
        ServiceType::Memcached => {
            let cfg: memcached::MemcachedConfig = serde_json::from_value(config.clone())?;
            Ok(Arc::new(memcached::MemcachedConnector::new(cfg)))
        }
        ServiceType::Mongodb => {
            let cfg: mongodb::MongodbConfig = serde_json::from_value(config.clone())?;
            Ok(Arc::new(mongodb::MongodbConnector::new(cfg)))
        }
    }
}

