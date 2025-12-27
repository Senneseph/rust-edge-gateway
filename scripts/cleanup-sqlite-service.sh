#!/bin/bash
# Cleanup live-sqlite service from DigitalOcean droplet
# This removes the unnecessary SQLite HTTP service since Rust Edge Gateway has embedded SQLite

set -e

echo "=== Cleaning up live-sqlite service ==="
echo ""

# Check if live-sqlite container exists
if docker ps -a --format '{{.Names}}' | grep -q '^live-sqlite$'; then
    echo "Step 1: Stopping and removing live-sqlite container..."
    docker stop live-sqlite || true
    docker rm live-sqlite || true
    echo "✓ Container removed"
else
    echo "Step 1: live-sqlite container not found (already removed)"
fi

echo ""

# Check if services_sqlite-data volume exists
if docker volume ls --format '{{.Name}}' | grep -q '^services_sqlite-data$'; then
    echo "Step 2: Removing services_sqlite-data volume..."
    docker volume rm services_sqlite-data || true
    echo "✓ Volume removed"
else
    echo "Step 2: services_sqlite-data volume not found (already removed)"
fi

echo ""

# Clean up any dangling volumes
echo "Step 3: Cleaning up dangling volumes..."
docker volume prune -f
echo "✓ Dangling volumes cleaned"

echo ""

# Clean up any dangling images
echo "Step 4: Cleaning up dangling images..."
docker image prune -f
echo "✓ Dangling images cleaned"

echo ""

# Show current docker resources
echo "=== Current Docker Resources ==="
echo ""
echo "Containers:"
docker ps -a --format "table {{.Names}}\t{{.Image}}\t{{.Status}}"
echo ""
echo "Volumes:"
docker volume ls
echo ""
echo "Disk usage:"
docker system df

echo ""
echo "=== Cleanup Complete ==="
echo ""
echo "The live-sqlite service has been removed."
echo "Rust Edge Gateway uses embedded SQLite (rusqlite) for its internal database."
echo "For handlers that need databases, use PostgreSQL or MySQL services instead."

