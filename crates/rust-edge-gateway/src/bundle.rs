//! Bundle Import - Import a .zip containing OpenAPI spec and handler code
//!
//! Bundle structure:
//! ```
//! bundle.zip
//! ├── openapi.yaml (or openapi.json, api.yaml, api.json, spec.yaml, spec.json)
//! └── handlers/
//!     ├── get_pet.rs         # Matches operationId "get_pet" or "getPet"
//!     ├── create_pet.rs      # Matches operationId "create_pet" or "createPet"
//!     └── list_pets.rs       # Matches operationId "list_pets" or "listPets"
//! ```
//!
//! Handler files can also be at the root level or in a `src/` directory.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Parsed bundle contents
#[derive(Debug)]
pub struct ParsedBundle {
    /// OpenAPI spec content (if found)
    pub openapi_spec: Option<String>,
    /// Handler code mapped by normalized operation ID
    pub handlers: HashMap<String, String>,
}

/// Possible OpenAPI file names (in priority order)
const OPENAPI_FILENAMES: &[&str] = &[
    "openapi.yaml",
    "openapi.yml",
    "openapi.json",
    "api.yaml",
    "api.yml",
    "api.json",
    "spec.yaml",
    "spec.yml",
    "spec.json",
    "swagger.yaml",
    "swagger.yml",
    "swagger.json",
];

/// Parse a zip bundle and extract OpenAPI spec and handlers
pub fn parse_bundle(zip_bytes: &[u8]) -> Result<ParsedBundle> {
    let cursor = Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor)
        .context("Failed to read zip archive")?;

    let mut openapi_spec = None;
    let mut handlers = HashMap::new();

    tracing::debug!("Parsing bundle with {} files", archive.len());

    // First pass: find OpenAPI spec
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let raw_name = file.name().to_string();
        // Normalize path separators (Windows uses backslashes)
        let name = raw_name.replace('\\', "/").to_lowercase();

        tracing::debug!("Bundle file {}: raw={:?} normalized={:?}", i, raw_name, name);

        // Check if this is an OpenAPI spec file
        let filename = name.rsplit('/').next().unwrap_or(&name);
        if OPENAPI_FILENAMES.contains(&filename) && openapi_spec.is_none() {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            openapi_spec = Some(content);
            tracing::debug!("Found OpenAPI spec: {}", filename);
        }
    }

    // Second pass: find handler files
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let raw_path = file.name().to_string();
        // Normalize path separators (Windows uses backslashes)
        let path = raw_path.replace('\\', "/");

        // Skip directories and non-.rs files
        if file.is_dir() || !path.to_lowercase().ends_with(".rs") {
            continue;
        }

        // Extract the handler name from the filename
        let filename = path.rsplit('/').next().unwrap_or(&path);
        let handler_name = filename.trim_end_matches(".rs").trim_end_matches(".RS");

        // Skip if it looks like a Rust internal file
        if handler_name == "main" || handler_name == "lib" || handler_name == "mod" {
            continue;
        }

        // Read handler content
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        // Normalize the handler name for matching
        let normalized = normalize_handler_name(handler_name);
        tracing::debug!("Found handler: raw={:?} filename={:?} normalized={:?}", raw_path, handler_name, normalized);
        handlers.insert(normalized, content);
    }

    tracing::debug!("Parsed bundle: openapi={}, handlers={:?}", openapi_spec.is_some(), handlers.keys().collect::<Vec<_>>());

    Ok(ParsedBundle {
        openapi_spec,
        handlers,
    })
}

/// Normalize a handler name for matching against operationIds
/// Converts: "getPet" -> "get_pet", "get-pet" -> "get_pet", "GetPet" -> "get_pet"
pub fn normalize_handler_name(name: &str) -> String {
    let mut result = String::new();
    let mut prev_was_lower = false;
    
    for c in name.chars() {
        if c == '-' || c == '_' {
            result.push('_');
            prev_was_lower = false;
        } else if c.is_uppercase() {
            // Add underscore before uppercase if previous was lowercase (camelCase)
            if prev_was_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_was_lower = false;
        } else {
            result.push(c);
            prev_was_lower = c.is_lowercase();
        }
    }
    
    result
}

/// Match a handler to an operation ID from OpenAPI
/// Returns the handler code if a match is found
pub fn find_handler_for_operation(
    handlers: &HashMap<String, String>,
    operation_id: &str,
) -> Option<String> {
    let normalized_op_id = normalize_handler_name(operation_id);
    
    // Try exact match first
    if let Some(code) = handlers.get(&normalized_op_id) {
        return Some(code.clone());
    }
    
    // Try without underscores (for flexibility)
    let no_underscores: String = normalized_op_id.replace('_', "");
    for (name, code) in handlers {
        if name.replace('_', "") == no_underscores {
            return Some(code.clone());
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_normalize_handler_name() {
        assert_eq!(normalize_handler_name("getPet"), "get_pet");
        assert_eq!(normalize_handler_name("get_pet"), "get_pet");
        assert_eq!(normalize_handler_name("get-pet"), "get_pet");
        assert_eq!(normalize_handler_name("GetPet"), "get_pet");
        assert_eq!(normalize_handler_name("listAllPets"), "list_all_pets");
    }
}

