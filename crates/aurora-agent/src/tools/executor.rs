//! Tool execution engine
//!
//! This module implements the execution logic for all AI tools.

use super::{ToolResult, ToolUse};
use regex::Regex;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during tool execution
#[derive(Error, Debug)]
pub enum ToolError {
    /// File I/O error
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid tool input
    #[error("Invalid tool input: {0}")]
    InvalidInput(String),

    /// Tool not found
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Command execution error
    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    /// JSON parsing error
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
}

/// Tool executor that can execute tool requests
pub struct ToolExecutor {
    /// Working directory for file operations
    working_directory: std::path::PathBuf,
}

impl ToolExecutor {
    /// Create a new tool executor
    pub fn new() -> Self {
        Self {
            working_directory: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        }
    }

    /// Create a tool executor with a specific working directory
    pub fn with_working_directory(working_directory: impl Into<std::path::PathBuf>) -> Self {
        Self {
            working_directory: working_directory.into(),
        }
    }

    /// Execute a tool use request
    pub async fn execute(&self, tool_use: &ToolUse) -> ToolResult {
        let result = match tool_use.name.as_str() {
            "read" => self.execute_read(&tool_use.input).await,
            "write" => self.execute_write(&tool_use.input).await,
            "edit" => self.execute_edit(&tool_use.input).await,
            "bash" => self.execute_bash(&tool_use.input).await,
            "grep" => self.execute_grep(&tool_use.input).await,
            "glob" => self.execute_glob(&tool_use.input).await,
            unknown => Err(ToolError::ToolNotFound(unknown.to_string())),
        };

        match result {
            Ok(content) => ToolResult::success(tool_use.id.clone(), content),
            Err(e) => ToolResult::error(tool_use.id.clone(), e.to_string()),
        }
    }

    /// Execute the Read tool
    async fn execute_read(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing file_path".to_string()))?;

        let path = Path::new(file_path);
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory.join(path)
        };

