# Rust Edge Gateway - Build and Publish to Docker Hub
# Usage: .\scripts\build-and-publish.ps1 [version]
# Example: .\scripts\build-and-publish.ps1 v1.0.0

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
if (-not $env:DOCKER_HUB_USERNAME) {
    Write-Error "DOCKER_HUB_USERNAME not set in .env"
    exit 1
}

Write-Host "=== Building and Publishing Rust Edge Gateway ===" -ForegroundColor Green
Write-Host "Docker Hub User: $env:DOCKER_HUB_USERNAME"
Write-Host "Version: $Version"
Write-Host ""

# Step 1: Build the Docker image
Write-Host "Step 1: Building Docker image..." -ForegroundColor Cyan
docker build -f Dockerfile.prod -t rust-edge-gateway:$Version .

# Step 2: Tag for Docker Hub
Write-Host ""
Write-Host "Step 2: Tagging image for Docker Hub..." -ForegroundColor Cyan
docker tag rust-edge-gateway:$Version "$env:DOCKER_HUB_USERNAME/rust-edge-gateway:$Version"

# Also tag as latest if this is a version tag
if ($Version -ne "latest") {
    docker tag rust-edge-gateway:$Version "$env:DOCKER_HUB_USERNAME/rust-edge-gateway:latest"
}

# Step 3: Login to Docker Hub
Write-Host ""
Write-Host "Step 3: Logging in to Docker Hub..." -ForegroundColor Cyan
if ($env:DOCKER_HUB_TOKEN) {
    $env:DOCKER_HUB_TOKEN | docker login -u $env:DOCKER_HUB_USERNAME --password-stdin
} else {
    Write-Host "No DOCKER_HUB_PERSONAL_ACCESS_TOKEN found, attempting interactive login..."
    docker login -u $env:DOCKER_HUB_USERNAME
}

# Step 4: Push to Docker Hub
Write-Host ""
Write-Host "Step 4: Pushing to Docker Hub..." -ForegroundColor Cyan
docker push "$env:DOCKER_HUB_USERNAME/rust-edge-gateway:$Version"

if ($Version -ne "latest") {
    docker push "$env:DOCKER_HUB_USERNAME/rust-edge-gateway:latest"
}

Write-Host ""
Write-Host "=== Build and Publish Complete ===" -ForegroundColor Green
Write-Host "Image published: $env:DOCKER_HUB_USERNAME/rust-edge-gateway:$Version"
if ($Version -ne "latest") {
    Write-Host "Also tagged as: $env:DOCKER_HUB_USERNAME/rust-edge-gateway:latest"
}
Write-Host ""
Write-Host "Next steps:"
Write-Host "1. Run: .\scripts\deploy-to-droplet.ps1"
Write-Host "2. Or manually deploy on the droplet"

