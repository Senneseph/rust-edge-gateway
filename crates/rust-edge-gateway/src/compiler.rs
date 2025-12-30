//! Handler compilation service
//!
//! Compiles uploaded Rust source files into dynamic library handlers (v2 architecture).

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Command;
use tokio::task;

use crate::config::AppConfig;

/// Template for handler Cargo.toml (v2 - dynamic library)
const CARGO_TOML_TEMPLATE: &str = r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[workspace]

[lib]
crate-type = ["cdylib"]

[dependencies]
rust-edge-gateway-sdk = { path = "{sdk_path}" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
{extra_dependencies}

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
"#;

/// Convert JSON dependencies to TOML format
///
/// Accepts dependencies in Cargo.toml JSON format:
/// - Simple: `{"regex": "1.10"}` -> `regex = "1.10"`
/// - Complex: `{"chrono": {"version": "0.4", "features": ["serde"]}}` -> `chrono = { version = "0.4", features = ["serde"] }`
fn json_deps_to_toml(deps: &serde_json::Value) -> String {
    let Some(obj) = deps.as_object() else {
        return String::new();
    };

    let mut lines = Vec::new();

    for (name, value) in obj {
        let toml_value = match value {
            // Simple version string: "1.10" -> "1.10"
            serde_json::Value::String(version) => {
                format!("{} = \"{}\"", name, version)
            }
            // Complex object: {"version": "0.4", "features": ["serde"]}
            serde_json::Value::Object(spec) => {
                let mut parts = Vec::new();

                // version
                if let Some(serde_json::Value::String(v)) = spec.get("version") {
                    parts.push(format!("version = \"{}\"", v));
                }

                // features
                if let Some(serde_json::Value::Array(features)) = spec.get("features") {
                    let features_str: Vec<String> = features
                        .iter()
                        .filter_map(|f| f.as_str().map(|s| format!("\"{}\"", s)))
                        .collect();
                    if !features_str.is_empty() {
                        parts.push(format!("features = [{}]", features_str.join(", ")));
                    }
                }

                // optional
                if let Some(serde_json::Value::Bool(opt)) = spec.get("optional") {
                    if *opt {
                        parts.push("optional = true".to_string());
                    }
                }

                // default-features
                if let Some(serde_json::Value::Bool(df)) = spec.get("default-features") {
                    if !*df {
                        parts.push("default-features = false".to_string());
                    }
                }

                format!("{} = {{ {} }}", name, parts.join(", "))
            }
            _ => continue,
        };

        lines.push(toml_value);
    }

    lines.join("\n")
}

/// Template for handler lib.rs wrapper (v2 - dynamic library entry point)
///
/// The user's handler.rs should define a function like:
/// ```ignore
/// use rust_edge_gateway_sdk::prelude::*;
///
/// pub fn handle(ctx: &Context, req: Request) -> Response {
///     Response::ok(json!({"message": "Hello!"}))
/// }
/// ```
const LIB_RS_TEMPLATE: &str = r#"//! Auto-generated handler wrapper (v2 dynamic library)
use rust_edge_gateway_sdk::prelude::*;

mod handler;

/// Entry point called by the gateway to handle requests.
/// This is the v2 dynamic library interface.
///
/// Signature matches runtime HandlerFn: extern "C" fn(&Context, Request) -> Response
#[no_mangle]
pub extern "C" fn handler_entry(ctx: &Context, req: Request) -> Response {
    handler::handle(ctx, req)
}
"#;

/// Compile a handler from source code
///
/// # Arguments
/// * `config` - Application configuration
/// * `id` - Handler ID (used for directory and package naming)
/// * `code` - Handler source code
/// * `dependencies` - Optional JSON dependencies to include in Cargo.toml
pub async fn compile_handler(
    config: &AppConfig,
    id: &str,
    code: &str,
    dependencies: Option<&serde_json::Value>,
) -> Result<String> {
    let handlers_dir = config.handlers_dir.clone();
    let id = id.to_string();
    let code = code.to_string();
    let deps = dependencies.cloned();

    // Run compilation in a blocking task
    task::spawn_blocking(move || {
        compile_handler_sync(&handlers_dir, &id, &code, deps.as_ref())
    }).await?
}

fn compile_handler_sync(
    handlers_dir: &PathBuf,
    id: &str,
    code: &str,
    dependencies: Option<&serde_json::Value>,
) -> Result<String> {
    // Create handler directory structure
    let handler_dir = handlers_dir.join(id);
    let src_dir = handler_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;

    // Calculate relative path to SDK
    // Assuming handlers_dir is at ./handlers and SDK is at ./crates/rust-edge-gateway-sdk
    let sdk_path = "../../crates/rust-edge-gateway-sdk";

    // Package name must start with a letter, so prefix with "handler_"
    let package_name = format!("handler_{}", id.replace('-', "_"));

    // Convert JSON dependencies to TOML format
    let extra_deps = dependencies
        .map(json_deps_to_toml)
        .unwrap_or_default();

    // Write Cargo.toml
    let cargo_toml = CARGO_TOML_TEMPLATE
        .replace("{name}", &package_name)
        .replace("{sdk_path}", sdk_path)
        .replace("{extra_dependencies}", &extra_deps);
    std::fs::write(handler_dir.join("Cargo.toml"), cargo_toml)?;

    // Write lib.rs wrapper (v2 dynamic library entry point)
    std::fs::write(src_dir.join("lib.rs"), LIB_RS_TEMPLATE)?;

    // Write user's handler code
    std::fs::write(src_dir.join("handler.rs"), code)?;

    // Compile with cargo
    tracing::info!("Compiling handler {} in {:?}", id, handler_dir);

    let output = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&handler_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Compilation failed:\n{}", stderr));
    }

    // Determine the library filename based on platform
    let lib_filename = format_library_name(&package_name);

    // The library is built in target/release/
    let lib_in_target = handler_dir
        .join("target")
        .join("release")
        .join(&lib_filename);

    if !lib_in_target.exists() {
        return Err(anyhow!("Library not found after compilation: {:?}", lib_in_target));
    }

    // Copy the library to the handler directory root for the registry to find
    // The registry expects: handlers/{id}/libhandler_{id}.so
    let lib_dest = handler_dir.join(&lib_filename);
    std::fs::copy(&lib_in_target, &lib_dest)?;

    tracing::info!("Handler compiled: {:?}", lib_dest);

    Ok(lib_dest.to_string_lossy().to_string())
}

/// Format the library filename for the current platform
#[cfg(target_os = "windows")]
fn format_library_name(package_name: &str) -> String {
    format!("{}.dll", package_name)
}

#[cfg(target_os = "linux")]
fn format_library_name(package_name: &str) -> String {
    format!("lib{}.so", package_name)
}

#[cfg(target_os = "macos")]
fn format_library_name(package_name: &str) -> String {
    format!("lib{}.dylib", package_name)
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn format_library_name(package_name: &str) -> String {
    format!("lib{}.so", package_name)
}
