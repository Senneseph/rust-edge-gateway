//! Authentication handlers for login, logout, and password change

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use std::path::PathBuf;
use tower_http::services::ServeFile;
use bcrypt::verify;
use std::sync::Arc;
use tracing::info;

use crate::db_admin::AdminDatabase;
use crate::AppState;

use super::password::validate_password;
use super::recaptcha::verify_recaptcha_token;
use super::types::{
    ChangePasswordData, LoginData, LoginResponse, PasswordChangeResponse,
};

/// Handler for serving the admin login page (GET request) - no authentication required
pub async fn admin_login_page(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    tracing::info!("Serving admin login page");
    
    // Construct path to the admin login HTML file
    let mut login_html_path = PathBuf::from(&state.config.static_dir);
    login_html_path.push("admin");
    login_html_path.push("login.html");
    
    // Read the HTML file
    let html_content = match std::fs::read_to_string(login_html_path) {
        Ok(content) => content,
        Err(e) => {
            tracing::error!("Failed to read admin login.html: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to load login page".to_string()));
        }
    };
    
    Ok((
        [(header::CONTENT_TYPE, "text/html")],
        html_content
    ))
}

/// Handler for serving the login page (GET request)
pub async fn login_page(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    tracing::info!("Serving login page");
    
    // Construct path to the login HTML file
    let mut login_html_path = PathBuf::from(&state.config.static_dir);
    login_html_path.push("auth");
    login_html_path.push("login.html");
    
    // Read the HTML file
    let html_content = match std::fs::read_to_string(login_html_path) {
        Ok(content) => content,
        Err(e) => {
            tracing::error!("Failed to read login.html: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to load login page".to_string()));
        }
    };
    
    Ok((
        [(header::CONTENT_TYPE, "text/html")],
        html_content
    ))
}

/// Handler for login POST request
pub async fn login(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(login_data): axum::extract::Json<LoginData>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    tracing::info!("Login handler called for username: {}", login_data.username);
    
    // Validate reCAPTCHA token first
    if let Some(recaptcha_secret_key) = &state.config.recaptcha_secret_key {
        tracing::info!("Verifying reCAPTCHA token: {}", login_data.recaptcha_token);
        
        // Check if token is empty
        if login_data.recaptcha_token.is_empty() {
            tracing::error!("Empty reCAPTCHA token received");
            return Err((
                StatusCode::BAD_REQUEST,
                "reCAPTCHA verification failed: empty token".to_string(),
            ));
        }
        
        if !verify_recaptcha_token(recaptcha_secret_key, &login_data.recaptcha_token, "login")
            .await
            .map_err(|e| {
                tracing::error!("reCAPTCHA verification error: {}", e);
                (
                    StatusCode::BAD_REQUEST,
                    format!("reCAPTCHA verification failed: {}", e),
                )
            })?
        {
            tracing::warn!("reCAPTCHA verification failed for valid token check");
            return Err((
                StatusCode::BAD_REQUEST,
                "reCAPTCHA verification failed".to_string(),
            ));
        }
    } else {
        tracing::error!("reCAPTCHA not configured");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "reCAPTCHA not configured".to_string(),
        ));
    }

    // Check rate limit for this username
    if let Err(retry_after) = state.login_rate_limiter.check(&login_data.username) {
        info!(username = %login_data.username, "Login rate limited");
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "Too many login attempts. Try again in {} seconds",
                retry_after.as_secs()
            ),
        ));
    }

    let admin_db = AdminDatabase::new(&state.config.data_dir).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to initialize admin database".to_string(),
        )
    })?;

    let user = admin_db
        .get_admin_by_username(&login_data.username)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to query admin database".to_string(),
            )
        })?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Invalid username or password".to_string(),
        ))?;

    if !verify(&login_data.password, &user.password_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Password verification failed".to_string(),
        )
    })? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid username or password".to_string(),
        ));
    }

    // Reset rate limit on successful login
    state.login_rate_limiter.reset(&login_data.username);

    // Create session
    let session_id = state.session_store.create_session(&login_data.username);
    let session_cookie = crate::session::create_session_cookie(&session_id, 24 * 60 * 60); // 24 hours

    // If password change is required, return success but indicate change needed
    if user.requires_password_change {
        info!(username = %login_data.username, "Admin user logged in, password change required");
        return Ok((
            [(header::SET_COOKIE, session_cookie)],
            axum::Json(LoginResponse {
                success: true,
                requires_password_change: true,
                message: "Password change required".to_string(),
            }),
        ));
    }

    info!(username = %login_data.username, "Admin user logged in successfully");
    Ok((
        [(header::SET_COOKIE, session_cookie)],
        axum::Json(LoginResponse {
            success: true,
            requires_password_change: false,
            message: "Login successful".to_string(),
        }),
    ))
}

/// Handler for password change
pub async fn change_password(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(change_data): axum::extract::Json<ChangePasswordData>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Validate new password strength
    if let Err(err) = validate_password(&change_data.new_password) {
        return Err((StatusCode::BAD_REQUEST, err));
    }

    let admin_db = AdminDatabase::new(&state.config.data_dir).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to initialize admin database".to_string(),
        )
    })?;

    let user = admin_db
        .get_admin_by_username(&change_data.username)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to query admin database".to_string(),
            )
        })?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid username".to_string()))?;

    // Verify current password
    if !verify(&change_data.current_password, &user.password_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Password verification failed".to_string(),
        )
    })? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Current password is incorrect".to_string(),
        ));
    }

    // Update password
    admin_db
        .update_admin_password(&change_data.username, &change_data.new_password)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update password".to_string(),
            )
        })?;

    info!(username = %change_data.username, "Admin password changed successfully");
    Ok(axum::Json(PasswordChangeResponse {
        success: true,
        message: "Password changed successfully".to_string(),
    }))
}

/// Handler for logout
pub async fn logout(State(state): State<Arc<AppState>>, request: Request) -> impl IntoResponse {
    // Extract session ID from cookie and delete it
    if let Some(session_id) = crate::session::extract_session_id_from_request(&request) {
        state.session_store.delete_session(&session_id);
    }

    // Clear session cookie
    let delete_cookie = crate::session::delete_session_cookie();

    (
        [(header::SET_COOKIE, delete_cookie)],
        axum::response::Html(
            "<html><body><h1>Logged out</h1><p>You have been logged out.</p>\
             <a href='/admin/login.html'>Login again</a></body></html>",
        ),
    )
}

/// Handler for getting reCAPTCHA site key (for static HTML pages)
pub async fn get_recaptcha_site_key(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let site_key = state.config.recaptcha_site_key.clone().unwrap_or_default();

    Ok(axum::Json(super::types::JsonResponse {
        success: true,
        data: serde_json::Value::String(site_key),
    }))
}
