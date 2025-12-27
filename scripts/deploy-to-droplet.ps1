# Rust Edge Gateway - Deploy to DigitalOcean Droplet
# Usage: .\scripts\deploy-to-droplet.ps1 [version]
# Example: .\scripts\deploy-to-droplet.ps1 v1.0.0

param(
    [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"

# Load .env file
if (Test-Path .env) {
    Get-Content .env | ForEach-Object {
        if ($_ -match '^([^#][^=]+)=(.*)$') {
            $name = $matches[1].Trim()
            $value = $matches[2].Trim()
            Set-Item -Path "env:$name" -Value $value
        }
    }
}

# Validate required variables
if (-not $env:DEPLOY_SERVER_IP) {
    Write-Error "DEPLOY_SERVER_IP not set in .env"
    exit 1
}

if (-not $env:SSH_KEY) {
    Write-Error "SSH_KEY not set in .env"
    exit 1
}

if (-not $env:DOCKER_HUB_USERNAME) {
    Write-Error "DOCKER_HUB_USERNAME not set in .env"
    exit 1
}

# Expand SSH_KEY path
$SSH_KEY_PATH = $ExecutionContext.InvokeCommand.ExpandString($env:SSH_KEY)

Write-Host "=== Deploying Rust Edge Gateway to DigitalOcean ===" -ForegroundColor Green
Write-Host "Server: $env:DEPLOY_SERVER_IP"
Write-Host "Version: $Version"
Write-Host "Docker Image: $env:DOCKER_HUB_USERNAME/rust-edge-gateway:$Version"
Write-Host ""

# Step 1: Copy .env file to droplet
Write-Host "Step 1: Copying .env file to droplet..." -ForegroundColor Cyan
scp -i $SSH_KEY_PATH .env root@$($env:DEPLOY_SERVER_IP):/opt/rust-edge-gateway/.env

# Step 2: Copy docker-compose.prod.yml to droplet
Write-Host ""
Write-Host "Step 2: Copying docker-compose.prod.yml to droplet..." -ForegroundColor Cyan
scp -i $SSH_KEY_PATH docker-compose.prod.yml root@$($env:DEPLOY_SERVER_IP):/opt/rust-edge-gateway/docker-compose.yml

# Step 3: Deploy on the droplet
Write-Host ""
Write-Host "Step 3: Deploying on droplet..." -ForegroundColor Cyan

$deployCommands = @"
cd /opt/rust-edge-gateway

# Stop existing containers
echo 'Stopping existing containers...'
docker-compose down || true

# Pull latest image from Docker Hub
echo 'Pulling image from Docker Hub...'
docker pull $env:DOCKER_HUB_USERNAME/rust-edge-gateway:$Version

# Start containers
echo 'Starting containers...'
docker-compose up -d

# Wait for services to start
echo 'Waiting for services to start...'
sleep 10

# Show status
echo ''
echo 'Container status:'
docker-compose ps

echo ''
echo 'Recent logs:'
docker-compose logs --tail 30

# Health check
echo ''
echo 'Health checks:'
curl -s -o /dev/null -w 'Admin UI (8081): HTTP %{http_code}\n' http://localhost:8081/auth/login || echo 'Admin UI: Not responding'
curl -s -o /dev/null -w 'Gateway (8080): HTTP %{http_code}\n' http://localhost:8080/health || echo 'Gateway: Not responding'
"@

ssh -i $SSH_KEY_PATH root@$env:DEPLOY_SERVER_IP $deployCommands

Write-Host ""
Write-Host "=== Deployment Complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "Access your application:"
Write-Host "  Admin UI: https://$env:TARGET_DOMAIN/admin/"
Write-Host "  Gateway:  https://$env:TARGET_DOMAIN/"
Write-Host ""
Write-Host "Login credentials:"
Write-Host "  Username: admin"
Write-Host "  Password: (from .env DEFAULT_ADMIN_PASSWORD)"
Write-Host ""
Write-Host "⚠️  Remember to change your password after first login!" -ForegroundColor Yellow

