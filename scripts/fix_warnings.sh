#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🔧 Fixing code warnings...${NC}\n"

# Move to the project root
cd "$(dirname "$0")/.."

# Run cargo fix on each crate to automatically fix simple warnings
echo -e "${YELLOW}Running cargo fix on lion_workflow...${NC}"
cargo fix --allow-dirty --lib -p lion_workflow

echo -e "${YELLOW}Running cargo fix on lion_runtime...${NC}"
cargo fix --allow-dirty --lib -p lion_runtime

echo -e "${YELLOW}Running cargo fix on lion_cli...${NC}"
cargo fix --allow-dirty --lib -p lion_cli

echo -e "${GREEN}✅ Automatic fixes applied${NC}\n"

echo -e "${YELLOW}Remaining warnings may need manual fixes.${NC}"
echo -e "${YELLOW}Check the output of 'cargo check --workspace' for any remaining warnings.${NC}\n"

echo -e "${GREEN}🎉 Fix script completed!${NC}"