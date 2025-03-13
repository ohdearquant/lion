#!/bin/bash
# Lion CLI Demo Script
# This script demonstrates common usage patterns for the Lion CLI

# Set colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Lion CLI Demo Script${NC}"
echo -e "This script will demonstrate key features of the Lion CLI\n"

# 1. Start the Lion microkernel
echo -e "${CYAN}Step 1: Starting the Lion microkernel...${NC}"
lion-cli system start
echo

# 2. Show initial system status
echo -e "${CYAN}Step 2: Checking initial system status...${NC}"
lion-cli system status
echo

# 3. Load calculator plugin
echo -e "${CYAN}Step 3: Loading calculator plugin...${NC}"
PLUGIN_ID=$(lion-cli plugin load --path ../plugins/calculator/calculator_plugin.wasm)
PLUGIN_ID=$(echo "$PLUGIN_ID" | grep -o "ID: [^ ]*" | cut -d' ' -f2)
echo -e "Loaded plugin with ID: ${GREEN}$PLUGIN_ID${NC}"
echo

# 4. List loaded plugins
echo -e "${CYAN}Step 4: Listing loaded plugins...${NC}"
lion-cli plugin list
echo

# 5. Grant file capability to the plugin
echo -e "${CYAN}Step 5: Granting file capability to the plugin...${NC}"
lion-cli plugin grant-cap --plugin "$PLUGIN_ID" --cap-type file --params '{"path":"/tmp/results.txt","read":true,"write":true,"execute":false}'
echo

# 6. Add a policy rule
echo -e "${CYAN}Step 6: Adding a policy rule...${NC}"
lion-cli policy add --rule-id demo-rule-1 --subject "plugin:$PLUGIN_ID" --object "network:example.com:80" --action allow
echo

# 7. List policy rules
echo -e "${CYAN}Step 7: Listing policy rules...${NC}"
lion-cli policy list
echo

# 8. Call a function in the plugin
echo -e "${CYAN}Step 8: Calling a function in the plugin...${NC}"
lion-cli plugin call "$PLUGIN_ID" calculate --args '{"x": 42, "y": 8, "operation": "add"}'
echo

# 9. Register a workflow
echo -e "${CYAN}Step 9: Registering a workflow...${NC}"
WORKFLOW_ID=$(lion-cli workflow register --file ../examples/hello_plugin/plugins/data/workflow.json)
WORKFLOW_ID=$(echo "$WORKFLOW_ID" | grep -o "ID: [^ ]*" | cut -d' ' -f2)
echo -e "Registered workflow with ID: ${GREEN}$WORKFLOW_ID${NC}"
echo

# 10. Start the workflow
echo -e "${CYAN}Step 10: Starting the workflow...${NC}"
lion-cli workflow start "$WORKFLOW_ID"
echo

# 11. Check workflow status
echo -e "${CYAN}Step 11: Checking workflow status...${NC}"
lion-cli workflow status "$WORKFLOW_ID"
echo

# 12. View system logs
echo -e "${CYAN}Step 12: Viewing system logs...${NC}"
lion-cli system logs --level INFO --component workflow
echo

# 13. Shutdown the microkernel
echo -e "${CYAN}Step 13: Shutting down the microkernel...${NC}"
lion-cli system shutdown
echo

echo -e "${GREEN}Demo completed successfully!${NC}"
echo -e "This script demonstrated the core functionality of the Lion CLI."
echo -e "For more information, see the documentation with: lion-cli --help"