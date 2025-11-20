# Session Start Hook Example (PowerShell)
# This hook runs when a new conversation session starts

# Access environment variables provided by the hook system
$PROJECT_ROOT = $env:PROJECT_ROOT
$INITIAL_MESSAGE = $env:INITIAL_MESSAGE

# Log session start
Write-Host "Session started in project: $PROJECT_ROOT"
Write-Host "Initial message: $INITIAL_MESSAGE"

# Example: Inject a custom instruction into the conversation
Write-Output "IMPORTANT: Please be concise and direct in your responses."

# Example: Check for project-specific requirements
$prefsFile = Join-Path $PROJECT_ROOT ".auroraheart-preferences"
if (Test-Path $prefsFile) {
    Get-Content $prefsFile
}

# Exit with 0 to indicate success
exit 0
