#!/bin/bash

# Test script for API key-protected import endpoints
# This script tests the new /api/import/* endpoints with API key authentication

set -e

# Configuration
BASE_URL="${BASE_URL:-http://localhost:8081}"
API_KEY="${API_KEY:-}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=========================================="
echo "Testing Import API with API Key Auth"
echo "=========================================="
echo ""

# Check if API key is provided
if [ -z "$API_KEY" ]; then
    echo -e "${RED}ERROR: API_KEY environment variable is not set${NC}"
    echo "Usage: API_KEY=your-api-key ./test-import-api.sh"
    exit 1
fi

echo -e "${YELLOW}Using API Key:${NC} ${API_KEY:0:8}...${API_KEY: -8}"
echo ""

# Test 1: Import OpenAPI spec with API key
echo -e "${YELLOW}Test 1: Import OpenAPI spec via /api/import/openapi${NC}"
OPENAPI_SPEC='{
  "openapi": "3.0.0",
  "info": {
    "title": "Test API",
    "version": "1.0.0"
  },
  "paths": {
    "/test": {
      "get": {
        "operationId": "getTest",
        "summary": "Test endpoint",
        "responses": {
          "200": {
            "description": "Success"
          }
        }
      }
    }
  }
}'

RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/import/openapi" \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"spec\": $(echo "$OPENAPI_SPEC" | jq -Rs .),
    \"domain\": \"test-api.local\",
    \"create_collection\": true,
    \"domain_id\": \"test-domain-id\"
  }")

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | head -n-1)

if [ "$HTTP_CODE" = "200" ]; then
    echo -e "${GREEN}✓ OpenAPI import successful (HTTP $HTTP_CODE)${NC}"
    echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"
else
    echo -e "${RED}✗ OpenAPI import failed (HTTP $HTTP_CODE)${NC}"
    echo "$BODY"
fi
echo ""

# Test 2: Try to import without API key (should fail)
echo -e "${YELLOW}Test 2: Import without API key (should fail with 401)${NC}"
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/import/openapi" \
  -H "Content-Type: application/json" \
  -d "{
    \"spec\": $(echo "$OPENAPI_SPEC" | jq -Rs .),
    \"domain\": \"test-api.local\"
  }")

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | head -n-1)

if [ "$HTTP_CODE" = "401" ]; then
    echo -e "${GREEN}✓ Correctly rejected without API key (HTTP $HTTP_CODE)${NC}"
else
    echo -e "${RED}✗ Expected 401, got HTTP $HTTP_CODE${NC}"
    echo "$BODY"
fi
echo ""

# Test 3: Try to import with invalid API key (should fail)
echo -e "${YELLOW}Test 3: Import with invalid API key (should fail with 401)${NC}"
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/import/openapi" \
  -H "Authorization: Bearer invalid-key-12345" \
  -H "Content-Type: application/json" \
  -d "{
    \"spec\": $(echo "$OPENAPI_SPEC" | jq -Rs .),
    \"domain\": \"test-api.local\"
  }")

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | head -n-1)

if [ "$HTTP_CODE" = "401" ]; then
    echo -e "${GREEN}✓ Correctly rejected invalid API key (HTTP $HTTP_CODE)${NC}"
else
    echo -e "${RED}✗ Expected 401, got HTTP $HTTP_CODE${NC}"
    echo "$BODY"
fi
echo ""

echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo "All tests completed. Check results above."
echo ""
echo "Note: To test bundle import, create a test bundle.zip and run:"
echo "  curl -X POST \"$BASE_URL/api/import/bundle?domain=test.local&compile=true&start=true\" \\"
echo "    -H \"Authorization: Bearer \$API_KEY\" \\"
echo "    -F \"bundle=@bundle.zip\""

