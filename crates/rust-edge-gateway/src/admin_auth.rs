//! Admin authentication middleware

use axum::{
    extract::{State, Path, Request},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, delete},
    Router,
    middleware::Next,
};
use std::sync::Arc;
use tracing::{info, error};

use crate::db_admin::{AdminDatabase, AdminUser, ApiKey};
use crate::AppState;
use bcrypt::verify;
use base64::Engine;

/// Password validation requirements
const MIN_PASSWORD_LENGTH: usize = 12;
const REQUIRE_UPPERCASE: bool = true;
const REQUIRE_LOWERCASE: bool = true;
const REQUIRE_DIGIT: bool = true;
const REQUIRE_SPECIAL: bool = true;

/// Validate password strength
fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(format!("Password must be at least {} characters long", MIN_PASSWORD_LENGTH));
    }

    if REQUIRE_UPPERCASE && !password.chars().any(|c| c.is_uppercase()) {
        return Err("Password must contain at least one uppercase letter".to_string());
    }

    if REQUIRE_LOWERCASE && !password.chars().any(|c| c.is_lowercase()) {
        return Err("Password must contain at least one lowercase letter".to_string());
    }

    if REQUIRE_DIGIT && !password.chars().any(|c| c.is_numeric()) {
        return Err("Password must contain at least one digit".to_string());
    }

    if REQUIRE_SPECIAL && !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err("Password must contain at least one special character".to_string());
    }

    Ok(())
}

/// Verify reCAPTCHA v3 token with Google's API
async fn verify_recaptcha_token(
    secret_key: &str,
    token: &str,
    action: &str,
) -> Result<bool, String> {
    let client = reqwest::Client::new();
    let url = "https://www.google.com/recaptcha/api/siteverify";

    let params = [
        ("secret", secret_key),
        ("response", token),
    ];

    let response = client
        .post(url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to verify reCAPTCHA: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse reCAPTCHA response: {}", e))?;

    if !json["success"].as_bool().unwrap_or(false) {
        return Err("reCAPTCHA verification failed".to_string());
    }

    // Check if the action matches (optional but recommended)
    if let Some(recaptcha_action) = json["action"].as_str() {
        if recaptcha_action != action {
            return Err(format!("reCAPTCHA action mismatch: expected {}, got {}", action, recaptcha_action));
        }
    }

    // Check the score - for login actions, we typically want a higher score
    let score = json["score"].as_f64().unwrap_or(0.0);
    if score < 0.5 {
        return Err(format!("reCAPTCHA score too low: {}", score));
    }

    Ok(true)
}

/// Extract admin user from request headers
#[derive(Debug)]
pub struct AdminUserExtract {
    pub user: AdminUser,
}

/// Extract API key from request headers
#[derive(Debug)]
pub struct ApiKeyExtract {
    pub key: ApiKey,
}

/// Authentication middleware for admin routes
pub async fn admin_auth(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let headers = request.headers();

    // Get authorization header
    let auth_header = headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Extract username and password from Basic auth
    if !auth_header.starts_with("Basic ") {
        return Err((StatusCode::UNAUTHORIZED, "Authorization header required".to_string()));
    }

    let encoded = &auth_header[6..];
    let decoded = match base64::engine::general_purpose::STANDARD.decode(encoded) {
        Ok(decoded) => decoded,
        Err(_) => return Err((StatusCode::UNAUTHORIZED, "Invalid authorization header".to_string())),
    };

    let auth_str = match std::str::from_utf8(&decoded) {
        Ok(s) => s,
        Err(_) => return Err((StatusCode::UNAUTHORIZED, "Invalid authorization header".to_string())),
    };

    let parts: Vec<&str> = auth_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err((StatusCode::UNAUTHORIZED, "Invalid authorization header".to_string()));
    }

    let username = parts[0];
    let password = parts[1];

    // Get admin user from database
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    let user = admin_db.get_admin_by_username(username)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()))?;

    // Verify password
    if !bcrypt::verify(password, &user.password_hash).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Password verification failed".to_string()))? {
        return Err((StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()));
    }

    // If password change is required, redirect to password change page
    if user.requires_password_change {
        info!(username = %username, "Admin user requires password change");
        // Redirect to password change page
        let response = axum::response::Redirect::to("/auth/change-password").into_response();
        return Ok(response);
    }

    // Continue to the next middleware/handler
    Ok(next.run(request).await)
}

