#!/bin/bash
set -e

# Rust Edge Gateway - Build and Publish to Docker Hub
# Usage: ./scripts/build-and-publish.sh [version]
# Example: ./scripts/build-and-publish.sh v1.0.0

VERSION="${1:-latest}"

# Load .env file
if [ -f .env ]; then
    export $(grep -v '^#' .env | grep -v '^$' | xargs)
fi

# Validate required variables
if [ -z "$DOCKER_HUB_USERNAME" ]; then
    echo "Error: DOCKER_HUB_USERNAME not set in .env"
    exit 1
fi

echo "=== Building and Publishing Rust Edge Gateway ==="
echo "Docker Hub User: $DOCKER_HUB_USERNAME"
echo "Version: $VERSION"
echo ""

# Step 1: Build the Docker image
echo "Step 1: Building Docker image..."
docker build -f Dockerfile.prod -t rust-edge-gateway:$VERSION .

# Step 2: Tag for Docker Hub
echo ""
echo "Step 2: Tagging image for Docker Hub..."
docker tag rust-edge-gateway:$VERSION $DOCKER_HUB_USERNAME/rust-edge-gateway:$VERSION

# Also tag as latest if this is a version tag
if [ "$VERSION" != "latest" ]; then
    docker tag rust-edge-gateway:$VERSION $DOCKER_HUB_USERNAME/rust-edge-gateway:latest
fi

# Step 3: Login to Docker Hub (if token is set)
echo ""
echo "Step 3: Logging in to Docker Hub..."
if [ -n "$DOCKER_HUB_TOKEN" ]; then
    echo "$DOCKER_HUB_TOKEN" | docker login -u "$DOCKER_HUB_USERNAME" --password-stdin
else
    echo "No DOCKER_HUB_TOKEN found, attempting interactive login..."
    docker login -u "$DOCKER_HUB_USERNAME"
fi

# Step 4: Push to Docker Hub
echo ""
echo "Step 4: Pushing to Docker Hub..."
docker push $DOCKER_HUB_USERNAME/rust-edge-gateway:$VERSION

if [ "$VERSION" != "latest" ]; then
    docker push $DOCKER_HUB_USERNAME/rust-edge-gateway:latest
fi

echo ""
echo "=== Build and Publish Complete ==="
echo "Image published: $DOCKER_HUB_USERNAME/rust-edge-gateway:$VERSION"
if [ "$VERSION" != "latest" ]; then
    echo "Also tagged as: $DOCKER_HUB_USERNAME/rust-edge-gateway:latest"
fi
echo ""
echo "Next steps:"
echo "1. Run: ./scripts/deploy-to-droplet.sh"
echo "2. Or manually deploy on the droplet"

