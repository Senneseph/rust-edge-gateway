#!/bin/bash
# Pet Store Demo Setup Script
#
# This script sets up the Pet Store demo with your choice of storage backend.
# Usage: ./setup.sh <backend>
#
# Backends:
#   sqlite   - SQLite database (default, no external dependencies)
#   postgres - PostgreSQL database
#   mysql    - MySQL database
#   minio    - MinIO object storage (stores pets as JSON files)
#   ftp      - FTP/SFTP file storage (stores pets as JSON files)

set -e

BACKEND="${1:-sqlite}"
API_URL="${API_URL:-http://localhost:9081}"

echo "=============================================="
echo "  Pet Store Demo Setup"
echo "  Backend: $BACKEND"
echo "=============================================="

# 1. Create the domain
echo ""
echo "1. Creating domain 'petstore.example.com'..."
DOMAIN_RESPONSE=$(curl -s -X POST "$API_URL/api/domains" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "petstore.example.com",
    "description": "Pet Store Demo Domain"
  }')
DOMAIN_ID=$(echo $DOMAIN_RESPONSE | jq -r '.data.id // empty')
if [ -z "$DOMAIN_ID" ]; then
  echo "   Domain may already exist, fetching..."
  DOMAIN_ID=$(curl -s "$API_URL/api/domains" | jq -r '.data[] | select(.name=="petstore.example.com") | .id')
fi
echo "   Domain ID: $DOMAIN_ID"

# 2. Create the collection
echo ""
echo "2. Creating collection 'Pet Store API'..."
COLLECTION_RESPONSE=$(curl -s -X POST "$API_URL/api/collections" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"Pet Store API\",
    \"domain_id\": \"$DOMAIN_ID\",
    \"description\": \"Pet Store Demo - $BACKEND backend\"
  }")
COLLECTION_ID=$(echo $COLLECTION_RESPONSE | jq -r '.data.id // empty')
if [ -z "$COLLECTION_ID" ]; then
  echo "   Collection may already exist, fetching..."
  COLLECTION_ID=$(curl -s "$API_URL/api/collections" | jq -r '.data[] | select(.name=="Pet Store API") | .id')
fi
echo "   Collection ID: $COLLECTION_ID"

# 3. Create the service based on backend choice
echo ""
echo "3. Creating $BACKEND service..."
case $BACKEND in
  sqlite)
    SERVICE_CONFIG='{
      "name": "petstore",
      "service_type": "sqlite",
      "config": {
        "path": "/data/petstore.db",
        "create_if_missing": true
      }
    }'
    ;;
  postgres)
    SERVICE_CONFIG='{
      "name": "petstore",
      "service_type": "postgres",
      "config": {
        "host": "postgres",
        "port": 5432,
        "database": "petstore",
        "username": "petstore",
        "password": "petstore"
      }
    }'
    ;;
  mysql)
    SERVICE_CONFIG='{
      "name": "petstore",
      "service_type": "mysql",
      "config": {
        "host": "mysql",
        "port": 3306,
        "database": "petstore",
        "username": "petstore",
        "password": "petstore"
      }
    }'
    ;;
  minio)
    SERVICE_CONFIG='{
      "name": "petstore",
      "service_type": "minio",
      "config": {
        "endpoint": "minio:9000",
        "access_key": "minioadmin",
        "secret_key": "minioadmin",
        "bucket": "petstore",
        "use_ssl": false
      }
    }'
    ;;
  ftp)
    SERVICE_CONFIG='{
      "name": "petstore",
      "service_type": "ftp",
      "config": {
        "host": "ftp",
        "port": 21,
        "username": "petstore",
        "password": "petstore",
        "protocol": "ftp",
        "base_path": "/data/pets"
      }
    }'
    ;;
  *)
    echo "Unknown backend: $BACKEND"
    exit 1
    ;;
esac

SERVICE_RESPONSE=$(curl -s -X POST "$API_URL/api/services" \
  -H "Content-Type: application/json" \
  -d "$SERVICE_CONFIG")
SERVICE_ID=$(echo $SERVICE_RESPONSE | jq -r '.data.id // empty')
echo "   Service ID: $SERVICE_ID"

# 4. Create endpoints
echo ""
echo "4. Creating Pet Store endpoints..."

# List pets (GET /pets)
echo "   Creating GET /pets..."
curl -s -X POST "$API_URL/api/endpoints" \
  -H "Content-Type: application/json" \
  -d "{
    \"collection_id\": \"$COLLECTION_ID\",
    \"name\": \"List Pets\",
    \"path\": \"/pets\",
    \"method\": \"GET\",
    \"handler_code\": $(cat handlers/list_pets.rs | jq -Rs .)
  }" > /dev/null

# Create pet (POST /pets)
echo "   Creating POST /pets..."
curl -s -X POST "$API_URL/api/endpoints" \
  -H "Content-Type: application/json" \
  -d "{
    \"collection_id\": \"$COLLECTION_ID\",
    \"name\": \"Create Pet\",
    \"path\": \"/pets\",
    \"method\": \"POST\",
    \"handler_code\": $(cat handlers/create_pet.rs | jq -Rs .)
  }" > /dev/null

# Get pet (GET /pets/{petId})
echo "   Creating GET /pets/{petId}..."
curl -s -X POST "$API_URL/api/endpoints" \
  -H "Content-Type: application/json" \
  -d "{
    \"collection_id\": \"$COLLECTION_ID\",
    \"name\": \"Get Pet\",
    \"path\": \"/pets/{petId}\",
    \"method\": \"GET\",
    \"handler_code\": $(cat handlers/get_pet.rs | jq -Rs .)
  }" > /dev/null

# Update pet (PUT /pets/{petId})
echo "   Creating PUT /pets/{petId}..."
curl -s -X POST "$API_URL/api/endpoints" \
  -H "Content-Type: application/json" \
  -d "{
    \"collection_id\": \"$COLLECTION_ID\",
    \"name\": \"Update Pet\",
    \"path\": \"/pets/{petId}\",
    \"method\": \"PUT\",
    \"handler_code\": $(cat handlers/update_pet.rs | jq -Rs .)
  }" > /dev/null

# Delete pet (DELETE /pets/{petId})
echo "   Creating DELETE /pets/{petId}..."
curl -s -X POST "$API_URL/api/endpoints" \
  -H "Content-Type: application/json" \
  -d "{
    \"collection_id\": \"$COLLECTION_ID\",
    \"name\": \"Delete Pet\",
    \"path\": \"/pets/{petId}\",
    \"method\": \"DELETE\",
    \"handler_code\": $(cat handlers/delete_pet.rs | jq -Rs .)
  }" > /dev/null

echo ""
echo "5. Compiling and starting endpoints..."
# Get all endpoint IDs and compile/start them
ENDPOINTS=$(curl -s "$API_URL/api/endpoints" | jq -r '.data[] | select(.collection_id=="'"$COLLECTION_ID"'") | .id')
for EP_ID in $ENDPOINTS; do
  echo "   Compiling $EP_ID..."
  curl -s -X POST "$API_URL/api/endpoints/$EP_ID/compile" > /dev/null
  echo "   Starting $EP_ID..."
  curl -s -X POST "$API_URL/api/endpoints/$EP_ID/start" > /dev/null
done

echo ""
echo "=============================================="
echo "  Setup Complete!"
echo "=============================================="
echo ""
echo "Your Pet Store API is ready at: petstore.example.com"
echo "Backend: $BACKEND"
echo ""
echo "Try it out:"
echo "  curl http://petstore.example.com/pets"
echo "  curl -X POST http://petstore.example.com/pets -d '{\"name\":\"Buddy\"}'"

