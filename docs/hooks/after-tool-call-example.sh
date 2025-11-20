#!/bin/bash
# After Tool Call Hook Example
# This hook runs after each tool execution completes

# Access environment variables provided by the hook system
TOOL_NAME="${TOOL_NAME:-}"
TOOL_ID="${TOOL_ID:-}"
TOOL_INPUT="${TOOL_INPUT:-}"
TOOL_OUTPUT="${TOOL_OUTPUT:-}"
IS_ERROR="${IS_ERROR:-false}"

# Log tool result
if [ "$IS_ERROR" == "true" ]; then
    echo "Tool $TOOL_NAME failed with error!"
    echo "Error: $TOOL_OUTPUT"
else
    echo "Tool $TOOL_NAME completed successfully"
fi

# Example: Track tool usage statistics
LOG_FILE="$HOME/.auroraheart/tool-usage.log"
mkdir -p "$(dirname "$LOG_FILE")"
echo "$(date +%Y-%m-%d\ %H:%M:%S) - $TOOL_NAME - Error: $IS_ERROR" >> "$LOG_FILE"

# Example: Provide feedback on common issues
if [ "$TOOL_NAME" == "bash" ] && [ "$IS_ERROR" == "true" ]; then
    echo "HINT: Check if the command is available in your PATH"
fi

# Exit with 0 to indicate success
exit 0
