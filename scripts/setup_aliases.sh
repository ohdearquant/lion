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
# Build and check commands
ci = "run --quiet -p agentic_cli -- ci"
test-cli = "run --quiet -p agentic_cli -- test-cli"

# Demo commands
demo = "run -p agentic_cli -- demo --data test-message --correlation-id 123e4567-e89b-12d3-a456-426614174000"
plugin = "run -p agentic_cli -- load-plugin --manifest examples/hello_plugin/manifest.toml"
agent = "run -p agentic_cli -- spawn-agent --prompt test-prompt --correlation-id 123e4567-e89b-12d3-a456-426614174000"
EOL

echo -e "\n${GREEN}✨ Setup complete! You can now use:${NC}"
echo "  cargo ci        - Run all CI checks"
echo "  cargo test-cli  - Run CLI tests"
echo "  cargo demo      - Run a demo task"
echo "  cargo plugin    - Load and run the hello plugin"
echo "  cargo agent     - Spawn an agent"

echo -e "\n${GREEN}Aliases have been added to ${CONFIG_FILE}${NC}"
