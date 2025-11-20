# Session Hooks

Session hooks allow you to modify AuroraHeart's behavior at key lifecycle events by running custom shell scripts.

## Hook Types

AuroraHeart supports four types of hooks:

1. **SessionStart** - Runs when a new conversation session begins
2. **SessionEnd** - Runs when a conversation session ends
3. **BeforeToolCall** - Runs before each tool execution
4. **AfterToolCall** - Runs after each tool execution completes

## How Hooks Work

Hooks are shell scripts (bash or PowerShell) that:
- Receive context through environment variables
- Can inject instructions into the conversation through stdout
- Cannot block or prevent operations (hooks are informational only)
- Run asynchronously and don't affect performance

### Environment Variables by Hook Type

#### SessionStart Hook
- `PROJECT_ROOT` - Root directory of the current project
- `INITIAL_MESSAGE` - The first message in the conversation (if any)

#### SessionEnd Hook
- `PROJECT_ROOT` - Root directory of the current project
- `SESSION_DURATION` - Duration of the session in seconds

#### BeforeToolCall Hook
- `TOOL_NAME` - Name of the tool about to be executed
- `TOOL_ID` - Unique identifier for this tool call
- `TOOL_INPUT` - JSON string of the tool's input parameters

#### AfterToolCall Hook
- `TOOL_NAME` - Name of the tool that was executed
- `TOOL_ID` - Unique identifier for this tool call
- `TOOL_INPUT` - JSON string of the tool's input parameters
- `TOOL_OUTPUT` - The output/result from the tool
- `IS_ERROR` - "true" if the tool failed, "false" otherwise

## Setting Up Hooks

Hooks are configured through the plugin system. Create a plugin with hook definitions:

### Example Plugin Structure

```
.AuroraHeart/
└── plugins/
    └── my-hooks/
        ├── plugin.toml
        └── hooks/
            ├── session-start.sh
            ├── before-tool.sh
            └── after-tool.sh
```

### Example plugin.toml

```toml
[plugin]
name = "my-hooks"
version = "1.0.0"
description = "Custom session hooks"

[[hooks]]
type = "SessionStart"
script_path = "hooks/session-start.sh"

[[hooks]]
type = "BeforeToolCall"
script_path = "hooks/before-tool.sh"

[[hooks]]
type = "AfterToolCall"
script_path = "hooks/after-tool.sh"
```

## Example Use Cases

### 1. Inject Custom Instructions

Use SessionStart hooks to add project-specific instructions:

```bash
#!/bin/bash
# Read project preferences and inject them
if [ -f ".auroraheart-preferences" ]; then
    cat ".auroraheart-preferences"
fi
```

### 2. Validate Operations

Use BeforeToolCall hooks to warn about dangerous operations:

```bash
#!/bin/bash
if [ "$TOOL_NAME" == "bash" ] && [[ "$TOOL_INPUT" == *"rm -rf"* ]]; then
    echo "WARNING: Destructive operation detected!"
fi
```

### 3. Track Tool Usage

Use AfterToolCall hooks to log tool execution:

```bash
#!/bin/bash
LOG_FILE="$HOME/.auroraheart/tool-usage.log"
echo "$(date) - $TOOL_NAME - Error: $IS_ERROR" >> "$LOG_FILE"
```

### 4. Provide Contextual Help

Use AfterToolCall hooks to provide hints when tools fail:

```bash
#!/bin/bash
if [ "$IS_ERROR" == "true" ]; then
    echo "HINT: Check the tool documentation for proper usage"
fi
```

## Platform Support

Hooks work on both Unix (bash) and Windows (PowerShell):

- **Unix/Linux/Mac**: Use `.sh` files with bash shebang (`#!/bin/bash`)
- **Windows**: Use `.ps1` files for PowerShell scripts

AuroraHeart automatically detects the platform and uses the appropriate shell.

## Best Practices

1. **Keep hooks fast** - Hooks run synchronously, so avoid long-running operations
2. **Handle errors gracefully** - If your hook fails, it won't break the IDE
3. **Use stdout for injections** - Only output to stdout what you want injected into the conversation
4. **Use stderr for logging** - Diagnostic messages should go to stderr
5. **Make hooks optional** - Users should be able to disable hooks without breaking functionality

## Examples

See the example hook scripts in this directory:
- [session-start-example.sh](session-start-example.sh)
- [before-tool-call-example.sh](before-tool-call-example.sh)
- [after-tool-call-example.sh](after-tool-call-example.sh)
- [session-start-example.ps1](session-start-example.ps1) (Windows)
- [before-tool-call-example.ps1](before-tool-call-example.ps1) (Windows)
- [after-tool-call-example.ps1](after-tool-call-example.ps1) (Windows)

## Debugging Hooks

To debug hooks:

1. Check stderr output - Hook errors are logged there
2. Test hooks manually - Run them with appropriate environment variables set
3. Use verbose logging - Add debug output to your hooks
4. Check plugin loading - Ensure your plugin is loaded with `bd list --status open`

## Security Considerations

Hooks run with the same permissions as AuroraHeart, so:
- Only use hooks from trusted sources
- Review hook scripts before enabling them
- Be cautious with hooks that execute external commands
- Hooks cannot block operations, only provide feedback