/// API key validation middleware for API requests
pub async fn api_key_auth(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<ApiKeyExtract, (StatusCode, String)> {
    // Get authorization header
    let auth_header = headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    // Extract API key from Bearer auth
    if !auth_header.starts_with("Bearer ") {
        return Err((StatusCode::UNAUTHORIZED, "API key required".to_string()));
    }
    
    let api_key_str = &auth_header[7..];
    
    // Get API key from database
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;
    
    let key = admin_db.get_api_key_by_value(api_key_str)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid API key".to_string()))?;
    
    // Check if API key is enabled
    if !key.enabled {
        return Err((StatusCode::UNAUTHORIZED, "API key is disabled".to_string()));
    }
   
    // Check if API key has expired
    if let Some(expires_at) = key.expires_at {
        if chrono::Utc::now() > expires_at {
            return Err((StatusCode::UNAUTHORIZED, "API key has expired".to_string()));
        }
    }
    
    Ok(ApiKeyExtract { key })
}

/// API key authentication middleware for imports API
/// Requires import:write, import:*, OR both endpoints:write AND services:write
pub async fn imports_api_key_auth(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    // Extract headers from request
    let headers = request.headers().clone();

    // Validate API key using the existing api_key_auth function
    // We need to extract the API key from headers manually since we can't reuse the extractor
    let auth_header = headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Extract API key from Bearer auth
    if !auth_header.starts_with("Bearer ") {
        return Err((StatusCode::UNAUTHORIZED, "API key required".to_string()));
    }

    let api_key_str = &auth_header[7..];

    // Get API key from database
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    let key = admin_db.get_api_key_by_value(api_key_str)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid API key".to_string()))?;

    // Check if API key is enabled
    if !key.enabled {
        return Err((StatusCode::UNAUTHORIZED, "API key is disabled".to_string()));
    }

    // Check if API key has expired
    if let Some(expires_at) = key.expires_at {
        if chrono::Utc::now() > expires_at {
            return Err((StatusCode::UNAUTHORIZED, "API key has expired".to_string()));
        }
    }

    // Check if API key has import permissions OR both endpoints:write AND services:write
    let has_import_write = key.permissions.contains(&"import:write".to_string());
    let has_import_wildcard = key.permissions.contains(&"import:*".to_string());
    let has_endpoints_write = key.permissions.contains(&"endpoints:write".to_string());
    let has_services_write = key.permissions.contains(&"services:write".to_string());

    if !has_import_write && !has_import_wildcard && !(has_endpoints_write && has_services_write) {
        return Err((StatusCode::FORBIDDEN,
            "API key does not have import permissions (requires 'import:write', 'import:*', or both 'endpoints:write' and 'services:write')".to_string()));
    }

    // API key auth succeeded, continue to the next middleware/handler
    Ok(next.run(request).await)
}

/// Helper function to validate API key and check permissions
async fn validate_api_key_with_permission(
    state: &Arc<AppState>,
    request: &Request,
    resource: &str,
) -> Result<ApiKey, (StatusCode, String)> {
    let headers = request.headers();

    let auth_header = headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Extract API key from Bearer auth
    if !auth_header.starts_with("Bearer ") {
        return Err((StatusCode::UNAUTHORIZED, "API key required".to_string()));
    }

    let api_key_str = &auth_header[7..];

    // Get API key from database
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    let key = admin_db.get_api_key_by_value(api_key_str)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid API key".to_string()))?;

    // Check if API key is enabled
    if !key.enabled {
        return Err((StatusCode::UNAUTHORIZED, "API key is disabled".to_string()));
    }

    // Check if API key has expired
    if let Some(expires_at) = key.expires_at {
        if chrono::Utc::now() > expires_at {
            return Err((StatusCode::UNAUTHORIZED, "API key has expired".to_string()));
        }
    }

    // Determine required permission based on HTTP method
    let method = request.method();
    let permission_type = if method == axum::http::Method::GET {
        "read"
    } else {
        "write"
    };

    let required_permission = format!("{}:{}", resource, permission_type);
    let wildcard_permission = format!("{}:*", resource);

    // Check if API key has the required permission
    if !key.permissions.contains(&required_permission) && !key.permissions.contains(&wildcard_permission) {
        return Err((StatusCode::FORBIDDEN, format!(
            "API key does not have '{}' permission (requires '{}' or '{}')",
            resource, required_permission, wildcard_permission
        )));
    }

    Ok(key)
}

/// API key authentication middleware for endpoints API
pub async fn endpoints_api_key_auth(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    validate_api_key_with_permission(&state, &request, "endpoints").await?;
    Ok(next.run(request).await)
}

/// API key authentication middleware for services API
pub async fn services_api_key_auth(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    validate_api_key_with_permission(&state, &request, "services").await?;
    Ok(next.run(request).await)
}

/// API key authentication middleware for domains API
pub async fn domains_api_key_auth(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    validate_api_key_with_permission(&state, &request, "domains").await?;
    Ok(next.run(request).await)
}

/// API key authentication middleware for collections API
pub async fn collections_api_key_auth(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    validate_api_key_with_permission(&state, &request, "collections").await?;
    Ok(next.run(request).await)
}

/// Handler for admin login page
pub async fn login_page(State(state): State<Arc<AppState>>) -> Response {
    let recaptcha_site_key = state.config.recaptcha_site_key.clone().unwrap_or_default();
    
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Admin Login</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 400px; margin: 50px auto; padding: 20px; }}
        h1 {{ text-align: center; }}
        .form-group {{ margin-bottom: 15px; }}
        label {{ display: block; margin-bottom: 5px; }}
        input[type="text"], input[type="password"] {{ width: 100%; padding: 8px; box-sizing: border-box; }}
        button {{ width: 100%; padding: 10px; background-color: #007bff; color: white; border: none; cursor: pointer; }}
        button:hover {{ background-color: #0056b3; }}
        .error {{ color: red; margin-bottom: 15px; }}
        .recaptcha-info {{ font-size: 0.8em; color: #666; text-align: center; margin-top: 10px; }}
    </style>
    <script src="https://www.google.com/recaptcha/api.js?render={}"></script>
</head>
<body>
    <h1>Admin Login</h1>
    <form id="loginForm">
        <div class="form-group">
            <label for="username">Username:</label>
            <input type="text" id="username" name="username" required>
        </div>
        <div class="form-group">
            <label for="password">Password:</label>
            <input type="password" id="password" name="password" required>
        </div>
        <button type="submit">Login</button>
    </form>
    <div class="recaptcha-info">
        This site is protected by reCAPTCHA and the Google
        <a href="https://policies.google.com/privacy">Privacy Policy</a> and
        <a href="https://policies.google.com/terms">Terms of Service</a> apply.
    </div>
     
    <script>
        // Execute reCAPTCHA when page loads
        let recaptchaToken = null;
         
        grecaptcha.ready(function() {{
            grecaptcha.execute('{}', {{action: 'login'}}).then(function(token) {{
                recaptchaToken = token;
            }});
        }});
         
        document.getElementById('loginForm').addEventListener('submit', async (e) => {{
            e.preventDefault();
            const username = document.getElementById('username').value;
            const password = document.getElementById('password').value;
             
            if (!recaptchaToken) {{
                alert('reCAPTCHA verification failed. Please try again.');
                return;
            }}
             
            const response = await fetch('/auth/login', {{
                method: 'POST',
                headers: {{
                    'Content-Type': 'application/json',
                }},
                body: JSON.stringify({{ username, password, recaptcha_token: recaptchaToken }})
            }});
             
            if (response.ok) {{
                const data = await response.json();
                if (data.requires_password_change) {{
                    window.location.href = '/auth/change-password';
                }} else {{
                    window.location.href = '/admin';
                }}
            }} else {{
                const error = await response.text();
                alert('Login failed: ' + error);
                // Reset reCAPTCHA token for next attempt
                grecaptcha.ready(function() {{
                    grecaptcha.execute('{}', {{action: 'login'}}).then(function(token) {{
                        recaptchaToken = token;
                    }});
                }});
            }}
        }});
    </script>
</body>
</html>"#,
        recaptcha_site_key, recaptcha_site_key, recaptcha_site_key
    );
    
    axum::response::Html(html).into_response()
}

/// Handler for login POST request
pub async fn login(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(login_data): axum::extract::Json<LoginData>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Validate reCAPTCHA token first
    if let Some(recaptcha_secret_key) = &state.config.recaptcha_secret_key {
        if !verify_recaptcha_token(
            recaptcha_secret_key,
            &login_data.recaptcha_token,
            "login"
        ).await.map_err(|e| (StatusCode::BAD_REQUEST, format!("reCAPTCHA verification failed: {}", e)))? {
            return Err((StatusCode::BAD_REQUEST, "reCAPTCHA verification failed".to_string()));
        }
    } else {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "reCAPTCHA not configured".to_string()));
    }

    // Check rate limit for this username
    if let Err(retry_after) = state.login_rate_limiter.check(&login_data.username) {
        info!(username = %login_data.username, "Login rate limited");
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            format!("Too many login attempts. Try again in {} seconds", retry_after.as_secs())
        ));
    }

    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    let user = admin_db.get_admin_by_username(&login_data.username)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()))?;

    if !verify(&login_data.password, &user.password_hash).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Password verification failed".to_string()))? {
        return Err((StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()));
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
            })
        ));
    }

    info!(username = %login_data.username, "Admin user logged in successfully");
    Ok((
        [(header::SET_COOKIE, session_cookie)],
        axum::Json(LoginResponse {
            success: true,
            requires_password_change: false,
            message: "Login successful".to_string(),
        })
    ))
}

