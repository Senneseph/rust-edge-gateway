#!/bin/bash

# Comprehensive Security Test Script for Rust Edge Gateway
# Tests all authentication and authorization security features

set -e

echo "🔒 Rust Edge Gateway - Comprehensive Security Test"
echo "=================================================="
echo ""

# Configuration
ADMIN_URL="http://localhost:8081"
GATEWAY_URL="http://localhost:8080"
DEFAULT_PASSWORD="passworD123!"
NEW_PASSWORD="NewSecureP@ssw0rd123"
WEAK_PASSWORD="weak"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to test HTTP response
test_http() {
    local description="$1"
    local expected_code="$2"
    local url="$3"
    local method="${4:-GET}"
    local data="${5:-}"
    local headers="${6:-}"
    
    echo -n "Testing: $description... "
    
    if [ -n "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$url" \
            -H "Content-Type: application/json" \
            ${headers:+-H "$headers"} \
            -d "$data")
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$url" \
            ${headers:+-H "$headers"})
    fi
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" = "$expected_code" ]; then
        echo -e "${GREEN}✓ PASS${NC} (HTTP $http_code)"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC} (Expected HTTP $expected_code, got $http_code)"
        echo "  Response: $body"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

echo "📋 Test 1: Admin Authentication"
echo "--------------------------------"

# Test 1.1: Login page should be accessible
test_http "Login page accessible" 200 "$ADMIN_URL/auth/login"

# Test 1.2: Login with correct credentials
test_http "Login with valid credentials" 200 "$ADMIN_URL/auth/login" POST \
    "{\"username\":\"admin\",\"password\":\"$DEFAULT_PASSWORD\"}"

# Test 1.3: Login with incorrect password
test_http "Login with invalid password" 401 "$ADMIN_URL/auth/login" POST \
    "{\"username\":\"admin\",\"password\":\"wrongpassword\"}"

# Test 1.4: Login with non-existent user
test_http "Login with non-existent user" 401 "$ADMIN_URL/auth/login" POST \
    "{\"username\":\"hacker\",\"password\":\"password\"}"

echo ""
echo "📋 Test 2: Password Validation"
echo "--------------------------------"

# Test 2.1: Change password with weak password (should fail)
test_http "Reject weak password" 400 "$ADMIN_URL/auth/change-password" POST \
    "{\"username\":\"admin\",\"current_password\":\"$DEFAULT_PASSWORD\",\"new_password\":\"$WEAK_PASSWORD\"}"

# Test 2.2: Change password with strong password (should succeed)
test_http "Accept strong password" 200 "$ADMIN_URL/auth/change-password" POST \
    "{\"username\":\"admin\",\"current_password\":\"$DEFAULT_PASSWORD\",\"new_password\":\"$NEW_PASSWORD\"}"

echo ""
echo "📋 Test 3: API Key Management Protection"
echo "----------------------------------------"

# Test 3.1: Try to access API keys without authentication (should fail)
test_http "API keys list without auth" 401 "$ADMIN_URL/admin/api-keys"

# Test 3.2: Try to create API key without authentication (should fail)
test_http "Create API key without auth" 401 "$ADMIN_URL/admin/api-keys" POST \
    "{\"label\":\"test-key\",\"permissions\":[\"read\"]}"

echo ""
echo "📋 Test 4: Rate Limiting"
echo "------------------------"

echo "Testing login rate limiting (5 attempts in 15 minutes)..."
for i in {1..6}; do
    if [ $i -le 5 ]; then
        test_http "Login attempt $i/6" 401 "$ADMIN_URL/auth/login" POST \
            "{\"username\":\"testuser\",\"password\":\"wrongpass\"}"
    else
        test_http "Login attempt $i/6 (rate limited)" 429 "$ADMIN_URL/auth/login" POST \
            "{\"username\":\"testuser\",\"password\":\"wrongpass\"}"
    fi
done

echo ""
echo "📋 Test 5: Gateway API Key Requirement"
echo "---------------------------------------"

# Test 5.1: Health check should be public
test_http "Health check without API key" 200 "$GATEWAY_URL/health"

# Test 5.2: Other endpoints should require API key
test_http "Gateway endpoint without API key" 401 "$GATEWAY_URL/api/test"

# Test 5.3: Invalid API key should be rejected
test_http "Gateway with invalid API key" 401 "$GATEWAY_URL/api/test" GET "" \
    "Authorization: Bearer invalid-key-12345"

echo ""
echo "=================================================="
echo "📊 Test Results Summary"
echo "=================================================="
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All security tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some security tests failed!${NC}"
    exit 1
fi

