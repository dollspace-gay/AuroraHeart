# After Tool Call Hook Example (PowerShell)
# This hook runs after each tool execution completes

# Access environment variables provided by the hook system
$TOOL_NAME = $env:TOOL_NAME
$TOOL_ID = $env:TOOL_ID
$TOOL_INPUT = $env:TOOL_INPUT
$TOOL_OUTPUT = $env:TOOL_OUTPUT
$IS_ERROR = $env:IS_ERROR

# Log tool result
if ($IS_ERROR -eq "true") {
    Write-Host "Tool $TOOL_NAME failed with error!"
    Write-Host "Error: $TOOL_OUTPUT"
} else {
    Write-Host "Tool $TOOL_NAME completed successfully"
}

# Example: Track tool usage statistics
$logDir = Join-Path $env:USERPROFILE ".auroraheart"
$logFile = Join-Path $logDir "tool-usage.log"
if (-not (Test-Path $logDir)) {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}
$timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
Add-Content -Path $logFile -Value "$timestamp - $TOOL_NAME - Error: $IS_ERROR"

# Example: Provide feedback on common issues
if ($TOOL_NAME -eq "bash" -and $IS_ERROR -eq "true") {
    Write-Output "HINT: Check if the command is available in your PATH"
}

# Exit with 0 to indicate success
exit 0