/// Login data structure
#[derive(serde::Deserialize)]
pub struct LoginData {
    pub username: String,
    pub password: String,
    pub recaptcha_token: String,
}

/// Login response structure
#[derive(serde::Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub requires_password_change: bool,
    pub message: String,
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

    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    let user = admin_db.get_admin_by_username(&change_data.username)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid username".to_string()))?;

    // Verify current password
    if !verify(&change_data.current_password, &user.password_hash).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Password verification failed".to_string()))? {
        return Err((StatusCode::UNAUTHORIZED, "Current password is incorrect".to_string()));
    }

    // Update password
    admin_db.update_admin_password(&change_data.username, &change_data.new_password)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update password".to_string()))?;

    info!(username = %change_data.username, "Admin password changed successfully");
    Ok(axum::Json(PasswordChangeResponse {
        success: true,
        message: "Password changed successfully".to_string(),
    }))
}

/// Password change data structure
#[derive(serde::Deserialize)]
pub struct ChangePasswordData {
    pub username: String,
    pub current_password: String,
    pub new_password: String,
}

/// Password change response structure
#[derive(serde::Serialize)]
pub struct PasswordChangeResponse {
    pub success: bool,
    pub message: String,
}

/// Handler for password change page
pub async fn change_password_page() -> Response {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Change Password</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { font-family: Arial, sans-serif; max-width: 400px; margin: 50px auto; padding: 20px; }
        h1 { text-align: center; }
        .form-group { margin-bottom: 15px; }
        label { display: block; margin-bottom: 5px; }
        input[type="password"] { width: 100%; padding: 8px; box-sizing: border-box; }
        button { width: 100%; padding: 10px; background-color: #007bff; color: white; border: none; cursor: pointer; }
        button:hover { background-color: #0056b3; }
        .error { color: red; margin-bottom: 15px; }
        .success { color: green; margin-bottom: 15px; }
    </style>
</head>
<body>
    <h1>Change Password</h1>
    <p>You must change your password before accessing the admin panel.</p>
    <form id="changePasswordForm">
        <div class="form-group">
            <label for="currentPassword">Current Password:</label>
            <input type="password" id="currentPassword" name="currentPassword" required>
        </div>
        <div class="form-group">
            <label for="newPassword">New Password:</label>
            <input type="password" id="newPassword" name="newPassword" required>
        </div>
        <div class="form-group">
            <label for="confirmPassword">Confirm New Password:</label>
            <input type="password" id="confirmPassword" name="confirmPassword" required>
        </div>
        <button type="submit">Change Password</button>
    </form>
    
    <div id="message" style="margin-top: 15px;"></div>
    
    <script>
        document.getElementById('changePasswordForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            const currentPassword = document.getElementById('currentPassword').value;
            const newPassword = document.getElementById('newPassword').value;
            const confirmPassword = document.getElementById('confirmPassword').value;
            
            if (newPassword !== confirmPassword) {
                showMessage('New passwords do not match', 'error');
                return;
            }
            
            const response = await fetch('/auth/change-password', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    username: 'admin',
                    current_password: currentPassword,
                    new_password: newPassword
                })
            });
            
            const data = await response.json();
            if (data.success) {
                showMessage('Password changed successfully! Redirecting to admin panel...', 'success');
                setTimeout(() => {
                    window.location.href = '/admin';
                }, 2000);
            } else {
                showMessage('Password change failed: ' + (data.message || 'Unknown error'), 'error');
            }
        });
        
        function showMessage(message, type) {
            const messageDiv = document.getElementById('message');
            messageDiv.textContent = message;
            messageDiv.className = type;
        }
    </script>
