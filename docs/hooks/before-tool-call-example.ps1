# Before Tool Call Hook Example (PowerShell)
# This hook runs before each tool is executed

# Access environment variables provided by the hook system
$TOOL_NAME = $env:TOOL_NAME
$TOOL_ID = $env:TOOL_ID
$TOOL_INPUT = $env:TOOL_INPUT

# Log tool call
Write-Host "About to execute tool: $TOOL_NAME (ID: $TOOL_ID)"

# Example: Prevent dangerous operations in production
if ($TOOL_NAME -eq "bash" -and $TOOL_INPUT -like "*rm -rf*") {
    Write-Output "WARNING: Dangerous bash command detected!"
    # Hook can inject warnings but cannot block execution
}

# Example: Add context based on tool type
if ($TOOL_NAME -eq "write") {
    Write-Output "TIP: Remember to follow the project's coding style guidelines"
}

# Exit with 0 to indicate success
exit 0
