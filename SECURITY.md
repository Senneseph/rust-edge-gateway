# Security Features - Rust Edge Gateway

## Overview

This document describes the security features implemented in the Rust Edge Gateway to protect against unauthorized access and common attack vectors.

## Authentication & Authorization

### Admin Authentication

#### Initial Setup
1. **Default Admin Account**: On first startup, if `DEFAULT_ADMIN_PASSWORD` environment variable is set, an admin account is created with username `admin`
2. **Forced Password Change**: The initial admin account requires a password change on first login
3. **Password Storage**: All passwords are hashed using bcrypt with default cost factor (currently 12)

#### Password Requirements
All passwords must meet the following criteria:
- **Minimum Length**: 12 characters
- **Uppercase**: At least one uppercase letter (A-Z)
- **Lowercase**: At least one lowercase letter (a-z)
- **Digit**: At least one numeric digit (0-9)
- **Special Character**: At least one non-alphanumeric character

Example valid password: `MySecureP@ssw0rd123`

#### Login Process
1. User submits username and password to `/auth/login`
2. System checks rate limit (5 attempts per 15 minutes per username)
3. Password is verified against bcrypt hash
4. If `requires_password_change` flag is set, user is redirected to change password
5. On successful login, rate limit counter is reset

### API Key Authentication

#### Gateway Endpoints
All gateway endpoints (port 8080) require a valid API key except:
- `/health` - Health check endpoint (public)

#### API Key Format
- **Type**: Bearer token
- **Header**: `Authorization: Bearer <api-key>`
- **Storage**: SQLite database with bcrypt hashing
- **Features**:
  - Enable/disable functionality
  - Expiration dates
  - Permission scopes
  - Created by tracking

#### API Key Management
API key management routes are protected and require admin authentication:
- `GET /admin/api-keys` - List all API keys
- `POST /admin/api-keys` - Create new API key
- `POST /admin/api-keys/{key}/enable` - Enable an API key
- `POST /admin/api-keys/{key}/disable` - Disable an API key
- `DELETE /admin/api-keys/{id}` - Delete an API key

## Rate Limiting

### Login Rate Limiting
- **Limit**: 5 failed attempts per username
- **Window**: 15 minutes
- **Response**: HTTP 429 (Too Many Requests)
- **Reset**: Successful login resets the counter

### API Key Rate Limiting
- **Limit**: 100 requests per API key
- **Window**: 1 minute
- **Response**: HTTP 429 (Too Many Requests)
- **Purpose**: Prevent brute force attacks and API abuse

### Implementation
Rate limiting is implemented using an in-memory sliding window algorithm with DashMap for thread-safe concurrent access.

## Security Headers

The following security headers are configured in Caddyfile:
- `Strict-Transport-Security`: Enforces HTTPS
- `X-Content-Type-Options`: Prevents MIME sniffing
- `X-Frame-Options`: Prevents clickjacking
- `Content-Security-Policy`: Restricts resource loading

## Database Security

### SQLite Security
- **Location**: `./data/admin.db` (configurable via `RUST_EDGE_GATEWAY_DATA_DIR`)
- **Permissions**: File system permissions should restrict access to the application user only
- **Parameterized Queries**: All database queries use parameterized statements to prevent SQL injection

### Schema
```sql
-- Admin users table
CREATE TABLE admin_users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    requires_password_change INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- API keys table
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    permissions TEXT,
    FOREIGN KEY (created_by) REFERENCES admin_users(id)
);
```

## Deployment Security

### Environment Variables
Required environment variables for production:
```bash
DEFAULT_ADMIN_PASSWORD=<strong-password>  # Initial admin password
RUST_EDGE_GATEWAY_DATA_DIR=/app/data     # Data directory
```

### HTTPS/TLS
- Caddy automatically provisions TLS certificates via Let's Encrypt
- All HTTP traffic is redirected to HTTPS
- TLS configuration in `Caddyfile`

### Network Isolation
- Admin UI (port 8081): Should be restricted to internal network or VPN
- Gateway (port 8080): Public-facing with API key authentication
- Caddy (port 443): Public HTTPS endpoint

## Security Best Practices

### For Administrators
1. **Change Default Password**: Immediately change the default admin password on first login
2. **Use Strong Passwords**: Follow password requirements strictly
3. **Rotate API Keys**: Regularly rotate API keys and disable unused keys
4. **Monitor Logs**: Review authentication logs for suspicious activity
5. **Limit Admin Access**: Restrict admin UI access to trusted networks

### For Developers
1. **Never Commit Secrets**: Keep `.env` file out of version control
2. **Use Environment Variables**: Store sensitive configuration in environment variables
3. **Validate Input**: All user input is validated before processing
4. **Audit Dependencies**: Regularly update dependencies for security patches

## Security Testing

Run the comprehensive security test suite:
```bash
chmod +x test_security_comprehensive.sh
./test_security_comprehensive.sh
```

Tests include:
- Login authentication
- Password validation
- API key protection
- Rate limiting
- Unauthorized access prevention

## Incident Response

If you discover a security vulnerability:
1. **Do Not** create a public GitHub issue
2. Contact the maintainers privately
3. Provide detailed information about the vulnerability
4. Allow time for a patch to be developed before public disclosure

## Security Audit Log

| Date | Version | Change | Auditor |
|------|---------|--------|---------|
| 2025-12-23 | 0.2.0 | Initial security implementation | AI Assistant |
| 2025-12-23 | 0.2.0 | Added password validation | AI Assistant |
| 2025-12-23 | 0.2.0 | Added rate limiting | AI Assistant |
| 2025-12-23 | 0.2.0 | Fixed API key route protection | AI Assistant |

