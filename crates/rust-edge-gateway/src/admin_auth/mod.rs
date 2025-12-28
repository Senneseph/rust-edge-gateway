//! Admin authentication middleware and handlers
//!
//! This module provides authentication functionality for the admin interface:
//! - Password validation
//! - reCAPTCHA verification
//! - Basic auth middleware
//! - API key authentication middleware
//! - Login/logout/password change handlers
//! - API key management handlers

mod api_keys;
mod api_keys_actions;
mod api_middleware;
mod handlers;
mod middleware;
mod password;
mod recaptcha;
mod routes;
mod types;

// Re-export public types
pub use types::{
    AdminUserExtract, ApiKeyExtract, ChangePasswordData, CreateApiKeyData, JsonResponse,
    LoginData, LoginResponse, PasswordChangeResponse,
};

// Re-export password validation
pub use password::validate_password;

// Re-export reCAPTCHA verification
pub use recaptcha::verify_recaptcha_token;

// Re-export middleware functions
pub use api_middleware::{
    collections_api_key_auth, domains_api_key_auth, endpoints_api_key_auth, services_api_key_auth,
};
pub use middleware::{admin_auth, api_key_auth};

// Re-export handlers
pub use api_keys::{create_api_key, list_api_keys};
pub use api_keys_actions::{delete_api_key, disable_api_key, enable_api_key};
pub use handlers::{admin_login_page, change_password, get_recaptcha_site_key, login, login_page, logout};

// Re-export router creation functions
pub use routes::{create_admin_auth_router, create_protected_admin_routes};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_validation() {
        // Test that password validation works
        assert!(validate_password("Short1!").is_err());
        assert!(validate_password("ValidPassword1!").is_ok());
    }
}

