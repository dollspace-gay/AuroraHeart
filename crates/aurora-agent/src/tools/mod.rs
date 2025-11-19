//! Tool definitions and execution for Claude API
//!
//! This module defines the tools that Claude can use to interact with the IDE,
//! including their schemas and execution logic.

mod executor;

pub use executor::{ToolExecutor, ToolError};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// A tool that Claude can use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Name of the tool
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// JSON schema for the tool's input parameters
    pub input_schema: JsonValue,
}

/// Tool use request from Claude
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    /// Unique ID for this tool use
    pub id: String,
    /// Name of the tool to use
    pub name: String,
    /// Input parameters for the tool
    pub input: JsonValue,
}

/// Result of executing a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool use this is responding to
    pub tool_use_id: String,
    /// Content of the result (can be text or JSON)
    pub content: String,
    /// Whether the tool execution failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResult {
    /// Create a successful tool result
    pub fn success(tool_use_id: String, content: String) -> Self {
        Self {
            tool_use_id,
            content,
            is_error: None,
        }
    }

    /// Create an error tool result
    pub fn error(tool_use_id: String, error_message: String) -> Self {
        Self {
            tool_use_id,
            content: error_message,
            is_error: Some(true),
        }
    }
}

/// Create the Read tool definition
pub fn read_tool() -> Tool {
    Tool {
        name: "read".to_string(),
        description: "Read the contents of a file from the filesystem.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to read"
                }
            },
            "required": ["file_path"]
        }),
    }
}

/// Create the Write tool definition
pub fn write_tool() -> Tool {
    Tool {
        name: "write".to_string(),
        description: "Write content to a file, creating it if it doesn't exist or overwriting if it does.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["file_path", "content"]
        }),
    }
}

/// Create the Edit tool definition
pub fn edit_tool() -> Tool {
    Tool {
        name: "edit".to_string(),
        description: "Perform exact string replacements in a file.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact string to replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The string to replace it with"
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        }),
    }
}

/// Create the Bash tool definition
pub fn bash_tool() -> Tool {
    Tool {
        name: "bash".to_string(),
        description: "Execute a bash command and return its output.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                }
            },
            "required": ["command"]
        }),
    }
}

/// Create the Grep tool definition
pub fn grep_tool() -> Tool {
    Tool {
        name: "grep".to_string(),
        description: "Search for a pattern in files using regular expressions. Returns matching lines with file names and line numbers.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regular expression pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "The file or directory to search in (defaults to current directory)"
                },
                "file_pattern": {
                    "type": "string",
                    "description": "Optional glob pattern to filter files (e.g., '*.rs', '*.{js,ts}')"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Whether to perform case-insensitive search (default: false)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 100)"
                }
            },
            "required": ["pattern"]
        }),
    }
}

/// Create the Glob tool definition
pub fn glob_tool() -> Tool {
    Tool {
        name: "glob".to_string(),
        description: "Find files matching a glob pattern. Useful for discovering files by name or extension.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The glob pattern to match (e.g., '**/*.rs', 'src/**/*.{js,ts}')"
                },
                "path": {
                    "type": "string",
                    "description": "The base directory to search from (defaults to current directory)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 100)"
                }
            },
            "required": ["pattern"]
        }),
    }
}

/// Get all available tools
pub fn all_tools() -> Vec<Tool> {
    vec![
        read_tool(),
        write_tool(),
        edit_tool(),
        bash_tool(),
        grep_tool(),
        glob_tool(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_serialization() {
        let tool = read_tool();
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("read"));
        assert!(json.contains("file_path"));
    }

    #[test]
    fn test_tool_use_serialization() {
        let tool_use = ToolUse {
            id: "toolu_123".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({"file_path": "/tmp/test.txt"}),
        };

        let json = serde_json::to_string(&tool_use).unwrap();
        assert!(json.contains("toolu_123"));
        assert!(json.contains("read"));
        assert!(json.contains("/tmp/test.txt"));
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success(
            "toolu_123".to_string(),
            "File contents here".to_string(),
        );

        assert_eq!(result.tool_use_id, "toolu_123");
        assert_eq!(result.content, "File contents here");
        assert_eq!(result.is_error, None);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error(
            "toolu_123".to_string(),
            "File not found".to_string(),
        );

        assert_eq!(result.tool_use_id, "toolu_123");
        assert_eq!(result.content, "File not found");
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn test_all_tools() {
        let tools = all_tools();
        assert_eq!(tools.len(), 6);

        let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
        assert!(tool_names.contains(&"read".to_string()));
        assert!(tool_names.contains(&"write".to_string()));
        assert!(tool_names.contains(&"edit".to_string()));
        assert!(tool_names.contains(&"bash".to_string()));
        assert!(tool_names.contains(&"grep".to_string()));
        assert!(tool_names.contains(&"glob".to_string()));
    }

    #[test]
    fn test_read_tool_schema() {
        let tool = read_tool();
        assert_eq!(tool.name, "read");
        assert!(tool.description.contains("Read"));

        let schema = tool.input_schema;
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["file_path"].is_object());
        assert_eq!(schema["required"][0], "file_path");
    }

    #[test]
    fn test_write_tool_schema() {
        let tool = write_tool();
        assert_eq!(tool.name, "write");
        assert!(tool.description.contains("Write"));

        let schema = tool.input_schema;
        assert!(schema["properties"]["file_path"].is_object());
        assert!(schema["properties"]["content"].is_object());
        assert_eq!(schema["required"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_edit_tool_schema() {
        let tool = edit_tool();
        assert_eq!(tool.name, "edit");
        assert!(tool.description.contains("string replacements"));

        let schema = tool.input_schema;
        assert!(schema["properties"]["file_path"].is_object());
        assert!(schema["properties"]["old_string"].is_object());
        assert!(schema["properties"]["new_string"].is_object());
        assert_eq!(schema["required"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_bash_tool_schema() {
        let tool = bash_tool();
        assert_eq!(tool.name, "bash");
        assert!(tool.description.contains("bash command"));

        let schema = tool.input_schema;
        assert!(schema["properties"]["command"].is_object());
        assert_eq!(schema["required"][0], "command");
    }
}
