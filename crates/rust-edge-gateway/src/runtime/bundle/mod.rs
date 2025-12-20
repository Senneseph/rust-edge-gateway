//! Bundle deployment system
//!
//! This module handles the new v2 bundle format with manifest-based configuration.
//!
//! Bundle structure:
//! ```
//! my-api.bundle/
//! ├── bundle.yaml           # Manifest
//! ├── handlers/
//! │   ├── get_user.so      # Compiled handler (Linux)
//! │   ├── create_user.so
//! │   └── list_users.so
//! ├── schemas/
//! │   └── openapi.yaml     # API specification
//! └── assets/
//!     └── migrations.sql   # Database migrations
//! ```

pub mod manifest;
pub mod deploy;

pub use manifest::BundleManifest;
pub use deploy::BundleDeployer;