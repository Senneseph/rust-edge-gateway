//! MySQL/MariaDB Service Connector

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::ServiceConnector;
use crate::api::ServiceType;

/// MySQL connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MysqlConfig {
    /// Database host
    pub host: String,
    /// Database port
    #[serde(default = "default_mysql_port")]
    pub port: u16,
    /// Database name
    pub database: String,
    /// Username
    pub username: String,
    /// Password (not serialized for safety)
    #[serde(skip_serializing)]
    pub password: String,
    /// Use SSL/TLS
    #[serde(default)]
    pub use_ssl: bool,
    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
}

fn default_mysql_port() -> u16 { 3306 }
fn default_pool_size() -> u32 { 10 }

/// MySQL service connector
pub struct MysqlConnector {
    config: MysqlConfig,
}

impl MysqlConnector {
    pub fn new(config: MysqlConfig) -> Self {
        Self { config }
    }
    
    /// Get the connection string (without password)
    pub fn connection_string(&self) -> String {
        format!(
            "mysql://{}@{}:{}/{}",
            self.config.username,
            self.config.host,
            self.config.port,
            self.config.database
        )
    }
    
    /// Get configuration for establishing connection
    pub fn get_config(&self) -> &MysqlConfig {
        &self.config
    }
}

impl ServiceConnector for MysqlConnector {
    fn service_type(&self) -> ServiceType {
        ServiceType::Mysql
    }
    
    fn test_connection(&self) -> Result<()> {
        // Would establish a test connection
        // Placeholder - actual implementation would use mysql crate
        Ok(())
    }
    
    fn connection_info(&self) -> Value {
        json!({
            "type": "mysql",
            "host": self.config.host,
            "port": self.config.port,
            "database": self.config.database,
            "username": self.config.username,
            "use_ssl": self.config.use_ssl,
            "pool_size": self.config.pool_size,
        })
    }
}

