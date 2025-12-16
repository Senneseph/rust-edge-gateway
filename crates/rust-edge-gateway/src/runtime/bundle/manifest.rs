//! Bundle manifest parsing
//!
//! Parses bundle.yaml manifest files with environment variable substitution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};

/// The bundle manifest (bundle.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    /// Bundle metadata
    pub bundle: BundleInfo,
    
    /// Domains this bundle responds to
    #[serde(default)]
    pub domains: Vec<String>,
    
    /// TLS configuration
    #[serde(default)]
    pub tls: Option<TlsConfig>,
    
    /// Service configurations
    #[serde(default)]
    pub services: HashMap<String, ServiceConfig>,
    
    /// Route definitions
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
}

/// Bundle metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleInfo {
    /// Bundle name (unique identifier)
    pub name: String,
    
    /// Semantic version
    pub version: String,
    
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    
    /// Author information
    #[serde(default)]
    pub author: Option<String>,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// TLS provider: letsencrypt, manual, none
    #[serde(default = "default_tls_provider")]
    pub provider: String,
    
    /// Email for Let's Encrypt
    #[serde(default)]
    pub email: Option<String>,
    
    /// Enable HTTP-01 challenge
    #[serde(default = "default_true")]
    pub http_challenge: bool,
}

fn default_tls_provider() -> String { "none".to_string() }
fn default_true() -> bool { true }

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service type: mysql, postgres, sqlite, redis, minio, etc.
    pub kind: String,
    
    /// Connection string or DSN (supports ${ENV_VAR} syntax)
    #[serde(default)]
    pub dsn: Option<String>,
    
    /// Connection URL (alternative to dsn)
    #[serde(default)]
    pub url: Option<String>,
    
    /// Endpoint (for MinIO/S3)
    #[serde(default)]
    pub endpoint: Option<String>,
    
    /// Connection pool size
    #[serde(default)]
    pub pool_size: Option<u32>,
    
    /// Additional configuration as key-value pairs
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Route configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    /// HTTP method (GET, POST, PUT, DELETE, PATCH, etc.)
    pub method: String,
    
    /// URL path pattern (supports {param} syntax)
    pub path: String,
    
    /// Handler name (matches .so/.dll filename without extension)
    pub handler: String,
    
    /// Optional middleware
    #[serde(default)]
    pub middleware: Vec<String>,
    
    /// Optional timeout override (seconds)
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

impl BundleManifest {
    /// Parse a manifest from YAML content
    pub fn parse(yaml: &str) -> Result<Self> {
        // First, substitute environment variables
        let expanded = expand_env_vars(yaml);
        
        // Then parse
        serde_yaml::from_str(&expanded)
            .context("Failed to parse bundle manifest")
    }
    
    /// Load a manifest from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read manifest file: {:?}", path.as_ref()))?;
        Self::parse(&content)
    }
    
    /// Validate the manifest
    pub fn validate(&self) -> Result<()> {
        // Check required fields
        if self.bundle.name.is_empty() {
            anyhow::bail!("Bundle name is required");
        }
        if self.bundle.version.is_empty() {
            anyhow::bail!("Bundle version is required");
        }
        
        // Validate routes
        for route in &self.routes {
            if route.method.is_empty() {
                anyhow::bail!("Route method is required");
            }
            if route.path.is_empty() {
                anyhow::bail!("Route path is required");
            }
            if route.handler.is_empty() {
                anyhow::bail!("Route handler is required");
            }
        }
        
        // Validate services
        for (name, config) in &self.services {
            if config.kind.is_empty() {
                anyhow::bail!("Service '{}' must have a 'kind' field", name);
            }
        }
        
        Ok(())
    }
    
    /// Get the connection string for a service
    pub fn get_service_connection(&self, name: &str) -> Option<String> {
        self.services.get(name).and_then(|s| {
            s.dsn.clone().or_else(|| s.url.clone())
        })
    }
}

/// Expand environment variables in a string
/// Supports: ${VAR}, ${VAR:-default}, $VAR
fn expand_env_vars(input: &str) -> String {
    let mut result = input.to_string();
    
    // Pattern: ${VAR:-default} or ${VAR}
    let re = regex_lite::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)(?::-([^}]*))?\}").unwrap();
    result = re.replace_all(&result, |caps: &regex_lite::Captures| {
        let var_name = &caps[1];
        let default = caps.get(2).map(|m| m.as_str());
        
        std::env::var(var_name)
            .unwrap_or_else(|_| default.unwrap_or("").to_string())
    }).to_string();
    
    // Pattern: $VAR (simple)
    let re = regex_lite::Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    result = re.replace_all(&result, |caps: &regex_lite::Captures| {
        let var_name = &caps[1];
        std::env::var(var_name).unwrap_or_default()
    }).to_string();
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_minimal_manifest() {
        let yaml = r#"
bundle:
  name: test-api
  version: 1.0.0

routes:
  - method: GET
    path: /health
    handler: health_check
"#;
        
        let manifest = BundleManifest::parse(yaml).unwrap();
        assert_eq!(manifest.bundle.name, "test-api");
        assert_eq!(manifest.bundle.version, "1.0.0");
        assert_eq!(manifest.routes.len(), 1);
    }
    
    #[test]
    fn test_parse_full_manifest() {
        let yaml = r#"
bundle:
  name: users-api
  version: 1.2.3
  description: User management API
  author: Test Author

domains:
  - api.example.com
  - example.com

tls:
  provider: letsencrypt
  email: admin@example.com

services:
  mysql:
    kind: mysql
    dsn: mysql://user:pass@localhost/db
    pool_size: 10
  
  redis:
    kind: redis
    url: redis://localhost:6379

routes:
  - method: GET
    path: /users/{id}
    handler: get_user
    timeout_secs: 30
  
  - method: POST
    path: /users
    handler: create_user
"#;
        
        let manifest = BundleManifest::parse(yaml).unwrap();
        assert_eq!(manifest.bundle.name, "users-api");
        assert_eq!(manifest.domains.len(), 2);
        assert_eq!(manifest.services.len(), 2);
        assert_eq!(manifest.routes.len(), 2);
        assert!(manifest.tls.is_some());
    }
    
    #[test]
    fn test_env_var_expansion() {
        std::env::set_var("TEST_VAR", "hello");
        
        let input = "value: ${TEST_VAR}";
        let expanded = expand_env_vars(input);
        assert_eq!(expanded, "value: hello");
        
        let input_with_default = "value: ${MISSING_VAR:-default_value}";
        let expanded = expand_env_vars(input_with_default);
        assert_eq!(expanded, "value: default_value");
    }
}