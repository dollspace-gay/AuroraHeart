#!/bin/bash
# Before Tool Call Hook Example
# This hook runs before each tool is executed

# Access environment variables provided by the hook system
TOOL_NAME="${TOOL_NAME:-}"
TOOL_ID="${TOOL_ID:-}"
TOOL_INPUT="${TOOL_INPUT:-}"

# Log tool call
echo "About to execute tool: $TOOL_NAME (ID: $TOOL_ID)"

# Example: Prevent dangerous operations in production
if [ "$TOOL_NAME" == "bash" ] && [[ "$TOOL_INPUT" == *"rm -rf"* ]]; then
    echo "WARNING: Dangerous bash command detected!"
    # Hook can inject warnings but cannot block execution
fi

# Example: Add context based on tool type
if [ "$TOOL_NAME" == "write" ]; then
    echo "TIP: Remember to follow the project's coding style guidelines"
fi

# Exit with 0 to indicate success
exit 0