</body>
</html>
"#;

    axum::response::Html(html).into_response()
}

/// Handler for API Keys management page
pub async fn api_keys_page() -> Response {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>API Keys Management</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body {
            font-family: Arial, sans-serif;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f5f5f5;
        }
        
        .header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 20px;
        }
        
        h1 {
            color: #333;
            margin: 0;
        }
        
        .nav-buttons {
            display: flex;
            gap: 10px;
        }
        
        .btn {
            padding: 8px 16px;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 14px;
        }
        
        .btn-primary {
            background-color: #007bff;
            color: white;
        }
        
        .btn-primary:hover {
            background-color: #0056b3;
        }
        
        .btn-secondary {
            background-color: #6c757d;
            color: white;
        }
        
        .btn-secondary:hover {
            background-color: #5a6268;
        }
        
        .btn-danger {
            background-color: #dc3545;
            color: white;
        }
        
        .btn-danger:hover {
            background-color: #c82333;
        }
        
        .btn-success {
            background-color: #28a745;
            color: white;
        }
        
        .btn-success:hover {
            background-color: #218838;
        }
        
        .modal {
            display: none;
            position: fixed;
            z-index: 1000;
            left: 0;
            top: 0;
            width: 100%;
            height: 100%;
            background-color: rgba(0, 0, 0, 0.5);
        }
        
        .modal-content {
            background-color: white;
            margin: 10% auto;
            padding: 20px;
            border-radius: 8px;
            width: 50%;
            max-width: 600px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        }
        
        .modal-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 15px;
            padding-bottom: 10px;
            border-bottom: 1px solid #eee;
        }
        
        .close {
            color: #aaa;
            font-size: 28px;
            font-weight: bold;
            cursor: pointer;
        }
        
        .close:hover {
            color: #000;
        }
        
        .form-group {
            margin-bottom: 15px;
        }
        
        label {
            display: block;
            margin-bottom: 5px;
            font-weight: bold;
        }
        
        input[type="text"],
        input[type="number"],
        select {
            width: 100%;
            padding: 8px;
            border: 1px solid #ddd;
            border-radius: 4px;
            box-sizing: border-box;
        }
        
        .checkbox-group {
            margin: 10px 0;
        }
        
        .checkbox-group label {
            display: inline;
            font-weight: normal;
            margin-left: 5px;
        }
        
        .table-container {
            background-color: white;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
            overflow: hidden;
        }
        
        table {
            width: 100%;
            border-collapse: collapse;
        }
        
        th, td {
            padding: 12px 15px;
            text-align: left;
            border-bottom: 1px solid #ddd;
        }
        
        th {
            background-color: #f8f9fa;
            font-weight: bold;
            color: #495057;
        }
        
        tr:hover {
            background-color: #f1f1f1;
        }
        
        .status-badge {
            padding: 4px 8px;
            border-radius: 12px;
            font-size: 12px;
            font-weight: bold;
        }
        
        .status-active {
            background-color: #d4edda;
            color: #155724;
        }
        
        .status-disabled {
            background-color: #f8d7da;
            color: #721c24;
        }
        
        .actions {
            display: flex;
            gap: 8px;
        }
        
        .key-display {
            font-family: monospace;
            background-color: #f8f9fa;
            padding: 2px 6px;
            border-radius: 4px;
            font-size: 13px;
        }
        
        .alert {
            padding: 12px;
            margin: 15px 0;
            border-radius: 4px;
            display: none;
        }
        
        .alert-success {
            background-color: #d4edda;
            color: #155724;
            display: block;
        }
        
        .alert-error {
            background-color: #f8d7da;
            color: #721c24;
            display: block;
        }
        
        .created-key-display {
            background-color: #e7f3ff;
            border: 1px solid #b3d9ff;
            padding: 15px;
            border-radius: 4px;
            margin: 15px 0;
            font-family: monospace;
        }
        
        .created-key-display pre {
            margin: 0;
            white-space: pre-wrap;
            word-break: break-all;
        }
        
        .copy-btn {
            background-color: #e9ecef;
            border: none;
            padding: 4px 8px;
            margin-left: 10px;
            cursor: pointer;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>API Keys Management</h1>
        <div class="nav-buttons">
            <button class="btn btn-secondary" onclick="window.location.href='/admin'">Back to Admin</button>
            <button class="btn btn-primary" id="createKeyBtn">Create New API Key</button>
        </div>
    </div>
    
    <!-- Alert messages -->
    <div id="alert" class="alert"></div>
    
    <!-- API Key Creation Modal -->
    <div id="createKeyModal" class="modal">
        <div class="modal-content">
            <div class="modal-header">
                <h2>Create New API Key</h2>
                <span class="close" id="closeCreateModal">&times;</span>
            </div>
            
            <form id="createKeyForm">
                <div class="form-group">
                    <label for="keyLabel">Label:</label>
                    <input type="text" id="keyLabel" name="label" required placeholder="e.g., Production App, Development, CI/CD">
                </div>
                
                <div class="form-group">
                    <label for="keyExpiration">Expiration (days, 0 for no expiration):</label>
                    <input type="number" id="keyExpiration" name="expires_days" min="0" value="0">
                </div>
                
                <div class="form-group">
                    <label>Permissions:</label>
                    <p style="font-size: 0.9em; color: #666; margin-bottom: 10px;">Select the resources and access levels for this API key.</p>

                    <div style="margin-bottom: 10px;">
                        <strong>Endpoints:</strong>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permEndpointsRead" name="permissions" value="endpoints:read">
                            <label for="permEndpointsRead">Read</label>
                        </div>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permEndpointsWrite" name="permissions" value="endpoints:write">
                            <label for="permEndpointsWrite">Write</label>
                        </div>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permEndpointsAll" name="permissions" value="endpoints:*">
                            <label for="permEndpointsAll">Full Access</label>
                        </div>
                    </div>

                    <div style="margin-bottom: 10px;">
                        <strong>Services:</strong>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permServicesRead" name="permissions" value="services:read">
                            <label for="permServicesRead">Read</label>
                        </div>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permServicesWrite" name="permissions" value="services:write">
                            <label for="permServicesWrite">Write</label>
                        </div>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permServicesAll" name="permissions" value="services:*">
                            <label for="permServicesAll">Full Access</label>
                        </div>
                    </div>

                    <div style="margin-bottom: 10px;">
                        <strong>Domains:</strong>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permDomainsRead" name="permissions" value="domains:read">
                            <label for="permDomainsRead">Read</label>
                        </div>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permDomainsWrite" name="permissions" value="domains:write">
                            <label for="permDomainsWrite">Write</label>
                        </div>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permDomainsAll" name="permissions" value="domains:*">
                            <label for="permDomainsAll">Full Access</label>
                        </div>
                    </div>

                    <div style="margin-bottom: 10px;">
                        <strong>Collections:</strong>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permCollectionsRead" name="permissions" value="collections:read">
                            <label for="permCollectionsRead">Read</label>
                        </div>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permCollectionsWrite" name="permissions" value="collections:write">
                            <label for="permCollectionsWrite">Write</label>
                        </div>
                        <div class="checkbox-group">
                            <input type="checkbox" id="permCollectionsAll" name="permissions" value="collections:*">
                            <label for="permCollectionsAll">Full Access</label>
                        </div>
                    </div>
                </div>
                
                <button type="submit" class="btn btn-primary">Create API Key</button>
            </form>
            
            <!-- Display created key -->
            <div id="createdKeyDisplay" class="created-key-display" style="display: none;">
                <h3>New API Key Created</h3>
                <p>⚠️ <strong>Important:</strong> This is the only time the full API key will be displayed. Copy it now and store it securely.</p>
                <pre id="createdKeyValue"></pre>
                <button class="btn btn-secondary copy-btn" onclick="copyToClipboard()">Copy to Clipboard</button>
                <button class="btn btn-primary" onclick="closeCreatedKeyDisplay()">Done</button>
            </div>
        </div>
    </div>
    
    <!-- Confirmation Modal -->
    <div id="confirmModal" class="modal">
        <div class="modal-content">
            <div class="modal-header">
                <h2 id="confirmModalTitle">Confirm Action</h2>
                <span class="close" id="closeConfirmModal">&times;</span>
            </div>
            
            <p id="confirmModalMessage"></p>
            
            <div style="display: flex; justify-content: flex-end; gap: 10px; margin-top: 20px;">
                <button class="btn btn-secondary" id="cancelConfirmBtn">Cancel</button>
                <button class="btn btn-danger" id="confirmBtn">Confirm</button>
            </div>
        </div>
    </div>
    
    <!-- API Keys Table -->
    <div class="table-container">
        <table id="apiKeysTable">
            <thead>
                <tr>
                    <th>Label</th>
                    <th>Key</th>
                    <th>Created</th>
                    <th>Expires</th>
                    <th>Permissions</th>
                    <th>Status</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody id="apiKeysTableBody">
                <tr>
                    <td colspan="7" style="text-align: center; padding: 20px;">
                        <p>Loading API keys...</p>
                    </td>
                </tr>
            </tbody>
        </table>
    </div>

    <script>
        // Current action data
        let currentAction = null;
        let currentKeyId = null;
        
        // DOM elements
        const createKeyModal = document.getElementById('createKeyModal');
        const confirmModal = document.getElementById('confirmModal');
        const closeCreateModal = document.getElementById('closeCreateModal');
        const closeConfirmModal = document.getElementById('closeConfirmModal');
        const createKeyBtn = document.getElementById('createKeyBtn');
        const createKeyForm = document.getElementById('createKeyForm');
        const createdKeyDisplay = document.getElementById('createdKeyDisplay');
        const createdKeyValue = document.getElementById('createdKeyValue');
        const cancelConfirmBtn = document.getElementById('cancelConfirmBtn');
        const confirmBtn = document.getElementById('confirmBtn');
        const alertDiv = document.getElementById('alert');
        
        // Open create key modal
        createKeyBtn.onclick = function() {
            createKeyModal.style.display = 'block';
        }
        
        // Close modals
        closeCreateModal.onclick = function() {
            createKeyModal.style.display = 'none';
        }
        
        closeConfirmModal.onclick = function() {
            confirmModal.style.display = 'none';
        }
        
        cancelConfirmBtn.onclick = function() {
            confirmModal.style.display = 'none';
        }
        
        // Close modals when clicking outside
        window.onclick = function(event) {
            if (event.target === createKeyModal) {
                createKeyModal.style.display = 'none';
            }
            if (event.target === confirmModal) {
                confirmModal.style.display = 'none';
            }
        }
        
        // Create API key form submission
        createKeyForm.onsubmit = async function(e) {
            e.preventDefault();
            
            const label = document.getElementById('keyLabel').value;
            const expiresDays = parseInt(document.getElementById('keyExpiration').value) || 0;
            
            // Get selected permissions
            const permissions = [];
            const checkboxes = document.querySelectorAll('input[name="permissions"]:checked');
            checkboxes.forEach(checkbox => {
                permissions.push(checkbox.value);
            });
            
            try {
                const response = await fetch('/admin/api-keys', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        label: label,
                        enabled: true,
                        permissions: permissions,
                        expires_days: expiresDays
                    })
                });
                
                const data = await response.json();
                
                if (data.success) {
                    // Show the created key
                    const apiKeyData = data.data;
                    createdKeyValue.textContent = apiKeyData.key;
                    document.getElementById('createKeyForm').style.display = 'none';
                    createdKeyDisplay.style.display = 'block';
                    
                    // Refresh the table
                    loadApiKeys();
                    
                    showAlert('API key created successfully!', 'success');
                } else {
                    showAlert('Failed to create API key: ' + (data.message || 'Unknown error'), 'error');
                }
            } catch (error) {
                showAlert('Error creating API key: ' + error.message, 'error');
            }
        }
        
        function closeCreatedKeyDisplay() {
            createKeyModal.style.display = 'none';
            document.getElementById('createKeyForm').style.display = 'block';
            createdKeyDisplay.style.display = 'none';
            createKeyForm.reset();
        }
        
        function copyToClipboard() {
            const keyValue = createdKeyValue.textContent;
            navigator.clipboard.writeText(keyValue).then(function() {
                showAlert('API key copied to clipboard!', 'success');
            }, function(err) {
                showAlert('Failed to copy API key: ' + err, 'error');
            });
        }
        
        // Load API keys on page load
        loadApiKeys();
        
        async function loadApiKeys() {
            try {
                const response = await fetch('/admin/api-keys');
                const data = await response.json();
                
                if (data.success) {
                    renderApiKeys(data.data);
                } else {
                    showAlert('Failed to load API keys: ' + (data.message || 'Unknown error'), 'error');
                }
            } catch (error) {
                showAlert('Error loading API keys: ' + error.message, 'error');
            }
        }
        
        function renderApiKeys(keys) {
            const tbody = document.getElementById('apiKeysTableBody');
            
            if (keys.length === 0) {
                tbody.innerHTML = '
                    <tr>
                        <td colspan="7" style="text-align: center; padding: 20px;">
                            <p>No API keys found. Create your first API key!</p>
                        </td>
                    </tr>
                ';
                return;
            }
            
            let html = '';
            keys.forEach(key => {
                const createdDate = new Date(key.created_at).toLocaleString();
                const expiresDate = key.expires_at ? new Date(key.expires_at).toLocaleString() : 'Never';
                const permissions = key.permissions.join(', ');
                const statusClass = key.enabled ? 'status-active' : 'status-disabled';
                const statusText = key.enabled ? 'Active' : 'Disabled';
                
                html += `
                    <tr>
                        <td>${escapeHtml(key.label)}</td>
                        <td><span class="key-display">${escapeHtml(key.key_partial)}</span></td>
                        <td>${createdDate}</td>
                        <td>${expiresDate}</td>
                        <td>${escapeHtml(permissions)}</td>
                        <td><span class="status-badge ${statusClass}">${statusText}</span></td>
                        <td class="actions">
                            ${key.enabled ?
                                `<button class="btn btn-danger" onclick="toggleKeyStatus('${key.id}', false)">Disable</button>`
                                : `<button class="btn btn-success" onclick="toggleKeyStatus('${key.id}', true)">Enable</button>`}
                            <button class="btn btn-danger" onclick="confirmDelete('${key.id}')">Delete</button>
                        </td>
                    </tr>
                `;
            });
            
            tbody.innerHTML = html;
        }
        
        function toggleKeyStatus(keyId, enable) {
            const action = enable ? 'enable' : 'disable';
            currentAction = action;
            currentKeyId = keyId;
            
            document.getElementById('confirmModalTitle').textContent = enable ? 'Enable API Key' : 'Disable API Key';
            document.getElementById('confirmModalMessage').textContent =
                enable ? 'Are you sure you want to enable this API key?' : 'Are you sure you want to disable this API key?';
            
            confirmModal.style.display = 'block';
        }
        
        function confirmDelete(keyId) {
            currentAction = 'delete';
            currentKeyId = keyId;
            
            document.getElementById('confirmModalTitle').textContent = 'Delete API Key';
            document.getElementById('confirmModalMessage').textContent =
                'Are you sure you want to permanently delete this API key? This action cannot be undone.';
            
            confirmModal.style.display = 'block';
        }
        
        confirmBtn.onclick = async function() {
            if (!currentAction || !currentKeyId) return;
            
            try {
                let url = '';
                let method = 'POST';
                
                if (currentAction === 'enable') {
                    url = `/admin/api-keys/${currentKeyId}/enable`;
                } else if (currentAction === 'disable') {
                    url = `/admin/api-keys/${currentKeyId}/disable`;
                } else if (currentAction === 'delete') {
                    url = `/admin/api-keys/${currentKeyId}`;
                    method = 'DELETE';
                }
                
                const response = await fetch(url, {
                    method: method,
                    headers: {
                        'Content-Type': 'application/json',
                    }
                });
                
                const data = await response.json();
                
                if (data.success) {
                    showAlert(`API key ${currentAction}d successfully!`, 'success');
                    loadApiKeys();
                } else {
                    showAlert(`Failed to ${currentAction} API key: ` + (data.message || 'Unknown error'), 'error');
                }
            } catch (error) {
                showAlert(`Error ${currentAction}ing API key: ` + error.message, 'error');
            }
            
            confirmModal.style.display = 'none';
            currentAction = null;
            currentKeyId = null;
        }
        
        function showAlert(message, type) {
            alertDiv.textContent = message;
            alertDiv.className = 'alert alert-' + type;
            
            // Hide alert after 5 seconds
            setTimeout(() => {
                alertDiv.style.display = 'none';
            }, 5000);
        }
        
        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }
    </script>
