#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo "🚀 Running CI checks..."

# Clean and build
echo -e "\n${GREEN}Running cargo clean and build...${NC}"
cargo clean
cargo build
echo "✅ Build successful"

# Format check
echo -e "\n${GREEN}Checking code formatting...${NC}"
cargo fmt --all -- --check
echo "✅ Formatting check passed"

# Clippy
echo -e "\n${GREEN}Running clippy...${NC}"
cargo clippy --all-targets -- -D warnings
echo "✅ Clippy check passed"

# Tests
echo -e "\n${GREEN}Running tests...${NC}"
cargo test --all-targets
echo "✅ All tests passed"

# Doc tests
echo -e "\n${GREEN}Running doc tests...${NC}"
cargo test --doc
echo "✅ Doc tests passed"

# Documentation check
echo -e "\n${GREEN}Checking documentation...${NC}"
cargo doc --no-deps --document-private-items
echo "✅ Documentation check passed"

echo -e "\n${GREEN}All CI checks passed! 🎉${NC}"