//! Bundle deployment logic
//!
//! Handles extracting, validating, and deploying bundles to the gateway.

use std::path::{Path, PathBuf};
use std::io::{Cursor, Read};
use std::fs;
use anyhow::{Context, Result};
use zip::ZipArchive;

use super::manifest::BundleManifest;

use crate::runtime::handler::HandlerRegistry;

/// Result of a bundle deployment
#[derive(Debug)]
pub struct DeploymentResult {
    /// Bundle name
    pub bundle_name: String,
    
    /// Bundle version
    pub bundle_version: String,
    
    /// Number of handlers loaded
    pub handlers_loaded: usize,
    
    /// Number of services started
    pub services_started: usize,
    
    /// Number of routes registered
    pub routes_registered: usize,
    
    /// Previous version (if upgrading)
    pub previous_version: Option<String>,
    
    /// Whether rollback is available
    pub rollback_available: bool,
}

/// Bundle deployer - handles the deployment lifecycle
pub struct BundleDeployer {
    /// Directory where bundles are stored
    bundles_dir: PathBuf,
    
    /// Directory where handlers are extracted
    handlers_dir: PathBuf,
    
    /// Currently deployed bundles
    deployed: std::collections::HashMap<String, DeployedBundle>,
}

/// A deployed bundle
struct DeployedBundle {
    manifest: BundleManifest,
    handlers_path: PathBuf,
    version: String,
}

impl BundleDeployer {
    /// Create a new bundle deployer
    pub fn new(bundles_dir: PathBuf, handlers_dir: PathBuf) -> Self {
        // Ensure directories exist
        fs::create_dir_all(&bundles_dir).ok();
        fs::create_dir_all(&handlers_dir).ok();
        
        Self {
            bundles_dir,
            handlers_dir,
            deployed: std::collections::HashMap::new(),
        }
    }
    
    /// Deploy a bundle from a zip archive
    pub async fn deploy(
        &mut self,
        zip_bytes: &[u8],
        handler_registry: &HandlerRegistry,
    ) -> Result<DeploymentResult> {
        // Extract and parse the bundle
        let extracted = self.extract_bundle(zip_bytes)?;
        
        // Validate the manifest
        extracted.manifest.validate()?;
        
        let bundle_name = extracted.manifest.bundle.name.clone();
        let bundle_version = extracted.manifest.bundle.version.clone();
        
        // Check for previous version
        let previous_version = self.deployed.get(&bundle_name).map(|b| b.version.clone());
        
        // Create bundle directory
        let bundle_dir = self.handlers_dir.join(&bundle_name);
        fs::create_dir_all(&bundle_dir)
            .with_context(|| format!("Failed to create bundle directory: {:?}", bundle_dir))?;
        
        // Extract handlers to bundle directory
        let handlers_loaded = self.extract_handlers(&extracted, &bundle_dir)?;
        
        // Load handlers into registry
        for handler_name in &extracted.handler_names {
            let handler_path = bundle_dir.join(format_library_name(handler_name));
            if handler_path.exists() {
                handler_registry.load_from(handler_name, &handler_path).await?;
            }
        }
        
        // Start services (would be done through ServiceManager in full implementation)
        let services_started = extracted.manifest.services.len();
        
        // Register routes (would be done through router in full implementation)
        let routes_registered = extracted.manifest.routes.len();
        
        // Store deployed bundle info
        self.deployed.insert(bundle_name.clone(), DeployedBundle {
            manifest: extracted.manifest,
            handlers_path: bundle_dir,
            version: bundle_version.clone(),
        });
        
        let rollback_available = previous_version.is_some();
        
        Ok(DeploymentResult {
            bundle_name,
            bundle_version,
            handlers_loaded,
            services_started,
            routes_registered,
            previous_version,
            rollback_available,
        })
    }
    
    /// Extract a bundle from zip bytes
    fn extract_bundle(&self, zip_bytes: &[u8]) -> Result<ExtractedBundle> {
        let cursor = Cursor::new(zip_bytes);
        let mut archive = ZipArchive::new(cursor)
            .context("Failed to read zip archive")?;
        
        let mut manifest = None;
        let mut handler_files = Vec::new();
        let mut handler_names = Vec::new();
        
        // First pass: find manifest and catalog files
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            
            // Check for manifest
            if name.ends_with("bundle.yaml") || name.ends_with("bundle.yml") {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                manifest = Some(BundleManifest::parse(&content)?);
            }
            
            // Check for handler libraries
            if is_handler_library(&name) {
                let handler_name = extract_handler_name(&name);
                handler_files.push(name);
                handler_names.push(handler_name);
            }
        }
        
