//! Session management for admin authentication

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::AppState;

/// Session data
#[derive(Debug, Clone)]
pub struct Session {
    pub username: String,
    pub created_at: u64,
    pub expires_at: u64,
}

/// Session store
pub struct SessionStore {
    sessions: Arc<DashMap<String, Session>>,
    session_duration: Duration,
}

impl SessionStore {
    pub fn new(session_duration: Duration) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            session_duration,
        }
    }

    /// Create a new session
    pub fn create_session(&self, username: &str) -> String {
        let session_id = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let session = Session {
            username: username.to_string(),
            created_at: now,
            expires_at: now + self.session_duration.as_secs(),
        };

        self.sessions.insert(session_id.clone(), session);
        session_id
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        let session = self.sessions.get(session_id)?;
        
        // Check if session has expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now > session.expires_at {
            drop(session);
            self.sessions.remove(session_id);
            return None;
        }

        Some(session.clone())
    }

    /// Delete a session
    pub fn delete_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    /// Clean up expired sessions
    pub fn cleanup_expired(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.sessions.retain(|_, session| now <= session.expires_at);
    }
}

/// Extract session ID from cookie
fn extract_session_id(request: &Request) -> Option<String> {
    let cookie_header = request.headers().get(header::COOKIE)?;
    let cookie_str = cookie_header.to_str().ok()?;

    for cookie in cookie_str.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix("session_id=") {
            return Some(value.to_string());
        }
    }

    None
}

/// Extract session ID from request (public version for use in handlers)
pub fn extract_session_id_from_request(request: &Request) -> Option<String> {
    extract_session_id(request)
}

/// Middleware to check for valid session
/// Redirects to /auth/login if no valid session is found
pub async fn session_auth(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    let method = request.method();
    tracing::info!("Session auth middleware called for {} {}", method, path);
    
    // Extract session ID from cookie
    let session_id = match extract_session_id(&request) {
        Some(id) => id,
        None => {
            // No session cookie - redirect to login
            tracing::warn!("No session cookie found for {} {}, redirecting to login", method, path);
            return redirect_to_login();
        }
    };

    // Validate session
    match state.session_store.get_session(&session_id) {
        Some(_session) => {
            // Session is valid, continue to the next middleware/handler
            next.run(request).await
        }
        None => {
            // Invalid or expired session - redirect to login
            redirect_to_login()
        }
    }
}

/// Create a redirect response to the login page
fn redirect_to_login() -> Response {
    use axum::http::header;

    Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, "/auth/login")
        .body(axum::body::Body::empty())
        .unwrap()
}

/// Create a session cookie header value
pub fn create_session_cookie(session_id: &str, max_age: u64) -> String {
    format!(
        "session_id={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}",
        session_id, max_age
    )
}

/// Create a cookie to delete the session
pub fn delete_session_cookie() -> String {
    "session_id=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0".to_string()
}
