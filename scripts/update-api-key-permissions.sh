#!/bin/bash
# Script to update API key permissions in the database
# Run this inside the Docker container or on the server

set -e

# Load .env file if it exists
if [ -f /app/.env ]; then
    export $(grep -v '^#' /app/.env | xargs)
elif [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

# Check required environment variables
if [ -z "$RUST_EDGE_GATEWAY_API_KEY" ]; then
    echo "ERROR: RUST_EDGE_GATEWAY_API_KEY not set"
    exit 1
fi

# Find the admin database
ADMIN_DB_PATH="${RUST_EDGE_GATEWAY_DATA_DIR:-/app/data}/admin.db"

if [ ! -f "$ADMIN_DB_PATH" ]; then
    echo "ERROR: Admin database not found at $ADMIN_DB_PATH"
    exit 1
fi

echo "Updating API key permissions..."

# The permissions we need for full API access
PERMISSIONS='["domains:*","collections:*","endpoints:*","services:*","import:*"]'

# Update the API key permissions using SQLite
sqlite3 "$ADMIN_DB_PATH" "UPDATE api_keys SET permissions = '$PERMISSIONS' WHERE key = '$RUST_EDGE_GATEWAY_API_KEY';"

if [ $? -eq 0 ]; then
    echo "API key permissions updated successfully!"
    echo "Permissions: domains:*, collections:*, endpoints:*, services:*, import:*"
else
    echo "ERROR: Failed to update API key permissions"
    exit 1
fi

# Verify the update
echo ""
echo "Verifying update..."
sqlite3 "$ADMIN_DB_PATH" "SELECT key, label, permissions FROM api_keys WHERE key = '$RUST_EDGE_GATEWAY_API_KEY';"

echo ""
echo "You can now run the deployment script."

