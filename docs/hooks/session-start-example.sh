#!/bin/bash
# Session Start Hook Example
# This hook runs when a new conversation session starts

# Access environment variables provided by the hook system
PROJECT_ROOT="${PROJECT_ROOT:-}"
INITIAL_MESSAGE="${INITIAL_MESSAGE:-}"

# Log session start
echo "Session started in project: $PROJECT_ROOT"
echo "Initial message: $INITIAL_MESSAGE"

# Example: Inject a custom instruction into the conversation
echo "IMPORTANT: Please be concise and direct in your responses."

# Example: Check for project-specific requirements
if [ -f "$PROJECT_ROOT/.auroraheart-preferences" ]; then
    cat "$PROJECT_ROOT/.auroraheart-preferences"
fi

# Exit with 0 to indicate success
exit 0