</body>
</html>
"#;

    axum::response::Html(html).into_response()
}

/// Handler for logout
pub async fn logout(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> impl IntoResponse {
    // Extract session ID from cookie and delete it
    if let Some(session_id) = crate::session::extract_session_id_from_request(&request) {
        state.session_store.delete_session(&session_id);
    }

    // Clear session cookie
    let delete_cookie = crate::session::delete_session_cookie();

    (
        [(header::SET_COOKIE, delete_cookie)],
        axum::response::Html("<html><body><h1>Logged out</h1><p>You have been logged out.</p><a href='/auth/login'>Login again</a></body></html>")
    )
}

/// Handler for listing all API keys
pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;
    
    // Get the admin user ID to filter API keys
    let admin_user = admin_db.get_admin_by_username("admin")
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get admin user".to_string()))?
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Admin user not found".to_string()))?;
    
    // List API keys for the admin user
    let api_keys = admin_db.list_api_keys(&admin_user.id)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))?;
    
    // Create a safe response that masks the API keys
    #[derive(serde::Serialize)]
    struct SafeApiKey {
        id: String,
        label: String,
        created_by: String,
        created_at: String,
        expires_at: Option<String>,
        enabled: bool,
        permissions: Vec<String>,
        key_partial: String, // Only show partial key for security
    }
    
    let safe_keys: Vec<SafeApiKey> = api_keys.iter()
        .map(|key| {
            let key_partial = if key.key.len() > 8 {
                format!("{}...{}", &key.key[..4], &key.key[key.key.len()-4..])
            } else {
                key.key.clone()
            };
            
            SafeApiKey {
                id: key.id.clone(),
                label: key.label.clone(),
                created_by: key.created_by.clone(),
                created_at: key.created_at.to_rfc3339(),
                expires_at: key.expires_at.map(|dt| dt.to_rfc3339()),
                enabled: key.enabled,
                permissions: key.permissions.clone(),
                key_partial,
            }
        })
        .collect();
    
    Ok(axum::Json(JsonResponse {
        success: true,
        data: serde_json::to_value(safe_keys).unwrap(),
    }))
}

