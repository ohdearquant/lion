#!/bin/bash
set -e # Exit on any error

# Move to the project root
cd "$(dirname "$0")/.."

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 Testing Lion CLI...${NC}\n"

# Function to run a command and check its exit status
run_test() {
    local test_name=$1
    local command=$2
    echo -e "Running test: ${test_name}"
    echo -e "Command: ${command}\n"
    if eval "$command"; then
        echo -e "${GREEN}✓ Test passed: ${test_name}${NC}\n"
        # Add a small delay between tests
        sleep 1
    else
        echo -e "${RED}✗ Test failed: ${test_name}${NC}\n"
        exit 1
    fi
}

# Function to run a command and capture its output
run_command() {
    local command=$1
    local output
    output=$(eval "$command" 2>&1)
    echo "$output"
    return ${PIPESTATUS[0]}
}

# Build the CLI
echo "Building CLI..."
cargo build
echo -e "${GREEN}✓ Build successful${NC}\n"

# Run CLI-specific tests
cargo test -p lion_cli

# Generate a test UUID for correlation
TEST_UUID="123e4567-e89b-12d3-a456-426614174000"

# Test 1: Submit a basic task
run_test "Basic Task Submission" "cargo run --bin lion_cli -- demo --data 'Hello, World!' --correlation-id $TEST_UUID"

# Test 2: Load and invoke the calculator plugin
echo "Loading calculator plugin..."
output=$(run_command "cargo run --bin lion_cli -- load-plugin --manifest plugins/calculator/manifest.toml")
PLUGIN_ID=$(echo "$output" | grep "Plugin ID:" | cut -d' ' -f3)
if [ -z "$PLUGIN_ID" ]; then
    echo -e "${RED}Failed to get plugin ID${NC}"
    echo "Output was: $output"
    exit 1
fi
echo -e "${GREEN}✓ Calculator plugin loaded with ID: $PLUGIN_ID${NC}\n"
sleep 1

# Test 3: Invoke the calculator plugin to add numbers
echo "Invoking plugin..."
# Use printf to properly escape the JSON string
output=$(printf 'cargo run --bin lion_cli -- invoke-plugin --plugin-id %s --input '\''{"function":"add","args":{"a":5,"b":3}}'\'' --correlation-id %s' "$PLUGIN_ID" "$TEST_UUID" | sh)
if [ $? -eq 0 ]; then
    echo "$output"
    echo -e "${GREEN}✓ Plugin invocation successful${NC}\n"
else
    echo -e "${RED}✗ Plugin invocation failed${NC}"
    echo "Output was: $output"
    exit 1
fi
sleep 1

# Test 4: Spawn an agent with streaming output
run_test "Agent Spawning" "cargo run --bin lion_cli -- spawn-agent --prompt 'Process this text with streaming output' --correlation-id $TEST_UUID"

# Test 5: Multiple agents with different prompts
echo "Testing multiple concurrent agents..."
for prompt in "First task" "Second task" "Third task"; do
    run_test "Concurrent Agent - $prompt" "cargo run --bin lion_cli -- spawn-agent --prompt '$prompt' --correlation-id $TEST_UUID"
done


echo -e "${GREEN}🎉 All CLI tests completed successfully!${NC}"
