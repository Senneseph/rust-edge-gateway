# Rust Edge Gateway Deployment Instructions

## Prerequisites

- Rust toolchain installed (`rustup`)
- Docker installed and running
- SSH access to DigitalOcean Droplet
- `.env` file with correct credentials

## Deployment Steps

### 1. Build the Production Binary

```bash
# Build the Rust application in release mode
 cargo build --release --manifest-path crates/rust-edge-gateway/Cargo.toml
```

### 2. Prepare Environment Variables

Ensure your `.env` file contains the required environment variables, including:

```
DEFAULT_ADMIN_PASSWORD=your_secure_password_here
DIGITALOCEAN_ACCESS_TOKEN=your_token
DEPLOY_SERVER_IP=your_server_ip
TARGET_DOMAIN=your_domain.com
DOCS_DOMAIN=docs.your_domain.com
SSH_KEY=$env:USERPROFILE\.ssh\your_key
```

The `DEFAULT_ADMIN_PASSWORD` will be used to create the initial admin user on first startup and will require a password change on first login.

### 2. Build the Production Docker Image

```bash
# Build the Docker image using the production Dockerfile
 docker build -t rust-edge-gateway:prod -f Dockerfile.prod .
```

### 3. Copy Files to DigitalOcean Droplet

```bash
# Copy the built binary and docker-compose file to the server
scp -i "$HOME/.ssh/a-icon-deploy" target/release/rust-edge-gateway root@167.71.191.234:/opt/rust-edge-gateway/
scp -i "$HOME/.ssh/a-icon-deploy" docker-compose.prod.yml root@167.71.191.234:/opt/rust-edge-gateway/
scp -i "$HOME/.ssh/a-icon-deploy" .env root@167.71.191.234:/opt/rust-edge-gateway/
```

### 4. Connect to the Droplet and Deploy

```bash
# Connect to the DigitalOcean Droplet
ssh -i "$HOME/.ssh/a-icon-deploy" root@167.71.191.234
```

Once connected to the server, run the following commands:

```bash
# Navigate to the deployment directory
cd /opt/rust-edge-gateway

# Stop the current service (if running)
docker-compose -f docker-compose.prod.yml down

# Pull the latest image (if needed)
docker pull rust-edge-gateway:prod

# Start the service with the production configuration
docker-compose -f docker-compose.prod.yml up -d

# Check the service status
docker-compose -f docker-compose.prod.yml logs -f
```

### 5. Verify Deployment

Visit the application at: https://rust-edge-gateway.iffuso.com/

Check that:
- The application is responding
- All services are running
- The correct version is deployed
- Login with username `admin` and the password from `DEFAULT_ADMIN_PASSWORD`
- You are prompted to change your password on first login (security requirement)

## Troubleshooting

If the deployment fails:

1. Check Docker logs: `docker-compose -f docker-compose.prod.yml logs`
2. Verify the binary has execute permissions: `chmod +x /opt/rust-edge-gateway/rust-edge-gateway`
3. Ensure the .env file has the correct environment variables
4. Verify network connectivity to required services (database, storage, etc.)

## Rollback Procedure

To rollback to a previous version:

1. Stop the current service: `docker-compose -f docker-compose.prod.yml down`
2. Restore the previous docker-compose.prod.yml and .env files from backup
3. Restart the service: `docker-compose -f docker-compose.prod.yml up -d`

> **Note**: The SSH key path in the instructions assumes the key is stored at `$HOME/.ssh/a-icon-deploy`. If your key is stored elsewhere, update the paths accordingly.