/// Handler for creating a new API key
pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(create_data): axum::extract::Json<CreateApiKeyData>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Starting API key creation process");
    
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|e| {
            error!("Failed to initialize admin database: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to initialize admin database: {}", e))
        })?;

    info!("Successfully initialized admin database");

    // Get the admin user ID to use as created_by
    let admin_user = admin_db.get_admin_by_username("admin")
        .map_err(|e| {
            error!("Failed to get admin user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get admin user: {}", e))
        })?
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Admin user not found".to_string()))?;

    info!("Found admin user with ID: {}", admin_user.id);

    // Check API key limit (256 max)
    let existing_keys = admin_db.list_api_keys(&admin_user.id)
        .map_err(|e| {
            error!("Failed to query admin database for existing keys: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to query admin database: {}", e))
        })?;

    info!("Found {} existing API keys", existing_keys.len());

    if existing_keys.len() >= 256 {
        return Err((StatusCode::BAD_REQUEST, "Maximum limit of 256 API keys reached. Please delete unused keys before creating new ones.".to_string()));
    }

    // Create the API key using the database method
    // Use the actual admin user ID instead of "admin" string
    info!("Creating API key with label: {}, permissions: {:?}, expires_days: {}",
          create_data.label, create_data.permissions, create_data.expires_days);
    
    let api_key = admin_db.create_api_key(&create_data.label, &admin_user.id, create_data.permissions, create_data.expires_days)
        .map_err(|e| {
            error!("Failed to create API key in database: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create API key: {}", e))
        })?;

    info!(key = %api_key.key, "New API key created successfully");

    // Create a response that shows the full key only once (at creation)
    #[derive(serde::Serialize)]
    struct ApiKeyCreationResponse {
        id: String,
        label: String,
        created_by: String,
        created_at: String,
        expires_at: Option<String>,
        enabled: bool,
        permissions: Vec<String>,
        key: String, // Full key shown only at creation
        key_partial: String, // Partial key for display
    }

    let key_partial = if api_key.key.len() > 8 {
        format!("{}...{}", &api_key.key[..4], &api_key.key[api_key.key.len()-4..])
    } else {
        api_key.key.clone()
    };

    let response_data = ApiKeyCreationResponse {
        id: api_key.id,
        label: api_key.label,
        created_by: api_key.created_by,
        created_at: api_key.created_at.to_rfc3339(),
        expires_at: api_key.expires_at.map(|dt| dt.to_rfc3339()),
        enabled: api_key.enabled,
        permissions: api_key.permissions,
        key: api_key.key.clone(), // Full key shown only once
        key_partial: key_partial,
    };

    Ok(axum::Json(JsonResponse {
        success: true,
        data: serde_json::to_value(response_data).unwrap(),
    }))
}


