#!/bin/bash
set -e # Exit on any error

# Move to the project root
cd "$(dirname "$0")/.."

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}ℹ️ Lion CLI is not fully implemented yet. Running structure validation tests instead.${NC}\n"

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

echo -e "${GREEN}🚀 Testing project structure...${NC}\n"

# Test 1: Verify project structure
echo "Checking project directory structure..."
directories=(
    "lion/lion_core"
    "lion/lion_capability"
    "lion/lion_concurrency"
    "lion/lion_isolation"
    "lion/lion_observability"
    "lion/lion_policy"
    "lion/lion_runtime"
    "lion/lion_workflow"
    "lion/lion_cli"
)

for dir in "${directories[@]}"; do
    if [ -d "$dir" ]; then
        echo -e "${GREEN}✓ Directory exists: $dir${NC}"
    else
        echo -e "${RED}✗ Missing directory: $dir${NC}"
        exit 1
    fi
done

# Test 2: Check if all Cargo.toml files exist and are valid
echo -e "\nChecking Cargo.toml files..."
for dir in "${directories[@]}"; do
    if [ -f "$dir/Cargo.toml" ]; then
        # Validate the Cargo.toml syntax
        if cargo verify-project --manifest-path="$dir/Cargo.toml" > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Valid Cargo.toml in $dir${NC}"
        else
            echo -e "${RED}✗ Invalid Cargo.toml in $dir${NC}"
            exit 1
        fi
    else
        echo -e "${RED}✗ Missing Cargo.toml in $dir${NC}"
        exit 1
    fi
done

# Test 3: Run cargo check on the workspace excluding lion_cli
echo -e "\nRunning cargo check on workspace (excluding lion_cli)..."
run_test "Cargo Check" "cargo check --workspace --exclude lion_cli"

echo -e "\n${YELLOW}Note: Full CLI testing will be implemented when lion_cli is complete${NC}"
echo -e "${YELLOW}The current tests verify that the project structure is correct${NC}"

echo -e "\n${GREEN}🎉 All structure tests completed successfully!${NC}"
