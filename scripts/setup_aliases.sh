#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}🔧 Setting up cargo aliases...${NC}\n"

# Create cargo config directory if it doesn't exist
mkdir -p ~/.cargo

# Create or update cargo config file
CONFIG_FILE=~/.cargo/config.toml

# Add aliases
cat > "$CONFIG_FILE" << 'EOL'
[alias]
# CI and testing commands - using direct script execution
ci = "run --quiet -p lion_cli -- ci"
test-cli = "run --quiet -p lion_cli -- test-cli"

# Utility aliases for common tasks
format-all = "fmt --all"
check-all = "check --workspace --exclude lion_cli"
build-all = "build --workspace --exclude lion_cli"
test-all = "test --workspace --exclude lion_cli"
doc-all = "doc --workspace --exclude lion_cli --no-deps"

# Note: The following commands are placeholders and will be implemented
# when lion_cli is fully functional
# demo = "run -p lion_cli -- demo --data test-message --correlation-id 123e4567-e89b-12d3-a456-426614174000"
# plugin = "run -p lion_cli -- load-plugin --manifest examples/hello_plugin/manifest.toml"
# agent = "run -p lion_cli -- spawn-agent --prompt test-prompt --correlation-id 123e4567-e89b-12d3-a456-426614174000"
EOL

echo -e "\n${GREEN}✨ Setup complete! You can now use:${NC}"
echo "  cargo ci        - Run all CI checks"
echo "  cargo test-cli  - Run CLI tests"
echo "  cargo format-all - Format all code"
echo "  cargo check-all - Run cargo check across all workspaces (except lion_cli)"
echo "  cargo build-all - Build all workspaces (except lion_cli)"
echo "  cargo test-all  - Run tests across all workspaces (except lion_cli)"
echo "  cargo doc-all   - Generate documentation for all workspaces (except lion_cli)"
echo -e "\n${RED}Note: Some commands like 'demo', 'plugin', and 'agent' will be available when lion_cli is implemented${NC}"

echo -e "\n${GREEN}Aliases have been added to ${CONFIG_FILE}${NC}"
