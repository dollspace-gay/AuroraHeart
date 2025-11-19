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

/// Create the List Directory tool definition
pub fn list_directory_tool() -> Tool {
    Tool {
        name: "list_directory".to_string(),
        description: "List directory contents with file metadata including size, modified time, and type.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The directory path to list (defaults to current directory)"
                },
                "show_hidden": {
                    "type": "boolean",
                    "description": "Whether to show hidden files (default: false)"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Whether to list subdirectories recursively (default: false)"
                }
            },
            "required": []
        }),
    }
}

/// Create the Multi-File Replace tool definition
pub fn multi_replace_tool() -> Tool {
    Tool {
        name: "multi_replace".to_string(),
        description: "Search and replace a pattern across multiple files with preview support.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regular expression pattern to search for"
                },
                "replacement": {
                    "type": "string",
                    "description": "The replacement string (can include capture groups like $1, $2)"
                },
                "path": {
                    "type": "string",
                    "description": "The directory to search in (defaults to current directory)"
                },
                "file_pattern": {
                    "type": "string",
                    "description": "Optional glob pattern to filter files (e.g., '*.rs', '*.{js,ts}')"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Whether to perform case-insensitive search (default: false)"
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "If true, preview changes without modifying files (default: true)"
                },
                "max_files": {
                    "type": "integer",
                    "description": "Maximum number of files to process (default: 50)"
                }
            },
            "required": ["pattern", "replacement"]
        }),
    }
}

/// Create the Syntax Check tool definition
pub fn syntax_check_tool() -> Tool {
    Tool {
        name: "syntax_check".to_string(),
        description: "Check syntax validity of code files using language-specific validators. Supports Rust, JavaScript, TypeScript, Python, and more.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to check"
                },
                "language": {
                    "type": "string",
                    "description": "Optional language override (rust, javascript, typescript, python, etc.). If not specified, detected from file extension."
                },
                "strict": {
                    "type": "boolean",
                    "description": "Enable strict checking mode with more detailed diagnostics (default: false)"
                }
            },
            "required": ["file_path"]
        }),
    }
}

/// Create the Code Format tool definition
pub fn code_format_tool() -> Tool {
    Tool {
        name: "code_format".to_string(),
        description: "Format code files according to language-specific style guidelines. Supports Rust (rustfmt), JavaScript/TypeScript (prettier), Python (black), Go (gofmt), and C/C++ (clang-format).".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to format"
                },
                "language": {
                    "type": "string",
                    "description": "Optional language override (rust, javascript, typescript, python, go, c, cpp). If not specified, detected from file extension."
                },
                "check_only": {
                    "type": "boolean",
                    "description": "If true, only check if file is formatted correctly without modifying (default: false)"
                }
            },
            "required": ["file_path"]
        }),
    }
}

/// Create the Code Analysis tool definition
pub fn code_analysis_tool() -> Tool {
    Tool {
        name: "code_analysis".to_string(),
        description: "Analyze code quality, complexity, and security vulnerabilities. Supports Rust (clippy, cargo-audit), JavaScript/TypeScript (eslint, npm audit), Python (pylint, bandit), and more.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to analyze (file or directory)"
                },
                "language": {
                    "type": "string",
                    "description": "Optional language override (rust, javascript, typescript, python, go). If not specified, detected from file extension or project structure."
                },
                "analysis_type": {
                    "type": "string",
                    "description": "Type of analysis: 'quality' (linting/code smells), 'security' (vulnerability scanning), or 'all' (default: 'all')",
                    "enum": ["quality", "security", "all"]
                },
                "strict": {
                    "type": "boolean",
                    "description": "Enable strict analysis mode with more detailed checks (default: false)"
                }
            },
            "required": ["path"]
        }),
    }
}

/// Create the Copy tool definition
pub fn copy_tool() -> Tool {
    Tool {
        name: "copy".to_string(),
        description: "Copy files or directories to a new location. Supports recursive copying of directory trees.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "The source file or directory path to copy"
                },
                "destination": {
                    "type": "string",
                    "description": "The destination path where the source should be copied"
                },
                "overwrite": {
                    "type": "boolean",
                    "description": "Whether to overwrite existing files at the destination (default: false)"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "If true, copy directories recursively (default: true)"
                }
            },
            "required": ["source", "destination"]
        }),
    }
}

/// Create the Delete tool definition
pub fn delete_tool() -> Tool {
    Tool {
        name: "delete".to_string(),
        description: "Delete files or directories. Includes safety checks and requires confirmation for directory deletion.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file or directory to delete"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "If true, delete directories and their contents recursively (default: false)"
                }
            },
            "required": ["path"]
        }),
    }
}

/// Create the Move tool definition
pub fn move_tool() -> Tool {
    Tool {
        name: "move".to_string(),
        description: "Move or rename files and directories to a new location. Supports cross-directory moves and atomic renames.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "The source file or directory path to move"
                },
                "destination": {
                    "type": "string",
                    "description": "The destination path where the source should be moved"
                },
                "overwrite": {
                    "type": "boolean",
                    "description": "Whether to overwrite existing files at the destination (default: false)"
                }
            },
            "required": ["source", "destination"]
        }),
    }
}

/// Create the Build tool definition
pub fn build_tool() -> Tool {
    Tool {
        name: "build".to_string(),
        description: "Execute build commands for various project types. Supports Rust (cargo build), JavaScript/TypeScript (npm run build), Python (python setup.py build), Go (go build), and custom build commands.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "project_type": {
                    "type": "string",
                    "description": "Optional project type override (rust, javascript, typescript, python, go). If not specified, detected from project structure.",
                    "enum": ["rust", "javascript", "typescript", "python", "go", "custom"]
                },
                "build_type": {
                    "type": "string",
                    "description": "Build configuration type: 'debug' or 'release' (default: 'debug')",
                    "enum": ["debug", "release"]
                },
                "custom_command": {
                    "type": "string",
                    "description": "Custom build command to execute (used when project_type is 'custom')"
                },
                "args": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Additional arguments to pass to the build command"
                },
                "working_directory": {
                    "type": "string",
                    "description": "Working directory for the build (defaults to current directory)"
                }
            },
            "required": []
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
        list_directory_tool(),
        multi_replace_tool(),
        syntax_check_tool(),
        code_format_tool(),
        code_analysis_tool(),
        copy_tool(),
        delete_tool(),
        move_tool(),
        build_tool(),
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
        assert_eq!(tools.len(), 15);

        let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
        assert!(tool_names.contains(&"read".to_string()));
        assert!(tool_names.contains(&"write".to_string()));
        assert!(tool_names.contains(&"edit".to_string()));
        assert!(tool_names.contains(&"bash".to_string()));
        assert!(tool_names.contains(&"grep".to_string()));
        assert!(tool_names.contains(&"glob".to_string()));
        assert!(tool_names.contains(&"list_directory".to_string()));
        assert!(tool_names.contains(&"multi_replace".to_string()));
        assert!(tool_names.contains(&"syntax_check".to_string()));
        assert!(tool_names.contains(&"code_format".to_string()));
        assert!(tool_names.contains(&"code_analysis".to_string()));
        assert!(tool_names.contains(&"copy".to_string()));
        assert!(tool_names.contains(&"delete".to_string()));
        assert!(tool_names.contains(&"move".to_string()));
        assert!(tool_names.contains(&"build".to_string()));
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
