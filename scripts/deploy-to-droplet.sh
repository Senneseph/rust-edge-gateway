#!/bin/bash
set -e

# Rust Edge Gateway - Deploy to DigitalOcean Droplet
# Usage: ./scripts/deploy-to-droplet.sh [version]
# Example: ./scripts/deploy-to-droplet.sh v1.0.0

VERSION="${1:-latest}"

# Load .env file
if [ -f .env ]; then
    export $(grep -v '^#' .env | grep -v '^$' | xargs)
fi

# Validate required variables
if [ -z "$DEPLOY_SERVER_IP" ]; then
    echo "Error: DEPLOY_SERVER_IP not set in .env"
    exit 1
fi

if [ -z "$SSH_KEY" ]; then
    echo "Error: SSH_KEY not set in .env"
    exit 1
fi

if [ -z "$DOCKER_HUB_USERNAME" ]; then
    echo "Error: DOCKER_HUB_USERNAME not set in .env"
    exit 1
fi

# Convert Windows-style SSH_KEY path if needed
SSH_KEY_PATH="${SSH_KEY//\$env:USERPROFILE/$HOME}"
SSH_KEY_PATH="${SSH_KEY_PATH//\\//}"

echo "=== Deploying Rust Edge Gateway to DigitalOcean ==="
echo "Server: $DEPLOY_SERVER_IP"
echo "Version: $VERSION"
echo "Docker Image: $DOCKER_HUB_USERNAME/rust-edge-gateway:$VERSION"
echo ""

# Step 1: Copy .env file to droplet
echo "Step 1: Copying .env file to droplet..."
scp -i "$SSH_KEY_PATH" .env root@$DEPLOY_SERVER_IP:/opt/rust-edge-gateway/.env

# Step 2: Copy docker-compose.prod.yml to droplet
echo ""
echo "Step 2: Copying docker-compose.prod.yml to droplet..."
scp -i "$SSH_KEY_PATH" docker-compose.prod.yml root@$DEPLOY_SERVER_IP:/opt/rust-edge-gateway/docker-compose.yml

# Step 3: Deploy on the droplet
echo ""
echo "Step 3: Deploying on droplet..."
ssh -i "$SSH_KEY_PATH" root@$DEPLOY_SERVER_IP << ENDSSH
    cd /opt/rust-edge-gateway
    
    # Stop existing containers
    echo "Stopping existing containers..."
    docker-compose down || true
    
    # Pull latest image from Docker Hub
    echo "Pulling image from Docker Hub..."
    docker pull $DOCKER_HUB_USERNAME/rust-edge-gateway:$VERSION

    # Start containers
    echo "Starting containers..."
    docker-compose up -d
    
    # Wait for services to start
    echo "Waiting for services to start..."
    sleep 10
    
    # Show status
    echo ""
    echo "Container status:"
    docker-compose ps
    
    echo ""
    echo "Recent logs:"
    docker-compose logs --tail 30
    
    # Health check
    echo ""
    echo "Health checks:"
    curl -s -o /dev/null -w "Admin UI (8081): HTTP %{http_code}\n" http://localhost:8081/auth/login || echo "Admin UI: Not responding"
    curl -s -o /dev/null -w "Gateway (8080): HTTP %{http_code}\n" http://localhost:8080/health || echo "Gateway: Not responding"
ENDSSH

echo ""
echo "=== Deployment Complete ==="
echo ""
echo "Access your application:"
echo "  Admin UI: https://$TARGET_DOMAIN/admin/"
echo "  Gateway:  https://$TARGET_DOMAIN/"
echo ""
echo "Login credentials:"
echo "  Username: admin"
echo "  Password: (from .env DEFAULT_ADMIN_PASSWORD)"
echo ""
echo "⚠️  Remember to change your password after first login!"

