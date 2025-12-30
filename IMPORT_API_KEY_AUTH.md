# Import API Key Authentication Implementation

## Overview

This document describes the implementation of API key authentication for bundle and OpenAPI import endpoints. Previously, these endpoints were only accessible via session authentication (for the Admin UI). Now, external tools and CI/CD pipelines can use API key authentication to import bundles programmatically.

## Problem Statement

The bundle import endpoint (`/api/admin/import/bundle`) required session authentication, which only worked for browser-based Admin UI access. External tools and CI/CD pipelines needed to import bundles using API key authentication, but there was no API-key-protected import endpoint available.

## Solution

Added new API-key-protected import routes at `/api/import/*` that mirror the session-protected routes at `/api/admin/import/*`.

### Route Structure

| Route | Auth | Use Case |
|-------|------|----------|
| `/api/import/bundle` | API Key | External tools, CI/CD pipelines |
| `/api/import/openapi` | API Key | External tools, CI/CD pipelines |
| `/api/admin/import/bundle` | Session | Admin UI (browser-based) |
| `/api/admin/import/openapi` | Session | Admin UI (browser-based) |

## Implementation Details

### 1. New Middleware: `imports_api_key_auth`

**File:** `crates/rust-edge-gateway/src/admin_auth.rs`

Created a new middleware function that validates API keys with special import permissions:

```rust
pub async fn imports_api_key_auth(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)>
```

**Permission Requirements:**

The middleware accepts API keys with any of the following permission combinations:
- `import:write` - Dedicated import write permission
- `import:*` - Wildcard import permission
- **Both** `endpoints:write` AND `services:write` - Since importing creates both endpoints and services

This flexible permission model allows:
- Dedicated import-only API keys
- Reusing existing API keys that have both endpoint and service write permissions
- Future expansion with `import:read` for read-only import operations

### 2. New Router: `imports_api`

**File:** `crates/rust-edge-gateway/src/main.rs`

Created a new router for import operations:

```rust
let imports_api = Router::new()
    .route("/openapi", post(api::import_openapi))
    .route("/bundle", post(api::import_bundle))
    .layer(axum::middleware::from_fn_with_state(state.clone(), imports_api_key_auth));
```

Nested in the admin router at `/api/import`:

```rust
let admin_router = Router::new()
    .nest("/api/endpoints", endpoints_api)
    .nest("/api/services", services_api)
    .nest("/api/domains", domains_api)
    .nest("/api/collections", collections_api)
    .nest("/api/import", imports_api)  // NEW: API key auth
    .nest("/api/admin", admin_api)     // Session auth (includes /import/*)
    // ...
```

### 3. Admin UI Updates

**File:** `static/admin/app.js`

Added new permissions to the available permissions list:

```javascript
availablePermissions: [
    // ... existing permissions ...
    'import:read',
    'import:write'
]
```

These permissions now appear as checkboxes when creating API keys in the Admin UI.

## Usage Examples

### 1. Create an API Key with Import Permissions

Via Admin UI:
1. Navigate to API Keys section
2. Click "Generate API Key"
3. Check the `import:write` permission
4. Generate and copy the key

Via API (requires session auth):
```bash
curl -X POST "http://localhost:8081/admin/api-keys" \
  -H "Content-Type: application/json" \
  -H "Cookie: session_id=YOUR_SESSION" \
  -d '{
    "label": "CI/CD Import Key",
    "enabled": true,
    "permissions": ["import:write"],
    "expires_days": 90
  }'
```

### 2. Import OpenAPI Spec with API Key

```bash
curl -X POST "http://localhost:8081/api/import/openapi" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "spec": "openapi: 3.0.0\ninfo:\n  title: My API\n...",
    "domain": "api.example.com",
    "create_collection": true,
    "domain_id": "domain-uuid"
  }'
```

### 3. Import Bundle with API Key

```bash
curl -X POST "http://localhost:8081/api/import/bundle?domain=api.example.com&compile=true&start=true" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -F "bundle=@my-api-bundle.zip"
```

### 4. Production Example (from prompt)

```bash
curl -X POST "https://rust-edge-gateway.iffuso.com/api/import/bundle?domain=a-icon.com&compile=true&start=true" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -F "bundle=@bundle.zip"
```

## Security Considerations

1. **Permission Granularity**: Import operations require explicit permissions, preventing unauthorized API creation
2. **Backward Compatibility**: Existing session-based routes remain unchanged for Admin UI
3. **Flexible Permissions**: Supports dedicated import permissions or combined endpoint+service permissions
4. **API Key Best Practices**: 
   - Use short-lived keys for CI/CD (set expiration)
   - Rotate keys regularly
   - Use minimum required permissions
   - Store keys securely (environment variables, secrets managers)

## Testing

A test script is provided at `test-import-api.sh`:

```bash
# Set your API key
export API_KEY="your-api-key-here"

# Run tests
./test-import-api.sh
```

The script tests:
- ✓ Import with valid API key
- ✓ Rejection without API key (401)
- ✓ Rejection with invalid API key (401)

## Files Modified

1. `crates/rust-edge-gateway/src/admin_auth.rs` - Added `imports_api_key_auth` middleware
2. `crates/rust-edge-gateway/src/main.rs` - Added `imports_api` router and nested it
3. `static/admin/app.js` - Added `import:read` and `import:write` permissions

## Migration Notes

- **No breaking changes**: Existing session-based import routes continue to work
- **New functionality**: API key-based import is now available alongside session-based import
- **Permission setup**: Administrators need to grant `import:write` permission to API keys that need import access

