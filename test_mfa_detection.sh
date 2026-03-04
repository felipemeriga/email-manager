#!/bin/bash

echo "Testing MFA Detection"
echo "===================="
echo ""

# Test 1: Get latest MFA code (should find your 234123)
echo "Test 1: Getting latest MFA code..."
curl -s -H "Authorization: Bearer testtoken" "http://localhost:8080/mfa/latest?minutes=10" | jq '.' || echo "No code found"

echo ""
echo "Test 2: Getting all MFA codes from last 10 minutes..."
curl -s -H "Authorization: Bearer testtoken" "http://localhost:8080/mfa/codes?minutes=10&limit=5" | jq '.'

echo ""
echo "Test 3: Force refresh and get recent emails to debug..."
curl -s -H "Authorization: Bearer testtoken" "http://localhost:8080/emails/recent?limit=3&fresh=true" | jq '.emails[] | {subject, sender_email, date}'

echo ""
echo "Debugging tips:"
echo "- Check if 'MFA Code' appears in recent emails"
echo "- Check if date is within the time window"
echo "- Run with RUST_LOG=debug for detailed logs"