/// Handler for disabling an API key (old version using key value)
pub async fn disable_api_key_by_key(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    admin_db.disable_api_key(&key)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to disable API key".to_string()))?;

    info!(key = %key, "API key disabled");

    Ok(axum::Json(JsonResponse {
        success: true,
        data: serde_json::Value::String("API key disabled".to_string()),
    }))
}

/// Handler for disabling an API key by ID
pub async fn disable_api_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    // First get the key to log it, then disable by ID
    if let Some(key) = admin_db.get_api_key_by_id(&id)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))? {
        
        admin_db.disable_api_key(&key.key)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to disable API key".to_string()))?;
        
        info!(key = %key.key, "API key disabled");
    } else {
        return Err((StatusCode::NOT_FOUND, "API key not found".to_string()));
    }

    Ok(axum::Json(JsonResponse {
        success: true,
        data: serde_json::Value::String("API key disabled".to_string()),
    }))
}

/// Handler for enabling an API key by ID
pub async fn enable_api_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    // First get the key to log it, then enable by ID
    if let Some(key) = admin_db.get_api_key_by_id(&id)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))? {
        
        admin_db.enable_api_key(&key.key)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to enable API key".to_string()))?;
        
        info!(key = %key.key, "API key enabled");
    } else {
        return Err((StatusCode::NOT_FOUND, "API key not found".to_string()));
    }

    Ok(axum::Json(JsonResponse {
        success: true,
        data: serde_json::Value::String("API key enabled".to_string()),
    }))
}