        let manifest = manifest.ok_or_else(|| anyhow::anyhow!("No bundle.yaml found in archive"))?;
        
        Ok(ExtractedBundle {
            manifest,
            handler_files,
            handler_names,
            archive_bytes: zip_bytes.to_vec(),
        })
    }
    
    /// Extract handlers from bundle to target directory
    fn extract_handlers(&self, bundle: &ExtractedBundle, target_dir: &Path) -> Result<usize> {
        let cursor = Cursor::new(&bundle.archive_bytes);
        let mut archive = ZipArchive::new(cursor)?;
        
        let mut count = 0;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            
            if is_handler_library(&name) {
                let filename = Path::new(&name).file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or(name.clone());
                
                let target_path = target_dir.join(&filename);
                let mut target_file = fs::File::create(&target_path)?;
                std::io::copy(&mut file, &mut target_file)?;
                count += 1;
                
                tracing::info!("Extracted handler: {:?}", target_path);
            }
        }
        
        Ok(count)
    }
    
    /// Rollback to previous version
    pub async fn rollback(&mut self, bundle_name: &str) -> Result<()> {
        // In a full implementation, this would:
        // 1. Unload current handlers
        // 2. Load previous version handlers
        // 3. Restart services with previous config
        // 4. Update routing
        
        tracing::warn!("Rollback for {} - not yet fully implemented", bundle_name);
        Ok(())
    }
    
    /// Get deployed bundle info
    pub fn get_deployed(&self, bundle_name: &str) -> Option<&BundleManifest> {
        self.deployed.get(bundle_name).map(|b| &b.manifest)
    }
    
    /// List all deployed bundles
    pub fn list_deployed(&self) -> Vec<(&str, &str)> {
        self.deployed.iter()
            .map(|(name, b)| (name.as_str(), b.version.as_str()))
            .collect()
    }
}

/// Extracted bundle contents
struct ExtractedBundle {
    manifest: BundleManifest,
    handler_files: Vec<String>,
    handler_names: Vec<String>,
    archive_bytes: Vec<u8>,
}

/// Check if a file is a handler library
fn is_handler_library(name: &str) -> bool {
    name.ends_with(".so") || name.ends_with(".dll") || name.ends_with(".dylib")
}

/// Extract handler name from library filename
fn extract_handler_name(path: &str) -> String {
    let filename = Path::new(path).file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or(path.to_string());
    
    // Remove extension and prefix
    let name = filename
        .trim_end_matches(".so")
        .trim_end_matches(".dll")
        .trim_end_matches(".dylib")
        .trim_start_matches("lib")
        .trim_start_matches("handler_");
    
    name.to_string()
}

/// Format the library filename for the current platform
#[cfg(target_os = "windows")]
fn format_library_name(name: &str) -> String {
    format!("handler_{}.dll", name.replace('-', "_"))
}

#[cfg(target_os = "linux")]
fn format_library_name(name: &str) -> String {
    format!("libhandler_{}.so", name.replace('-', "_"))
}

#[cfg(target_os = "macos")]
fn format_library_name(name: &str) -> String {
    format!("libhandler_{}.dylib", name.replace('-', "_"))
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn format_library_name(name: &str) -> String {
    format!("libhandler_{}.so", name.replace('-', "_"))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_handler_library() {
        assert!(is_handler_library("handler.so"));
        assert!(is_handler_library("libhandler.dylib"));
        assert!(is_handler_library("handler.dll"));
        assert!(!is_handler_library("handler.rs"));
        assert!(!is_handler_library("bundle.yaml"));
    }
    
    #[test]
    fn test_extract_handler_name() {
        assert_eq!(extract_handler_name("libhandler_get_user.so"), "get_user");
        assert_eq!(extract_handler_name("handler_create_user.dll"), "create_user");
        assert_eq!(extract_handler_name("handlers/libhandler_list.so"), "list");
    }
}