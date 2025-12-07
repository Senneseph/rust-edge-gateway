//! MinIO/S3-compatible Object Storage Service Connector

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::ServiceConnector;
use crate::api::ServiceType;

/// MinIO connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinioConfig {
    /// MinIO endpoint URL (e.g., "http://localhost:9000")
    pub endpoint: String,
    /// Access key ID
    pub access_key: String,
    /// Secret access key
    #[serde(skip_serializing)]
    pub secret_key: String,
    /// Default bucket name
    pub bucket: Option<String>,
    /// Use SSL/TLS
    #[serde(default)]
    pub use_ssl: bool,
    /// AWS region (for S3 compatibility)
    #[serde(default = "default_region")]
    pub region: String,
}

fn default_region() -> String { "us-east-1".to_string() }

/// MinIO service connector
pub struct MinioConnector {
    config: MinioConfig,
}

impl MinioConnector {
    pub fn new(config: MinioConfig) -> Self {
        Self { config }
    }
    
    /// Get presigned URL for object upload
    pub fn presigned_put_url(&self, bucket: &str, key: &str, expires_secs: u64) -> Result<String> {
        // Note: Full implementation would use aws-sdk-s3 or similar
        // This is a placeholder showing the interface
        Ok(format!(
            "{}/{}/{}?X-Amz-Expires={}",
            self.config.endpoint, bucket, key, expires_secs
        ))
    }
    
    /// Get presigned URL for object download
    pub fn presigned_get_url(&self, bucket: &str, key: &str, expires_secs: u64) -> Result<String> {
        Ok(format!(
            "{}/{}/{}?X-Amz-Expires={}",
            self.config.endpoint, bucket, key, expires_secs
        ))
    }
    
    /// Get the configured endpoint
    pub fn endpoint(&self) -> &str {
        &self.config.endpoint
    }
    
    /// Get default bucket if configured
    pub fn default_bucket(&self) -> Option<&str> {
        self.config.bucket.as_deref()
    }
}

impl ServiceConnector for MinioConnector {
    fn service_type(&self) -> ServiceType {
        ServiceType::Minio
    }
    
    fn test_connection(&self) -> Result<()> {
        // Would make a HEAD request to the endpoint
        // Placeholder for now
        Ok(())
    }
    
    fn connection_info(&self) -> Value {
        json!({
            "type": "minio",
            "endpoint": self.config.endpoint,
            "bucket": self.config.bucket,
            "use_ssl": self.config.use_ssl,
            "region": self.config.region,
        })
    }
}

