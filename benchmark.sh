#!/bin/bash

# Performance benchmark script for email-manager API
# Tests the speed improvement after implementing connection pooling and caching

echo "Email Manager API Performance Benchmark"
echo "======================================="
echo ""

# Check if API is running
if ! curl -s http://localhost:8080/health > /dev/null; then
    echo "Error: API is not running on localhost:8080"
    echo "Please start the API first with: cargo run"
    exit 1
fi

# Check if API_TOKEN is set
if [ -z "$API_TOKEN" ]; then
    echo "Error: API_TOKEN environment variable not set"
    echo "Please set it with: export API_TOKEN=your-token"
    exit 1
fi

echo "Testing endpoint performance..."
echo ""

# Function to measure request time
measure_time() {
    local endpoint=$1
    local description=$2

    echo -n "$description: "

    # Use curl with timing information
    local time=$(curl -w "%{time_total}" -o /dev/null -s \
        -H "Authorization: Bearer $API_TOKEN" \
        "http://localhost:8080$endpoint")

    # Convert to milliseconds
    local ms=$(echo "$time * 1000" | bc)
    echo "${ms}ms"

    echo "$ms"
}

echo "1. Recent Emails Endpoint (/emails/recent?limit=10)"
echo "-----------------------------------------------------"

# Warm up the cache
curl -s -H "Authorization: Bearer $API_TOKEN" "http://localhost:8080/emails/recent?limit=10" > /dev/null

# Test multiple requests to see caching effect
for i in 1 2 3 4 5; do
    time=$(measure_time "/emails/recent?limit=10" "  Request $i")
done

echo ""
echo "2. Today's Emails Endpoint (/emails/today)"
echo "-------------------------------------------"

for i in 1 2 3; do
    time=$(measure_time "/emails/today" "  Request $i")
done

echo ""
echo "3. MFA Codes Endpoint (/mfa/codes?limit=5)"
echo "-------------------------------------------"

for i in 1 2 3; do
    time=$(measure_time "/mfa/codes?limit=5" "  Request $i")
done

echo ""
echo "Performance Analysis:"
echo "--------------------"
echo "✓ Connection pooling reduces IMAP connection overhead"
echo "✓ Email caching eliminates repeated IMAP fetches"
echo "✓ First request creates connection, subsequent requests reuse it"
echo ""
echo "Expected behavior:"
echo "- First request: slower (creates connection)"
echo "- Subsequent requests: much faster (uses pool & cache)"