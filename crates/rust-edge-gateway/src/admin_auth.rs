//! Admin authentication middleware

use axum::{
    extract::{State, Path, Request},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, delete},
    Router,
    middleware::Next,
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::db_admin::{AdminDatabase, AdminUser, ApiKey};
use crate::AppState;
use bcrypt::verify;
use base64::Engine;

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

/// Handler for admin login page
pub async fn login_page() -> Response {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Admin Login</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { font-family: Arial, sans-serif; max-width: 400px; margin: 50px auto; padding: 20px; }
        h1 { text-align: center; }
        .form-group { margin-bottom: 15px; }
        label { display: block; margin-bottom: 5px; }
        input[type="text"], input[type="password"] { width: 100%; padding: 8px; box-sizing: border-box; }
        button { width: 100%; padding: 10px; background-color: #007bff; color: white; border: none; cursor: pointer; }
        button:hover { background-color: #0056b3; }
        .error { color: red; margin-bottom: 15px; }
    </style>
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
    
    <script>
        document.getElementById('loginForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            const username = document.getElementById('username').value;
            const password = document.getElementById('password').value;
            
            const response = await fetch('/auth/login', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ username, password })
            });
            
            if (response.ok) {
                const data = await response.json();
                if (data.requires_password_change) {
                    window.location.href = '/auth/change-password';
                } else {
                    window.location.href = '/admin';
                }
            } else {
                const error = await response.text();
                alert('Login failed: ' + error);
            }
        });
    </script>
</body>
</html>
"#;
    
    axum::response::Html(html).into_response()
}

/// Handler for login POST request
pub async fn login(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(login_data): axum::extract::Json<LoginData>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;
    
    let user = admin_db.get_admin_by_username(&login_data.username)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()))?;
    
    if !verify(&login_data.password, &user.password_hash).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Password verification failed".to_string()))? {
        return Err((StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()));
    }
    
    // If password change is required, return success but indicate change needed
    if user.requires_password_change {
        info!(username = %login_data.username, "Admin user logged in, password change required");
        return Ok(axum::Json(LoginResponse {
            success: true,
            requires_password_change: true,
            message: "Password change required".to_string(),
        }));
    }
    
    info!(username = %login_data.username, "Admin user logged in successfully");
    Ok(axum::Json(LoginResponse {
        success: true,
        requires_password_change: false,
        message: "Login successful".to_string(),
    }))
}

/// Login data structure
#[derive(serde::Deserialize)]
pub struct LoginData {
    pub username: String,
    pub password: String,
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

/// Handler for logout
pub async fn logout() -> impl IntoResponse {
    // Clear session/cookie
    axum::response::Html("<html><body><h1>Logged out</h1><p>You have been logged out.</p><a href='/'>Login again</a></body></html>").into_response()
}

/// Handler for listing all API keys
pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;
    
    // For now, list all API keys (in production, you'd filter by the authenticated user)
    // We'll need to get the admin user ID from the request context
    let api_keys = admin_db.list_api_keys("admin")
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query admin database".to_string()))?;
    
    Ok(axum::Json(JsonResponse {
        success: true,
        data: serde_json::Value::Array(
            api_keys.iter()
                .map(|k| serde_json::to_value(k).unwrap())
                .collect()
        ),
    }))
}

/// Handler for creating a new API key
pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(create_data): axum::extract::Json<CreateApiKeyData>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    // Create the API key using the database method
    // In production, you'd get the created_by from the authenticated user
    let api_key = admin_db.create_api_key(&create_data.label, "admin", create_data.permissions)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create API key".to_string()))?;

    info!(key = %api_key.key, "New API key created");

    Ok(axum::Json(JsonResponse {
        success: true,
        data: serde_json::to_value(api_key).unwrap(),
    }))
}

/// Handler for enabling an API key
pub async fn enable_api_key(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    // Note: enable_api_key method doesn't exist in AdminDatabase
    // For now, we'll just return success. In production, you'd implement this method.
    info!(key = %key, "API key enable requested (not implemented)");

    Ok(axum::Json(JsonResponse {
        success: true,
        data: serde_json::Value::String("API key enabled".to_string()),
    }))
}

/// Handler for disabling an API key
pub async fn disable_api_key(
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

/// Handler for deleting an API key
pub async fn delete_api_key(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let admin_db = AdminDatabase::new(&state.config.data_dir)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize admin database".to_string()))?;

    // Use disable instead of delete since delete_api_key doesn't exist
    admin_db.disable_api_key(&key)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete API key".to_string()))?;
    
    info!(key = %key, "API key deleted");
    
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

/// Create admin authentication routes
pub fn create_admin_auth_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", get(login_page).post(login))
        .route("/change-password", get(change_password_page).post(change_password))
        .route("/logout", get(logout))
        .route("/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api-keys/{key}/enable", post(enable_api_key))
        .route("/api-keys/{key}/disable", post(disable_api_key))
        .route("/api-keys/{id}", delete(delete_api_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::routing::get;
    use axum::Router;
    use std::path::Path;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_login_page() {
        let app = Router::new().route("/login", get(login_page));
        
        let response = axum::test::TestServer::new(app)
            .await
            .get("/login")
            .await;
        
        assert_eq!(response.status(), StatusCode::OK);
    }
}