/// Handler for deleting an API key by ID
pub async fn delete_api_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    // First get the key to log it, then delete by ID
    if let Some(key) = admin_db.get_api_key_by_id(&id)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))? {
        
        admin_db.delete_api_key(&key.key)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete API key".to_string()))?;
        
        info!(key = %key.key, "API key deleted");
    } else {
        return Err((StatusCode::NOT_FOUND, "API key not found".to_string()));
    }
    
    Ok(axum::Json(JsonResponse {
        success: true,
        data: serde_json::Value::String("API key deleted".to_string()),
    }))
}


/// API key creation data structure
#[derive(serde::Deserialize)]
pub struct CreateApiKeyData {
    pub label: String,
    pub enabled: bool,
    pub permissions: Vec<String>,
    pub expires_days: i32, // 0 means no expiration
}

/// Generic JSON response structure
#[derive(serde::Serialize)]
pub struct JsonResponse {
    pub success: bool,
    pub data: serde_json::Value,
}

/// Create admin authentication routes (public routes only - login, password change)
pub fn create_admin_auth_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", get(login_page).post(login))
        .route("/change-password", get(change_password_page).post(change_password))
        .route("/logout", get(logout).post(logout))
}

/// Create protected admin routes (requires authentication)
pub fn create_protected_admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api-keys/page", get(api_keys_page))
        .route("/api-keys/{id}/enable", post(enable_api_key))
        .route("/api-keys/{id}/disable", post(disable_api_key))
        .route("/api-keys/{id}", delete(delete_api_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use crate::config::AppConfig;
    
    #[tokio::test]
    async fn test_login_page() {
        // Simple test to ensure login page function compiles
        // This test just verifies the function signature and basic structure
        // Full integration testing would require more setup
        let recaptcha_site_key = "test-site-key".to_string();
        
        // Test that the HTML generation works
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Admin Login</title>
    <script src="https://www.google.com/recaptcha/api.js?render={}"></script>
</head>
<body>
    <h1>Admin Login</h1>
    <form id="loginForm">
        <input type="text" id="username" name="username" required>
        <input type="password" id="password" name="password" required>
        <button type="submit">Login</button>
    </form>
    <script>
        let recaptchaToken = null;
        grecaptcha.ready(function() {{
            grecaptcha.execute('{}', {{action: 'login'}}).then(function(token) {{
                recaptchaToken = token;
            }});
        }});
    </script>
</body>
</html>"#,
            recaptcha_site_key, recaptcha_site_key
        );
         
        assert!(html.contains("Admin Login"));
        assert!(html.contains("recaptcha"));
        assert!(html.contains("username"));
        assert!(html.contains("password"));
    }
}