        let contents = tokio::fs::read_to_string(&absolute_path).await?;
        Ok(contents)
    }

    /// Execute the Write tool
    async fn execute_write(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing file_path".to_string()))?;

        let content = input["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing content".to_string()))?;

        let path = Path::new(file_path);
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory.join(path)
        };

        // Create parent directories if they don't exist
        if let Some(parent) = absolute_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&absolute_path, content).await?;
        Ok(format!("Successfully wrote {} bytes to {}", content.len(), file_path))
    }

    /// Execute the Edit tool
    async fn execute_edit(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing file_path".to_string()))?;

        let old_string = input["old_string"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing old_string".to_string()))?;

        let new_string = input["new_string"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing new_string".to_string()))?;

        let path = Path::new(file_path);
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory.join(path)
        };

        let contents = tokio::fs::read_to_string(&absolute_path).await?;

        // Check if old_string exists in the file
        if !contents.contains(old_string) {
            return Err(ToolError::InvalidInput(format!(
                "String not found in file: {}",
                old_string
            )));
        }

        // Replace the string
        let new_contents = contents.replace(old_string, new_string);

        // Write back to file
        tokio::fs::write(&absolute_path, new_contents).await?;

        Ok(format!(
            "Successfully replaced string in {}",
            file_path
        ))
    }

    /// Execute the Bash tool
    async fn execute_bash(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let command = input["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing command".to_string()))?;

        // Determine the shell based on the platform
        #[cfg(target_os = "windows")]
        let shell = "cmd";
        #[cfg(target_os = "windows")]
        let shell_arg = "/C";

        #[cfg(not(target_os = "windows"))]
        let shell = "sh";
        #[cfg(not(target_os = "windows"))]
        let shell_arg = "-c";

        let output = tokio::process::Command::new(shell)
            .arg(shell_arg)
            .arg(command)
            .current_dir(&self.working_directory)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::CommandFailed(format!(
                "Command exited with code {:?}: {}",
                output.status.code(),
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Execute the Grep tool
    async fn execute_grep(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let pattern_str = input["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing pattern".to_string()))?;

        let search_path = input["path"]
            .as_str()
            .map(|p| {
                let path = Path::new(p);
                if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    self.working_directory.join(path)
                }
            })
            .unwrap_or_else(|| self.working_directory.clone());

        let case_insensitive = input["case_insensitive"].as_bool().unwrap_or(false);
        let max_results = input["max_results"].as_u64().unwrap_or(100) as usize;
        let file_pattern = input["file_pattern"].as_str();

        // Compile the regex pattern
        let regex_pattern = if case_insensitive {
            format!("(?i){}", pattern_str)
        } else {
            pattern_str.to_string()
        };

        let regex = Regex::new(&regex_pattern)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid regex pattern: {}", e)))?;

        let mut results = Vec::new();
        let mut match_count = 0;

        // Walk the directory tree
        self.search_files_recursive(&search_path, &regex, file_pattern, &mut results, &mut match_count, max_results).await?;

        if results.is_empty() {
            Ok(format!("No matches found for pattern: {}", pattern_str))
        } else {
            Ok(format!(
                "Found {} matches:\n\n{}",
                match_count,
                results.join("\n")
            ))
        }
    }

    /// Recursively search files for pattern matches
    fn search_files_recursive<'a>(
        &'a self,
        path: &'a Path,
        regex: &'a Regex,
        file_pattern: Option<&'a str>,
        results: &'a mut Vec<String>,
        match_count: &'a mut usize,
        max_results: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + 'a>> {
        Box::pin(async move {
        if *match_count >= max_results {
            return Ok(());
        }

        if !path.exists() {
            return Err(ToolError::InvalidInput(format!("Path does not exist: {}", path.display())));
        }

        if path.is_file() {
            // Check if file matches the file pattern
            if let Some(pattern) = file_pattern {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    let glob_pattern = glob::Pattern::new(pattern)
                        .map_err(|e| ToolError::InvalidInput(format!("Invalid glob pattern: {}", e)))?;
                    if !glob_pattern.matches(file_name) {
                        return Ok(());
                    }
                }
            }

            // Search in the file
            if let Ok(content) = tokio::fs::read_to_string(path).await {
                for (line_num, line) in content.lines().enumerate() {
                    if *match_count >= max_results {
                        break;
                    }
                    if regex.is_match(line) {
                        results.push(format!(
                            "{}:{}: {}",
                            path.display(),
                            line_num + 1,
                            line.trim()
                        ));
                        *match_count += 1;
                    }
                }
            }
        } else if path.is_dir() {
            // Recursively search subdirectories
            let mut entries = tokio::fs::read_dir(path).await?;
            while let Some(entry) = entries.next_entry().await? {
                self.search_files_recursive(
                    &entry.path(),
                    regex,
                    file_pattern,
                    results,
                    match_count,
                    max_results,
                ).await?;

                if *match_count >= max_results {
                    break;
                }
            }
        }

        Ok(())
        })
    }

    /// Execute the Glob tool
    async fn execute_glob(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let pattern = input["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing pattern".to_string()))?;

        let base_path = input["path"]
            .as_str()
            .map(|p| {
                let path = Path::new(p);
                if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    self.working_directory.join(path)
                }
            })
            .unwrap_or_else(|| self.working_directory.clone());

        let max_results = input["max_results"].as_u64().unwrap_or(100) as usize;

        // Build the full glob pattern
        let full_pattern = base_path.join(pattern);
        let pattern_str = full_pattern
            .to_str()
            .ok_or_else(|| ToolError::InvalidInput("Invalid path".to_string()))?;

        // Execute glob search
        let entries: Result<Vec<_>, _> = glob::glob(pattern_str)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid glob pattern: {}", e)))?
            .take(max_results)
            .collect();

        let paths = entries.map_err(|e| ToolError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Glob error: {}", e),
        )))?;

        if paths.is_empty() {
            Ok(format!("No files found matching pattern: {}", pattern))
        } else {
            let file_list: Vec<String> = paths
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            Ok(format!(
                "Found {} files:\n\n{}",
                file_list.len(),
                file_list.join("\n")
            ))
        }
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_tool() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Hello, World!").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({
                "file_path": "test.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.tool_use_id, "test_123");
        assert_eq!(result.content, "Hello, World!");
        assert_eq!(result.is_error, None);
    }

    #[tokio::test]
    async fn test_read_tool_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({
                "file_path": "nonexistent.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.tool_use_id, "test_123");
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("File I/O error") || result.content.contains("No such file"));
    }

    #[tokio::test]
    async fn test_write_tool() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "write".to_string(),
            input: serde_json::json!({
                "file_path": "new_file.txt",
                "content": "Test content"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.tool_use_id, "test_123");
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully wrote"));

        // Verify file was created
        let file_path = temp_dir.path().join("new_file.txt");
        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(contents, "Test content");
    }

    #[tokio::test]
    async fn test_write_tool_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "write".to_string(),
            input: serde_json::json!({
                "file_path": "subdir/nested/file.txt",
                "content": "Nested content"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);

        // Verify file was created in nested directory
        let file_path = temp_dir.path().join("subdir/nested/file.txt");
        assert!(file_path.exists());
        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(contents, "Nested content");
    }

    #[tokio::test]
    async fn test_edit_tool() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("edit_test.txt");
        tokio::fs::write(&file_path, "Hello, World!").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "edit".to_string(),
            input: serde_json::json!({
                "file_path": "edit_test.txt",
                "old_string": "World",
                "new_string": "Rust"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully replaced"));

        // Verify file was edited
        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(contents, "Hello, Rust!");
    }

    #[tokio::test]
    async fn test_edit_tool_string_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("edit_test.txt");
        tokio::fs::write(&file_path, "Hello, World!").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "edit".to_string(),
            input: serde_json::json!({
                "file_path": "edit_test.txt",
                "old_string": "NotFound",
                "new_string": "Replacement"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("String not found"));
    }

    #[tokio::test]
    async fn test_bash_tool() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        #[cfg(target_os = "windows")]
        let command = "echo Hello from Windows";

        #[cfg(not(target_os = "windows"))]
        let command = "echo Hello from Unix";

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({
                "command": command
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);

        #[cfg(target_os = "windows")]
        assert!(result.content.contains("Hello from Windows"));

        #[cfg(not(target_os = "windows"))]
        assert!(result.content.contains("Hello from Unix"));
    }

    #[tokio::test]
    async fn test_bash_tool_command_failure() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({
                "command": "exit 1"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Command exited"));
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let executor = ToolExecutor::new();
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "unknown_tool".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Tool not found"));
    }

    #[tokio::test]
    async fn test_read_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("absolute_test.txt");
        tokio::fs::write(&file_path, "Absolute path content").await.unwrap();

        let executor = ToolExecutor::new(); // Uses current directory by default
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap()
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert_eq!(result.content, "Absolute path content");
    }

    #[tokio::test]
    async fn test_grep_tool() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        tokio::fs::write(temp_dir.path().join("test1.txt"), "Hello World\nFoo Bar\nHello Again").await.unwrap();
        tokio::fs::write(temp_dir.path().join("test2.txt"), "No match here\nJust text").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "grep".to_string(),
            input: serde_json::json!({
                "pattern": "Hello"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Hello World"));
        assert!(result.content.contains("Hello Again"));
        assert!(result.content.contains("Found 2 matches"));
    }

    #[tokio::test]
    async fn test_grep_tool_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("test.txt"), "HELLO world\nhello WORLD").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "grep".to_string(),
            input: serde_json::json!({
                "pattern": "hello",
                "case_insensitive": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Found 2 matches"));
    }

    #[tokio::test]
    async fn test_grep_tool_with_file_pattern() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("test.rs"), "fn main() {}").await.unwrap();
        tokio::fs::write(temp_dir.path().join("test.txt"), "fn main() {}").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "grep".to_string(),
            input: serde_json::json!({
                "pattern": "fn main",
                "file_pattern": "*.rs"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("test.rs"));
        assert!(!result.content.contains("test.txt"));
    }

    #[tokio::test]
    async fn test_grep_tool_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("test.txt"), "Some content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "grep".to_string(),
            input: serde_json::json!({
                "pattern": "NotFound"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("No matches found"));
    }

    #[tokio::test]
    async fn test_glob_tool() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("test1.rs"), "").await.unwrap();
        tokio::fs::write(temp_dir.path().join("test2.rs"), "").await.unwrap();
        tokio::fs::write(temp_dir.path().join("test.txt"), "").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "glob".to_string(),
            input: serde_json::json!({
                "pattern": "*.rs"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("test1.rs"));
        assert!(result.content.contains("test2.rs"));
        assert!(!result.content.contains("test.txt"));
        assert!(result.content.contains("Found 2 files"));
    }

    #[tokio::test]
    async fn test_glob_tool_recursive() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::create_dir(temp_dir.path().join("subdir")).await.unwrap();
        tokio::fs::write(temp_dir.path().join("test.rs"), "").await.unwrap();
        tokio::fs::write(temp_dir.path().join("subdir/nested.rs"), "").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "glob".to_string(),
            input: serde_json::json!({
                "pattern": "**/*.rs"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("test.rs"));
        assert!(result.content.contains("nested.rs"));
    }

    #[tokio::test]
    async fn test_glob_tool_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("test.txt"), "").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "glob".to_string(),
            input: serde_json::json!({
                "pattern": "*.rs"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("No files found"));
    }
}
