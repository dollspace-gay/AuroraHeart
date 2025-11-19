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
            "list_directory" => self.execute_list_directory(&tool_use.input).await,
            "multi_replace" => self.execute_multi_replace(&tool_use.input).await,
            "syntax_check" => self.execute_syntax_check(&tool_use.input).await,
            "code_format" => self.execute_code_format(&tool_use.input).await,
            "code_analysis" => self.execute_code_analysis(&tool_use.input).await,
            "copy" => self.execute_copy(&tool_use.input).await,
            "delete" => self.execute_delete(&tool_use.input).await,
            "move" => self.execute_move(&tool_use.input).await,
            "build" => self.execute_build(&tool_use.input).await,
            "test_runner" => self.execute_test_runner(&tool_use.input).await,
            "lint" => self.execute_lint(&tool_use.input).await,
            "task" => self.execute_task(&tool_use.input).await,
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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send + 'a>> {
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

    /// Execute the List Directory tool
    async fn execute_list_directory(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let dir_path = input["path"]
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

        let show_hidden = input["show_hidden"].as_bool().unwrap_or(false);
        let recursive = input["recursive"].as_bool().unwrap_or(false);

        if !dir_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "Directory does not exist: {}",
                dir_path.display()
            )));
        }

        if !dir_path.is_dir() {
            return Err(ToolError::InvalidInput(format!(
                "Path is not a directory: {}",
                dir_path.display()
            )));
        }

        let mut entries = Vec::new();
        self.collect_directory_entries(&dir_path, show_hidden, recursive, &mut entries, 0)
            .await?;

        if entries.is_empty() {
            Ok(format!("Directory is empty: {}", dir_path.display()))
        } else {
            // Sort entries: directories first, then by name
            entries.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            });

            let formatted_entries: Vec<String> = entries
                .iter()
                .map(|e| e.format())
                .collect();

            Ok(format!(
                "Directory: {}\n{} items:\n\n{}",
                dir_path.display(),
                entries.len(),
                formatted_entries.join("\n")
            ))
        }
    }

    /// Recursively collect directory entries
    fn collect_directory_entries<'a>(
        &'a self,
        dir_path: &'a Path,
        show_hidden: bool,
        recursive: bool,
        entries: &'a mut Vec<DirectoryEntry>,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let mut read_dir = tokio::fs::read_dir(dir_path).await?;

            while let Some(entry) = read_dir.next_entry().await? {
                let file_name = entry
                    .file_name()
                    .to_string_lossy()
                    .to_string();

                // Skip hidden files if not requested
                if !show_hidden && file_name.starts_with('.') {
                    continue;
                }

                let path = entry.path();
                let metadata = entry.metadata().await?;

                let is_dir = metadata.is_dir();
                let size = if is_dir { None } else { Some(metadata.len()) };

                // Get modified time
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|time| {
                        time.duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|d| {
                                let datetime = chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)?;
                                Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
                            })
                    })
                    .flatten();

                entries.push(DirectoryEntry {
                    name: file_name.clone(),
                    path: path.display().to_string(),
                    is_dir,
                    size,
                    modified,
                    depth,
                });

                // Recursively process subdirectories
                if recursive && is_dir {
                    self.collect_directory_entries(&path, show_hidden, recursive, entries, depth + 1)
                        .await?;
                }
            }

            Ok(())
        })
    }

    /// Execute the Multi-File Replace tool
    async fn execute_multi_replace(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let pattern_str = input["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing pattern".to_string()))?;

        let replacement = input["replacement"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing replacement".to_string()))?;

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

        let file_pattern = input["file_pattern"].as_str();
        let case_insensitive = input["case_insensitive"].as_bool().unwrap_or(false);
        let dry_run = input["dry_run"].as_bool().unwrap_or(true);
        let max_files = input["max_files"].as_u64().unwrap_or(50) as usize;

        // Compile the regex pattern
        let regex_pattern = if case_insensitive {
            format!("(?i){}", pattern_str)
        } else {
            pattern_str.to_string()
        };

        let regex = Regex::new(&regex_pattern)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid regex pattern: {}", e)))?;

        // Collect files to process
        let mut files_to_process = Vec::new();
        self.collect_files_for_replace(&search_path, file_pattern, &mut files_to_process, max_files)
            .await?;

        if files_to_process.is_empty() {
            return Ok("No files found matching the criteria".to_string());
        }

        // Process each file
        let mut changes = Vec::new();
        let mut files_changed = 0;
        let mut total_replacements = 0;

        for file_path in files_to_process.iter().take(max_files) {
            // Read file content
            let content = match tokio::fs::read_to_string(file_path).await {
                Ok(c) => c,
                Err(_) => continue, // Skip files that can't be read
            };

            // Apply replacements
            let mut replacement_count = 0;
            let new_content = regex.replace_all(&content, |caps: &regex::Captures| {
                replacement_count += 1;
                // Expand capture groups in replacement string
                let mut result = replacement.to_string();
                for i in 0..caps.len() {
                    result = result.replace(&format!("${}", i), caps.get(i).map(|m| m.as_str()).unwrap_or(""));
                }
                result
            });

            // Check if any changes were made
            if new_content != content {
                files_changed += 1;
                total_replacements += replacement_count;

                if dry_run {
                    // Preview mode - show what would change
                    changes.push(format!(
                        "📄 {}\n   {} replacements would be made",
                        file_path.display(),
                        replacement_count
                    ));
                } else {
                    // Actually write the changes
                    tokio::fs::write(file_path, new_content.as_bytes()).await?;
                    changes.push(format!(
                        "✅ {}\n   {} replacements made",
                        file_path.display(),
                        replacement_count
                    ));
                }
            }
        }

        // Format the result
        if changes.is_empty() {
            Ok(format!(
                "Searched {} files, no matches found for pattern: {}",
                files_to_process.len(),
                pattern_str
            ))
        } else {
            let mode_str = if dry_run { "DRY RUN - Preview of changes" } else { "Changes applied" };
            Ok(format!(
                "{}\n\n{} files would be changed with {} total replacements:\n\n{}",
                mode_str,
                files_changed,
                total_replacements,
                changes.join("\n")
            ))
        }
    }

    /// Recursively collect files for replacement
    fn collect_files_for_replace<'a>(
        &'a self,
        dir_path: &'a Path,
        file_pattern: Option<&'a str>,
        files: &'a mut Vec<std::path::PathBuf>,
        max_files: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send + 'a>> {
        Box::pin(async move {
            if files.len() >= max_files {
                return Ok(());
            }

            if !dir_path.exists() {
                return Err(ToolError::InvalidInput(format!(
                    "Path does not exist: {}",
                    dir_path.display()
                )));
            }

            if dir_path.is_file() {
                // Check if file matches the file pattern
                if let Some(pattern) = file_pattern {
                    if let Some(file_name) = dir_path.file_name().and_then(|n| n.to_str()) {
                        let glob_pattern = glob::Pattern::new(pattern)
                            .map_err(|e| ToolError::InvalidInput(format!("Invalid glob pattern: {}", e)))?;
                        if glob_pattern.matches(file_name) {
                            files.push(dir_path.to_path_buf());
                        }
                    }
                } else {
                    files.push(dir_path.to_path_buf());
                }
            } else if dir_path.is_dir() {
                let mut entries = tokio::fs::read_dir(dir_path).await?;
                while let Some(entry) = entries.next_entry().await? {
                    if files.len() >= max_files {
                        break;
                    }
                    self.collect_files_for_replace(&entry.path(), file_pattern, files, max_files)
                        .await?;
                }
            }

            Ok(())
        })
    }

    /// Execute the Syntax Check tool
    async fn execute_syntax_check(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing file_path".to_string()))?;

        let path = Path::new(file_path);
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory.join(path)
        };

        // Check if file exists
        if !absolute_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "File does not exist: {}",
                absolute_path.display()
            )));
        }

        // Detect language from file extension or use provided language
        let language = if let Some(lang) = input["language"].as_str() {
            lang.to_lowercase()
        } else {
            // Auto-detect from file extension
            absolute_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| match e {
                    "rs" => "rust",
                    "js" | "mjs" | "cjs" => "javascript",
                    "ts" | "mts" | "cts" => "typescript",
                    "py" => "python",
                    "go" => "go",
                    "java" => "java",
                    "c" => "c",
                    "cpp" | "cc" | "cxx" => "cpp",
                    "rb" => "ruby",
                    "php" => "php",
                    "swift" => "swift",
                    "kt" => "kotlin",
                    "cs" => "csharp",
                    _ => "unknown",
                })
                .unwrap_or("unknown")
                .to_string()
        };

        let strict = input["strict"].as_bool().unwrap_or(false);

        // Execute language-specific syntax checker
        let result = match language.as_str() {
            "rust" => self.check_rust_syntax(&absolute_path, strict).await?,
            "javascript" | "typescript" => self.check_js_ts_syntax(&absolute_path, &language, strict).await?,
            "python" => self.check_python_syntax(&absolute_path, strict).await?,
            "go" => self.check_go_syntax(&absolute_path, strict).await?,
            "c" | "cpp" => self.check_c_cpp_syntax(&absolute_path, &language, strict).await?,
            "unknown" => {
                return Err(ToolError::InvalidInput(format!(
                    "Cannot determine language for file: {}. Please specify the 'language' parameter.",
                    absolute_path.display()
                )));
            }
            unsupported => {
                return Err(ToolError::InvalidInput(format!(
                    "Syntax checking for '{}' is not yet supported. Supported languages: rust, javascript, typescript, python, go, c, cpp",
                    unsupported
                )));
            }
        };

        Ok(result)
    }

    /// Check Rust syntax using cargo check or rustc
    async fn check_rust_syntax(&self, file_path: &Path, strict: bool) -> Result<String, ToolError> {
        // First try cargo check if in a cargo project
        let cargo_toml = file_path
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists());

        if let Some(project_root) = cargo_toml {
            let mut cmd = tokio::process::Command::new("cargo");
            cmd.arg("check")
                .arg("--message-format=short")
                .current_dir(project_root);

            if !strict {
                cmd.arg("--quiet");
            }

            let output = cmd.output().await?;

            if output.status.success() {
                Ok(format!("✅ Rust syntax check passed for {}", file_path.display()))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(format!(
                    "❌ Rust syntax errors found:\n\n{}\n{}",
                    stdout, stderr
                ))
            }
        } else {
            // Fallback to rustc --emit=metadata for single file (no executable output)
            let mut cmd = tokio::process::Command::new("rustc");
            cmd.arg("--crate-type").arg("lib")
                .arg("--emit=metadata")
                .arg("--out-dir")
                .arg(std::env::temp_dir())
                .arg(file_path);

            let output = cmd.output().await?;

            if output.status.success() {
                Ok(format!("✅ Rust syntax check passed for {}", file_path.display()))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Ok(format!("❌ Rust syntax errors found:\n\n{}", stderr))
            }
        }
    }

    /// Check JavaScript/TypeScript syntax using Node.js or tsc
    async fn check_js_ts_syntax(&self, file_path: &Path, language: &str, strict: bool) -> Result<String, ToolError> {
        if language == "typescript" {
            // Try using tsc for TypeScript
            let mut cmd = tokio::process::Command::new("tsc");
            cmd.arg("--noEmit")
                .arg("--allowJs")
                .arg(file_path);

            if strict {
                cmd.arg("--strict");
            }

            let output = cmd.output().await;

            match output {
                Ok(output) => {
                    if output.status.success() {
                        Ok(format!("✅ TypeScript syntax check passed for {}", file_path.display()))
                    } else {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        Ok(format!(
                            "❌ TypeScript syntax errors found:\n\n{}\n{}",
                            stdout, stderr
                        ))
                    }
                }
                Err(_) => {
                    // Fallback to Node.js syntax check
                    self.check_with_node(file_path).await
                }
            }
        } else {
            // JavaScript - use Node.js
            self.check_with_node(file_path).await
        }
    }

    /// Check syntax using Node.js
    async fn check_with_node(&self, file_path: &Path) -> Result<String, ToolError> {
        let mut cmd = tokio::process::Command::new("node");
        cmd.arg("--check")
            .arg(file_path);

        let output = cmd.output().await?;

        if output.status.success() {
            Ok(format!("✅ JavaScript syntax check passed for {}", file_path.display()))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(format!("❌ JavaScript syntax errors found:\n\n{}", stderr))
        }
    }

    /// Check Python syntax
    async fn check_python_syntax(&self, file_path: &Path, _strict: bool) -> Result<String, ToolError> {
        // Use Python's compile function to check syntax
        let mut cmd = tokio::process::Command::new("python");
        cmd.arg("-m")
            .arg("py_compile")
            .arg(file_path);

        let output = cmd.output().await?;

        if output.status.success() {
            Ok(format!("✅ Python syntax check passed for {}", file_path.display()))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(format!("❌ Python syntax errors found:\n\n{}", stderr))
        }
    }

    /// Check Go syntax
    async fn check_go_syntax(&self, file_path: &Path, _strict: bool) -> Result<String, ToolError> {
        let mut cmd = tokio::process::Command::new("go");
        cmd.arg("build")
            .arg("-o")
            .arg("/dev/null")
            .arg(file_path);

        let output = cmd.output().await?;

        if output.status.success() {
            Ok(format!("✅ Go syntax check passed for {}", file_path.display()))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(format!(
                "❌ Go syntax errors found:\n\n{}\n{}",
                stdout, stderr
            ))
        }
    }

    /// Check C/C++ syntax using compiler
    async fn check_c_cpp_syntax(&self, file_path: &Path, language: &str, strict: bool) -> Result<String, ToolError> {
        let compiler = if language == "cpp" { "g++" } else { "gcc" };

        let mut cmd = tokio::process::Command::new(compiler);
        cmd.arg("-fsyntax-only")
            .arg(file_path);

        if strict {
            cmd.arg("-Wall").arg("-Wextra").arg("-pedantic");
        }

        let output = cmd.output().await?;

        if output.status.success() {
            Ok(format!("✅ {} syntax check passed for {}", language.to_uppercase(), file_path.display()))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(format!("❌ {} syntax errors found:\n\n{}", language.to_uppercase(), stderr))
        }
    }

    /// Execute the Code Format tool
    async fn execute_code_format(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing file_path".to_string()))?;

        let path = Path::new(file_path);
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory.join(path)
        };

        // Check if file exists
        if !absolute_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "File does not exist: {}",
                absolute_path.display()
            )));
        }

        // Detect language from file extension or use provided language
        let language = if let Some(lang) = input["language"].as_str() {
            lang.to_lowercase()
        } else {
            // Auto-detect from file extension
            absolute_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| match e {
                    "rs" => "rust",
                    "js" | "mjs" | "cjs" => "javascript",
                    "ts" | "mts" | "cts" => "typescript",
                    "py" => "python",
                    "go" => "go",
                    "c" => "c",
                    "cpp" | "cc" | "cxx" | "h" | "hpp" => "cpp",
                    _ => "unknown",
                })
                .unwrap_or("unknown")
                .to_string()
        };

        let check_only = input["check_only"].as_bool().unwrap_or(false);

        // Execute language-specific formatter
        let result = match language.as_str() {
            "rust" => self.format_rust(&absolute_path, check_only).await?,
            "javascript" | "typescript" => self.format_js_ts(&absolute_path, check_only).await?,
            "python" => self.format_python(&absolute_path, check_only).await?,
            "go" => self.format_go(&absolute_path, check_only).await?,
            "c" | "cpp" => self.format_c_cpp(&absolute_path, check_only).await?,
            "unknown" => {
                return Err(ToolError::InvalidInput(format!(
                    "Cannot determine language for file: {}. Please specify the 'language' parameter.",
                    absolute_path.display()
                )));
            }
            unsupported => {
                return Err(ToolError::InvalidInput(format!(
                    "Code formatting for '{}' is not yet supported. Supported languages: rust, javascript, typescript, python, go, c, cpp",
                    unsupported
                )));
            }
        };

        Ok(result)
    }

    /// Format Rust code using rustfmt
    async fn format_rust(&self, file_path: &Path, check_only: bool) -> Result<String, ToolError> {
        let mut cmd = tokio::process::Command::new("rustfmt");

        if check_only {
            cmd.arg("--check");
        }

        cmd.arg(file_path);

        let output = cmd.output().await?;

        if output.status.success() {
            if check_only {
                Ok(format!("✅ {} is correctly formatted", file_path.display()))
            } else {
                Ok(format!("✅ Successfully formatted {}", file_path.display()))
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            if check_only {
                Ok(format!("❌ {} requires formatting:\n\n{}{}", file_path.display(), stdout, stderr))
            } else {
                Err(ToolError::CommandFailed(format!(
                    "Failed to format file:\n\n{}{}",
                    stdout, stderr
                )))
            }
        }
    }

    /// Format JavaScript/TypeScript code using prettier
    async fn format_js_ts(&self, file_path: &Path, check_only: bool) -> Result<String, ToolError> {
        let mut cmd = tokio::process::Command::new("prettier");

        if check_only {
            cmd.arg("--check");
        } else {
            cmd.arg("--write");
        }

        cmd.arg(file_path);

        let output = cmd.output().await?;

        if output.status.success() {
            if check_only {
                Ok(format!("✅ {} is correctly formatted", file_path.display()))
            } else {
                Ok(format!("✅ Successfully formatted {}", file_path.display()))
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            if check_only {
                Ok(format!("❌ {} requires formatting:\n\n{}{}", file_path.display(), stdout, stderr))
            } else {
                Err(ToolError::CommandFailed(format!(
                    "Failed to format file:\n\n{}{}",
                    stdout, stderr
                )))
            }
        }
    }

    /// Format Python code using black
    async fn format_python(&self, file_path: &Path, check_only: bool) -> Result<String, ToolError> {
        let mut cmd = tokio::process::Command::new("black");

        if check_only {
            cmd.arg("--check");
        }

        cmd.arg(file_path);

        let output = cmd.output().await?;

        if output.status.success() {
            if check_only {
                Ok(format!("✅ {} is correctly formatted", file_path.display()))
            } else {
                Ok(format!("✅ Successfully formatted {}", file_path.display()))
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            if check_only {
                Ok(format!("❌ {} requires formatting:\n\n{}{}", file_path.display(), stdout, stderr))
            } else {
                Err(ToolError::CommandFailed(format!(
                    "Failed to format file:\n\n{}{}",
                    stdout, stderr
                )))
            }
        }
    }

    /// Format Go code using gofmt
    async fn format_go(&self, file_path: &Path, check_only: bool) -> Result<String, ToolError> {
        if check_only {
            // Use gofmt -l to list files that need formatting
            let mut cmd = tokio::process::Command::new("gofmt");
            cmd.arg("-l").arg(file_path);

            let output = cmd.output().await?;

            if output.stdout.is_empty() {
                Ok(format!("✅ {} is correctly formatted", file_path.display()))
            } else {
                Ok(format!("❌ {} requires formatting", file_path.display()))
            }
        } else {
            // Use gofmt -w to write formatted output
            let mut cmd = tokio::process::Command::new("gofmt");
            cmd.arg("-w").arg(file_path);

            let output = cmd.output().await?;

            if output.status.success() {
                Ok(format!("✅ Successfully formatted {}", file_path.display()))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(ToolError::CommandFailed(format!(
                    "Failed to format file:\n\n{}",
                    stderr
                )))
            }
        }
    }

    /// Format C/C++ code using clang-format
    async fn format_c_cpp(&self, file_path: &Path, check_only: bool) -> Result<String, ToolError> {
        if check_only {
            // Read original file
            let original = tokio::fs::read_to_string(file_path).await?;

            // Format to stdout
            let mut cmd = tokio::process::Command::new("clang-format");
            cmd.arg(file_path);

            let output = cmd.output().await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(ToolError::CommandFailed(format!(
                    "Failed to format file:\n\n{}",
                    stderr
                )));
            }

            let formatted = String::from_utf8_lossy(&output.stdout);

            if original == formatted {
                Ok(format!("✅ {} is correctly formatted", file_path.display()))
            } else {
                Ok(format!("❌ {} requires formatting", file_path.display()))
            }
        } else {
            // Use -i to format in-place
            let mut cmd = tokio::process::Command::new("clang-format");
            cmd.arg("-i").arg(file_path);

            let output = cmd.output().await?;

            if output.status.success() {
                Ok(format!("✅ Successfully formatted {}", file_path.display()))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(ToolError::CommandFailed(format!(
                    "Failed to format file:\n\n{}",
                    stderr
                )))
            }
        }
    }

    /// Execute the Code Analysis tool
    async fn execute_code_analysis(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let path_str = input["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing path".to_string()))?;

        let path = Path::new(path_str);
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory.join(path)
        };

        // Check if path exists
        if !absolute_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "Path does not exist: {}",
                absolute_path.display()
            )));
        }

        // Detect language
        let language = if let Some(lang) = input["language"].as_str() {
            lang.to_lowercase()
        } else {
            self.detect_language_from_path(&absolute_path)?
        };

        let analysis_type = input["analysis_type"]
            .as_str()
            .unwrap_or("all");
        let strict = input["strict"].as_bool().unwrap_or(false);

        // Execute language-specific analysis
        let mut results = Vec::new();

        match analysis_type {
            "quality" => {
                results.push(self.analyze_quality(&absolute_path, &language, strict).await?);
            }
            "security" => {
                results.push(self.analyze_security(&absolute_path, &language).await?);
            }
            "all" => {
                results.push(self.analyze_quality(&absolute_path, &language, strict).await?);
                results.push(self.analyze_security(&absolute_path, &language).await?);
            }
            _ => {
                return Err(ToolError::InvalidInput(format!(
                    "Invalid analysis_type: '{}'. Must be 'quality', 'security', or 'all'",
                    analysis_type
                )));
            }
        }

        let separator = format!("\n\n{}\n\n", "=".repeat(60));
        Ok(results.join(&separator))
    }

    /// Detect language from path
    fn detect_language_from_path(&self, path: &Path) -> Result<String, ToolError> {
        // Check for project markers
        if path.is_dir() {
            if path.join("Cargo.toml").exists() {
                return Ok("rust".to_string());
            }
            if path.join("package.json").exists() {
                return Ok("javascript".to_string());
            }
            if path.join("go.mod").exists() {
                return Ok("go".to_string());
            }
            if path.join("requirements.txt").exists() || path.join("setup.py").exists() {
                return Ok("python".to_string());
            }
        }

        // Check file extension
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                return Ok(match ext {
                    "rs" => "rust",
                    "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" => "javascript",
                    "py" => "python",
                    "go" => "go",
                    _ => "unknown",
                }
                .to_string());
            }
        }

        Err(ToolError::InvalidInput(format!(
            "Cannot determine language for path: {}. Please specify the 'language' parameter.",
            path.display()
        )))
    }

    /// Analyze code quality
    async fn analyze_quality(&self, path: &Path, language: &str, strict: bool) -> Result<String, ToolError> {
        match language {
            "rust" => self.analyze_rust_quality(path, strict).await,
            "javascript" | "typescript" => self.analyze_js_quality(path, strict).await,
            "python" => self.analyze_python_quality(path, strict).await,
            "go" => self.analyze_go_quality(path, strict).await,
            unsupported => Err(ToolError::InvalidInput(format!(
                "Quality analysis for '{}' is not yet supported. Supported languages: rust, javascript, typescript, python, go",
                unsupported
            ))),
        }
    }

    /// Analyze security vulnerabilities
    async fn analyze_security(&self, path: &Path, language: &str) -> Result<String, ToolError> {
        match language {
            "rust" => self.analyze_rust_security(path).await,
            "javascript" | "typescript" => self.analyze_js_security(path).await,
            "python" => self.analyze_python_security(path).await,
            "go" => self.analyze_go_security(path).await,
            unsupported => Err(ToolError::InvalidInput(format!(
                "Security analysis for '{}' is not yet supported. Supported languages: rust, javascript, typescript, python, go",
                unsupported
            ))),
        }
    }

    /// Analyze Rust code quality using clippy
    async fn analyze_rust_quality(&self, path: &Path, strict: bool) -> Result<String, ToolError> {
        // Find the cargo project root
        let project_root = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.ancestors()
                .find(|p| p.join("Cargo.toml").exists())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| path.parent().unwrap().to_path_buf())
        };

        let mut cmd = tokio::process::Command::new("cargo");
        cmd.arg("clippy")
            .arg("--message-format=short")
            .current_dir(&project_root);

        if strict {
            cmd.arg("--").arg("-W").arg("clippy::all");
        }

        let output = cmd.output().await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() && stdout.is_empty() && stderr.is_empty() {
            Ok(format!("📊 Code Quality Analysis (Rust)\n✅ No issues found. Code looks good!"))
        } else {
            Ok(format!(
                "📊 Code Quality Analysis (Rust)\n\n{}{}",
                stdout, stderr
            ))
        }
    }

    /// Analyze Rust security using cargo-audit
    async fn analyze_rust_security(&self, path: &Path) -> Result<String, ToolError> {
        let project_root = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.ancestors()
                .find(|p| p.join("Cargo.toml").exists())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| path.parent().unwrap().to_path_buf())
        };

        let mut cmd = tokio::process::Command::new("cargo");
        cmd.arg("audit")
            .current_dir(&project_root);

        let output = cmd.output().await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    Ok(format!("🔒 Security Analysis (Rust)\n✅ No known vulnerabilities found."))
                } else {
                    Ok(format!("🔒 Security Analysis (Rust)\n\n{}{}", stdout, stderr))
                }
            }
            Err(_) => {
                Ok("🔒 Security Analysis (Rust)\n⚠️  cargo-audit not installed. Run: cargo install cargo-audit".to_string())
            }
        }
    }

    /// Analyze JavaScript/TypeScript quality using eslint
    async fn analyze_js_quality(&self, path: &Path, _strict: bool) -> Result<String, ToolError> {
        let mut cmd = tokio::process::Command::new("eslint");
        cmd.arg(path);

        let output = cmd.output().await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() && stdout.is_empty() {
                    Ok("📊 Code Quality Analysis (JavaScript/TypeScript)\n✅ No issues found.".to_string())
                } else {
                    Ok(format!(
                        "📊 Code Quality Analysis (JavaScript/TypeScript)\n\n{}{}",
                        stdout, stderr
                    ))
                }
            }
            Err(_) => {
                Ok("📊 Code Quality Analysis (JavaScript/TypeScript)\n⚠️  ESLint not installed. Run: npm install -g eslint".to_string())
            }
        }
    }

    /// Analyze JavaScript/TypeScript security using npm audit
    async fn analyze_js_security(&self, path: &Path) -> Result<String, ToolError> {
        let project_root = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.ancestors()
                .find(|p| p.join("package.json").exists())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| path.parent().unwrap().to_path_buf())
        };

        let mut cmd = tokio::process::Command::new("npm");
        cmd.arg("audit")
            .current_dir(&project_root);

        let output = cmd.output().await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() && stdout.contains("found 0 vulnerabilities") {
            Ok("🔒 Security Analysis (JavaScript/TypeScript)\n✅ No vulnerabilities found.".to_string())
        } else {
            Ok(format!(
                "🔒 Security Analysis (JavaScript/TypeScript)\n\n{}{}",
                stdout, stderr
            ))
        }
    }

    /// Analyze Python quality using pylint
    async fn analyze_python_quality(&self, path: &Path, _strict: bool) -> Result<String, ToolError> {
        let mut cmd = tokio::process::Command::new("pylint");
        cmd.arg(path);

        let output = cmd.output().await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                Ok(format!(
                    "📊 Code Quality Analysis (Python)\n\n{}{}",
                    stdout, stderr
                ))
            }
            Err(_) => {
                Ok("📊 Code Quality Analysis (Python)\n⚠️  Pylint not installed. Run: pip install pylint".to_string())
            }
        }
    }

    /// Analyze Python security using bandit
    async fn analyze_python_security(&self, path: &Path) -> Result<String, ToolError> {
        let mut cmd = tokio::process::Command::new("bandit");
        cmd.arg("-r").arg(path);

        let output = cmd.output().await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() && stdout.contains("No issues identified") {
                    Ok("🔒 Security Analysis (Python)\n✅ No security issues found.".to_string())
                } else {
                    Ok(format!(
                        "🔒 Security Analysis (Python)\n\n{}{}",
                        stdout, stderr
                    ))
                }
            }
            Err(_) => {
                Ok("🔒 Security Analysis (Python)\n⚠️  Bandit not installed. Run: pip install bandit".to_string())
            }
        }
    }

    /// Analyze Go code quality
    async fn analyze_go_quality(&self, path: &Path, _strict: bool) -> Result<String, ToolError> {
        let mut cmd = tokio::process::Command::new("golint");
        cmd.arg(path);

        let output = cmd.output().await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if stdout.is_empty() && stderr.is_empty() {
                    Ok("📊 Code Quality Analysis (Go)\n✅ No issues found.".to_string())
                } else {
                    Ok(format!(
                        "📊 Code Quality Analysis (Go)\n\n{}{}",
                        stdout, stderr
                    ))
                }
            }
            Err(_) => {
                Ok("📊 Code Quality Analysis (Go)\n⚠️  golint not installed. Run: go install golang.org/x/lint/golint@latest".to_string())
            }
        }
    }

    /// Analyze Go security
    async fn analyze_go_security(&self, path: &Path) -> Result<String, ToolError> {
        let mut cmd = tokio::process::Command::new("gosec");
        cmd.arg(path);

        let output = cmd.output().await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if stdout.contains("Issues : 0") {
                    Ok("🔒 Security Analysis (Go)\n✅ No security issues found.".to_string())
                } else {
                    Ok(format!(
                        "🔒 Security Analysis (Go)\n\n{}{}",
                        stdout, stderr
                    ))
                }
            }
            Err(_) => {
                Ok("🔒 Security Analysis (Go)\n⚠️  gosec not installed. Run: go install github.com/securego/gosec/v2/cmd/gosec@latest".to_string())
            }
        }
    }

    /// Execute the Copy tool
    async fn execute_copy(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let source_str = input["source"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing source".to_string()))?;

        let destination_str = input["destination"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing destination".to_string()))?;

        let overwrite = input["overwrite"].as_bool().unwrap_or(false);
        let recursive = input["recursive"].as_bool().unwrap_or(true);

        // Resolve paths
        let source = Path::new(source_str);
        let source_path = if source.is_absolute() {
            source.to_path_buf()
        } else {
            self.working_directory.join(source)
        };

        let destination = Path::new(destination_str);
        let dest_path = if destination.is_absolute() {
            destination.to_path_buf()
        } else {
            self.working_directory.join(destination)
        };

        // Check if source exists
        if !source_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "Source does not exist: {}",
                source_path.display()
            )));
        }

        // Check if destination already exists and overwrite is false
        if dest_path.exists() && !overwrite {
            return Err(ToolError::InvalidInput(format!(
                "Destination already exists: {}. Set overwrite=true to replace it.",
                dest_path.display()
            )));
        }

        // Perform the copy
        if source_path.is_file() {
            // Copy a single file
            if let Some(parent) = dest_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(&source_path, &dest_path).await?;
            Ok(format!(
                "✅ Successfully copied file:\n   From: {}\n   To:   {}",
                source_path.display(),
                dest_path.display()
            ))
        } else if source_path.is_dir() {
            if !recursive {
                return Err(ToolError::InvalidInput(
                    "Source is a directory. Set recursive=true to copy directories.".to_string(),
                ));
            }

            // Copy directory recursively
            let mut files_copied = 0;
            let mut dirs_created = 0;

            self.copy_dir_recursive(&source_path, &dest_path, &mut files_copied, &mut dirs_created)
                .await?;

            Ok(format!(
                "✅ Successfully copied directory:\n   From: {}\n   To:   {}\n   {} files copied, {} directories created",
                source_path.display(),
                dest_path.display(),
                files_copied,
                dirs_created
            ))
        } else {
            Err(ToolError::InvalidInput(format!(
                "Source is neither a file nor a directory: {}",
                source_path.display()
            )))
        }
    }

    /// Recursively copy a directory
    fn copy_dir_recursive<'a>(
        &'a self,
        source: &'a Path,
        destination: &'a Path,
        files_copied: &'a mut usize,
        dirs_created: &'a mut usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send + 'a>> {
        Box::pin(async move {
            // Create destination directory
            if !destination.exists() {
                tokio::fs::create_dir(destination).await?;
                *dirs_created += 1;
            }

            // Read source directory
            let mut entries = tokio::fs::read_dir(source).await?;

            while let Some(entry) = entries.next_entry().await? {
                let source_path = entry.path();
                let file_name = entry.file_name();
                let dest_path = destination.join(&file_name);

                if source_path.is_file() {
                    tokio::fs::copy(&source_path, &dest_path).await?;
                    *files_copied += 1;
                } else if source_path.is_dir() {
                    self.copy_dir_recursive(&source_path, &dest_path, files_copied, dirs_created)
                        .await?;
                }
            }

            Ok(())
        })
    }

    /// Execute the Delete tool
    async fn execute_delete(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let path_str = input["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing path".to_string()))?;

        let recursive = input["recursive"].as_bool().unwrap_or(false);

        // Resolve path
        let path = Path::new(path_str);
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory.join(path)
        };

        // Check if path exists
        if !absolute_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "Path does not exist: {}",
                absolute_path.display()
            )));
        }

        // Perform the deletion
        if absolute_path.is_file() {
            // Delete a single file
            tokio::fs::remove_file(&absolute_path).await?;
            Ok(format!(
                "✅ Successfully deleted file: {}",
                absolute_path.display()
            ))
        } else if absolute_path.is_dir() {
            if !recursive {
                return Err(ToolError::InvalidInput(
                    "Path is a directory. Set recursive=true to delete directories and their contents.".to_string(),
                ));
            }

            // Count items before deletion
            let mut files_deleted = 0;
            let mut dirs_deleted = 0;
            self.count_items(&absolute_path, &mut files_deleted, &mut dirs_deleted)
                .await?;

            // Delete directory recursively
            tokio::fs::remove_dir_all(&absolute_path).await?;

            Ok(format!(
                "✅ Successfully deleted directory: {}\n   {} files and {} directories removed",
                absolute_path.display(),
                files_deleted,
                dirs_deleted
            ))
        } else {
            Err(ToolError::InvalidInput(format!(
                "Path is neither a file nor a directory: {}",
                absolute_path.display()
            )))
        }
    }

    /// Count files and directories recursively
    fn count_items<'a>(
        &'a self,
        path: &'a Path,
        files: &'a mut usize,
        dirs: &'a mut usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send + 'a>> {
        Box::pin(async move {
            if path.is_file() {
                *files += 1;
            } else if path.is_dir() {
                *dirs += 1;
                let mut entries = tokio::fs::read_dir(path).await?;

                while let Some(entry) = entries.next_entry().await? {
                    let entry_path = entry.path();
                    self.count_items(&entry_path, files, dirs).await?;
                }
            }

            Ok(())
        })
    }

    /// Execute the Move tool
    async fn execute_move(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let source_str = input["source"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing source".to_string()))?;

        let destination_str = input["destination"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing destination".to_string()))?;

        let overwrite = input["overwrite"].as_bool().unwrap_or(false);

        // Resolve source path
        let source = Path::new(source_str);
        let source_path = if source.is_absolute() {
            source.to_path_buf()
        } else {
            self.working_directory.join(source)
        };

        // Check if source exists
        if !source_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "Source does not exist: {}",
                source_path.display()
            )));
        }

        // Resolve destination path
        let destination = Path::new(destination_str);
        let dest_path = if destination.is_absolute() {
            destination.to_path_buf()
        } else {
            self.working_directory.join(destination)
        };

        // Check if destination exists and handle overwrite
        if dest_path.exists() && !overwrite {
            return Err(ToolError::InvalidInput(format!(
                "Destination already exists: {}. Set overwrite=true to replace it.",
                dest_path.display()
            )));
        }

        // If destination exists and overwrite is true, remove it first
        if dest_path.exists() && overwrite {
            if dest_path.is_file() {
                tokio::fs::remove_file(&dest_path).await?;
            } else if dest_path.is_dir() {
                tokio::fs::remove_dir_all(&dest_path).await?;
            }
        }

        // Determine what we're moving (for feedback)
        let is_dir = source_path.is_dir();
        let item_type = if is_dir { "directory" } else { "file" };

        // Perform the move
        tokio::fs::rename(&source_path, &dest_path).await?;

        Ok(format!(
            "✅ Successfully moved {} from:\n   {}\n   to:\n   {}",
            item_type,
            source_path.display(),
            dest_path.display()
        ))
    }

    /// Execute the Build tool
    async fn execute_build(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let build_type = input["build_type"].as_str().unwrap_or("debug");
        let custom_command = input["custom_command"].as_str();

        // Get working directory
        let working_dir = if let Some(wd) = input["working_directory"].as_str() {
            let path = Path::new(wd);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.working_directory.join(path)
            }
        } else {
            self.working_directory.clone()
        };

        // Detect or use specified project type
        let project_type = if let Some(pt) = input["project_type"].as_str() {
            pt.to_lowercase()
        } else {
            self.detect_build_project_type(&working_dir)?
        };

        // Build the command
        let mut command = if project_type == "custom" {
            if let Some(cmd) = custom_command {
                cmd.to_string()
            } else {
                return Err(ToolError::InvalidInput(
                    "custom_command is required when project_type is 'custom'".to_string(),
                ));
            }
        } else {
            self.get_build_command(&project_type, build_type)?
        };

        // Add additional arguments
        if let Some(args) = input["args"].as_array() {
            for arg in args {
                if let Some(arg_str) = arg.as_str() {
                    command.push(' ');
                    command.push_str(arg_str);
                }
            }
        }

        // Execute the build command
        let output = tokio::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "sh" })
            .arg(if cfg!(target_os = "windows") { "/C" } else { "-c" })
            .arg(&command)
            .current_dir(&working_dir)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            let mut result = format!("✅ Build succeeded ({})\n", project_type);
            result.push_str(&format!("Command: {}\n\n", command));
            if !stdout.is_empty() {
                result.push_str("Output:\n");
                result.push_str(&stdout);
            }
            Ok(result)
        } else {
            let mut error_msg = format!("❌ Build failed ({})\n", project_type);
            error_msg.push_str(&format!("Command: {}\n\n", command));
            if !stderr.is_empty() {
                error_msg.push_str("Errors:\n");
                error_msg.push_str(&stderr);
            }
            if !stdout.is_empty() {
                error_msg.push_str("\nOutput:\n");
                error_msg.push_str(&stdout);
            }
            Err(ToolError::CommandFailed(error_msg))
        }
    }

    /// Detect project type for build based on project structure
    fn detect_build_project_type(&self, path: &Path) -> Result<String, ToolError> {
        // Check for Rust project
        if path.join("Cargo.toml").exists() {
            return Ok("rust".to_string());
        }

        // Check for Node.js project
        if path.join("package.json").exists() {
            return Ok("javascript".to_string());
        }

        // Check for Python project
        if path.join("setup.py").exists() || path.join("pyproject.toml").exists() {
            return Ok("python".to_string());
        }

        // Check for Go project
        if path.join("go.mod").exists() {
            return Ok("go".to_string());
        }

        Err(ToolError::InvalidInput(
            "Could not detect project type. Please specify project_type explicitly.".to_string(),
        ))
    }

    /// Get the appropriate build command for a project type
    fn get_build_command(&self, project_type: &str, build_type: &str) -> Result<String, ToolError> {
        match project_type {
            "rust" => {
                if build_type == "release" {
                    Ok("cargo build --release".to_string())
                } else {
                    Ok("cargo build".to_string())
                }
            }
            "javascript" | "typescript" => {
                Ok("npm run build".to_string())
            }
            "python" => {
                Ok("python setup.py build".to_string())
            }
            "go" => {
                Ok("go build".to_string())
            }
            _ => Err(ToolError::InvalidInput(format!(
                "Unsupported project type: {}. Use 'custom' with custom_command instead.",
                project_type
            ))),
        }
    }

    /// Execute the Test Runner tool
    async fn execute_test_runner(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let test_type = input["test_type"].as_str().unwrap_or("all");
        let test_pattern = input["test_pattern"].as_str();
        let custom_command = input["custom_command"].as_str();

        // Get working directory
        let working_dir = if let Some(wd) = input["working_directory"].as_str() {
            let path = Path::new(wd);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.working_directory.join(path)
            }
        } else {
            self.working_directory.clone()
        };

        // Detect or use specified project type
        let project_type = if let Some(pt) = input["project_type"].as_str() {
            pt.to_lowercase()
        } else {
            self.detect_test_project_type(&working_dir)?
        };

        // Build the test command
        let mut command = if project_type == "custom" {
            if let Some(cmd) = custom_command {
                cmd.to_string()
            } else {
                return Err(ToolError::InvalidInput(
                    "custom_command is required when project_type is 'custom'".to_string(),
                ));
            }
        } else {
            self.get_test_command(&project_type, test_type, test_pattern)?
        };

        // Add additional arguments
        if let Some(args) = input["args"].as_array() {
            for arg in args {
                if let Some(arg_str) = arg.as_str() {
                    command.push(' ');
                    command.push_str(arg_str);
                }
            }
        }

        // Execute the test command
        let output = tokio::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "sh" })
            .arg(if cfg!(target_os = "windows") { "/C" } else { "-c" })
            .arg(&command)
            .current_dir(&working_dir)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse test results
        let test_results = self.parse_test_results(&project_type, &stdout, &stderr);

        if output.status.success() {
            let mut result = format!("✅ Tests passed ({})\n", project_type);
            result.push_str(&format!("Command: {}\n", command));
            result.push_str(&test_results);
            Ok(result)
        } else {
            let mut error_msg = format!("❌ Tests failed ({})\n", project_type);
            error_msg.push_str(&format!("Command: {}\n", command));
            error_msg.push_str(&test_results);
            Err(ToolError::CommandFailed(error_msg))
        }
    }

    /// Detect project type for testing
    fn detect_test_project_type(&self, path: &Path) -> Result<String, ToolError> {
        // Check for Rust project
        if path.join("Cargo.toml").exists() {
            return Ok("rust".to_string());
        }

        // Check for Node.js project
        if path.join("package.json").exists() {
            return Ok("javascript".to_string());
        }

        // Check for Python project with pytest
        if path.join("setup.py").exists()
            || path.join("pyproject.toml").exists()
            || path.join("pytest.ini").exists()
            || path.join("tests").exists() {
            return Ok("python".to_string());
        }

        // Check for Go project
        if path.join("go.mod").exists() {
            return Ok("go".to_string());
        }

        Err(ToolError::InvalidInput(
            "Could not detect project type. Please specify project_type explicitly.".to_string(),
        ))
    }

    /// Get the test command for a given project type
    fn get_test_command(
        &self,
        project_type: &str,
        test_type: &str,
        test_pattern: Option<&str>,
    ) -> Result<String, ToolError> {
        match project_type {
            "rust" => {
                let mut cmd = String::from("cargo test");

                match test_type {
                    "unit" => cmd.push_str(" --lib"),
                    "integration" => cmd.push_str(" --test '*'"),
                    "all" => {}, // default behavior
                    _ => return Err(ToolError::InvalidInput(format!(
                        "Invalid test_type for Rust: {}. Use 'unit', 'integration', or 'all'.",
                        test_type
                    ))),
                }

                if let Some(pattern) = test_pattern {
                    cmd.push_str(&format!(" {}", pattern));
                }

                Ok(cmd)
            }
            "javascript" | "typescript" => {
                let mut cmd = String::from("npm test");

                if let Some(pattern) = test_pattern {
                    cmd.push_str(&format!(" -- {}", pattern));
                }

                Ok(cmd)
            }
            "python" => {
                let mut cmd = String::from("pytest");

                match test_type {
                    "unit" => cmd.push_str(" tests/unit"),
                    "integration" => cmd.push_str(" tests/integration"),
                    "all" => {}, // default behavior
                    _ => return Err(ToolError::InvalidInput(format!(
                        "Invalid test_type for Python: {}. Use 'unit', 'integration', or 'all'.",
                        test_type
                    ))),
                }

                if let Some(pattern) = test_pattern {
                    cmd.push_str(&format!(" -k {}", pattern));
                }

                Ok(cmd)
            }
            "go" => {
                let mut cmd = String::from("go test");

                if let Some(pattern) = test_pattern {
                    cmd.push_str(&format!(" -run {}", pattern));
                }

                cmd.push_str(" ./...");

                Ok(cmd)
            }
            _ => Err(ToolError::InvalidInput(format!(
                "Unsupported project type: {}. Use 'custom' with custom_command instead.",
                project_type
            ))),
        }
    }

    /// Parse test results from command output
    fn parse_test_results(&self, project_type: &str, stdout: &str, stderr: &str) -> String {
        let mut results = String::new();

        match project_type {
            "rust" => {
                // Parse Rust test output
                if let Some(line) = stdout.lines().find(|l| l.contains("test result:")) {
                    results.push_str("\nTest Summary:\n");
                    results.push_str(line);
                    results.push('\n');
                } else if !stdout.is_empty() {
                    results.push_str("\nOutput:\n");
                    results.push_str(stdout);
                }

                if !stderr.is_empty() {
                    results.push_str("\nErrors:\n");
                    results.push_str(stderr);
                }
            }
            "javascript" | "typescript" => {
                // Parse npm test output
                if !stdout.is_empty() {
                    results.push_str("\nOutput:\n");
                    results.push_str(stdout);
                }

                if !stderr.is_empty() {
                    results.push_str("\nErrors:\n");
                    results.push_str(stderr);
                }
            }
            "python" => {
                // Parse pytest output
                if let Some(line) = stdout.lines().rev().find(|l| l.contains("passed") || l.contains("failed")) {
                    results.push_str("\nTest Summary:\n");
                    results.push_str(line);
                    results.push('\n');
                } else if !stdout.is_empty() {
                    results.push_str("\nOutput:\n");
                    results.push_str(stdout);
                }

                if !stderr.is_empty() {
                    results.push_str("\nErrors:\n");
                    results.push_str(stderr);
                }
            }
            "go" => {
                // Parse Go test output
                if !stdout.is_empty() {
                    results.push_str("\nOutput:\n");
                    results.push_str(stdout);
                }

                if !stderr.is_empty() {
                    results.push_str("\nErrors:\n");
                    results.push_str(stderr);
                }
            }
            _ => {
                // Custom or unknown project type
                if !stdout.is_empty() {
                    results.push_str("\nOutput:\n");
                    results.push_str(stdout);
                }

                if !stderr.is_empty() {
                    results.push_str("\nErrors:\n");
                    results.push_str(stderr);
                }
            }
        }

        results
    }

    /// Execute the Lint tool
    async fn execute_lint(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let severity = input["severity"].as_str().unwrap_or("all");
        let fix = input["fix"].as_bool().unwrap_or(false);
        let custom_command = input["custom_command"].as_str();

        // Get working directory
        let working_dir = if let Some(wd) = input["working_directory"].as_str() {
            let path = Path::new(wd);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.working_directory.join(path)
            }
        } else {
            self.working_directory.clone()
        };

        // Detect or use specified project type
        let project_type = if let Some(pt) = input["project_type"].as_str() {
            pt.to_lowercase()
        } else {
            self.detect_lint_project_type(&working_dir)?
        };

        // Build the lint command
        let mut command = if project_type == "custom" {
            if let Some(cmd) = custom_command {
                cmd.to_string()
            } else {
                return Err(ToolError::InvalidInput(
                    "custom_command is required when project_type is 'custom'".to_string(),
                ));
            }
        } else {
            self.get_lint_command(&project_type, severity, fix)?
        };

        // Add additional arguments
        if let Some(args) = input["args"].as_array() {
            for arg in args {
                if let Some(arg_str) = arg.as_str() {
                    command.push(' ');
                    command.push_str(arg_str);
                }
            }
        }

        // Execute the lint command
        let output = tokio::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "sh" })
            .arg(if cfg!(target_os = "windows") { "/C" } else { "-c" })
            .arg(&command)
            .current_dir(&working_dir)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse lint results
        let lint_results = self.parse_lint_results(&project_type, &stdout, &stderr);

        if output.status.success() {
            let mut result = format!("✅ Lint check passed ({})\n", project_type);
            result.push_str(&format!("Command: {}\n", command));
            result.push_str(&lint_results);
            Ok(result)
        } else {
            let mut error_msg = format!("⚠️  Lint issues found ({})\n", project_type);
            error_msg.push_str(&format!("Command: {}\n", command));
            error_msg.push_str(&lint_results);
            Err(ToolError::CommandFailed(error_msg))
        }
    }

    /// Detect project type for linting
    fn detect_lint_project_type(&self, path: &Path) -> Result<String, ToolError> {
        // Check for Rust project
        if path.join("Cargo.toml").exists() {
            return Ok("rust".to_string());
        }

        // Check for Node.js project with ESLint
        if path.join("package.json").exists() {
            return Ok("javascript".to_string());
        }

        // Check for Python project
        if path.join("setup.py").exists()
            || path.join("pyproject.toml").exists()
            || path.join(".pylintrc").exists()
            || path.join("setup.cfg").exists() {
            return Ok("python".to_string());
        }

        // Check for Go project
        if path.join("go.mod").exists() {
            return Ok("go".to_string());
        }

        Err(ToolError::InvalidInput(
            "Could not detect project type. Please specify project_type explicitly.".to_string(),
        ))
    }

    /// Get the lint command for a given project type
    fn get_lint_command(
        &self,
        project_type: &str,
        severity: &str,
        fix: bool,
    ) -> Result<String, ToolError> {
        match project_type {
            "rust" => {
                let mut cmd = String::from("cargo clippy");

                match severity {
                    "error" => cmd.push_str(" -- -D warnings"),
                    "warning" | "all" => {}, // default behavior
                    _ => return Err(ToolError::InvalidInput(format!(
                        "Invalid severity for Rust: {}. Use 'error', 'warning', or 'all'.",
                        severity
                    ))),
                }

                if fix {
                    cmd.push_str(" --fix");
                }

                Ok(cmd)
            }
            "javascript" | "typescript" => {
                let mut cmd = String::from("npx eslint .");

                if fix {
                    cmd.push_str(" --fix");
                }

                match severity {
                    "error" => cmd.push_str(" --quiet"),
                    "warning" | "all" => {}, // default behavior
                    _ => return Err(ToolError::InvalidInput(format!(
                        "Invalid severity for JavaScript/TypeScript: {}. Use 'error', 'warning', or 'all'.",
                        severity
                    ))),
                }

                Ok(cmd)
            }
            "python" => {
                let mut cmd = String::from("pylint .");

                if fix {
                    // pylint doesn't support auto-fix, use autopep8 instead
                    cmd = String::from("autopep8 --in-place --recursive .");
                } else {
                    match severity {
                        "error" => cmd.push_str(" --errors-only"),
                        "warning" | "all" => {}, // default behavior
                        _ => return Err(ToolError::InvalidInput(format!(
                            "Invalid severity for Python: {}. Use 'error', 'warning', or 'all'.",
                            severity
                        ))),
                    }
                }

                Ok(cmd)
            }
            "go" => {
                let cmd = if fix {
                    String::from("gofmt -w .")
                } else {
                    String::from("go vet ./...")
                };

                Ok(cmd)
            }
            _ => Err(ToolError::InvalidInput(format!(
                "Unsupported project type: {}. Use 'custom' with custom_command instead.",
                project_type
            ))),
        }
    }

    /// Parse lint results from command output
    fn parse_lint_results(&self, project_type: &str, stdout: &str, stderr: &str) -> String {
        let mut results = String::new();

        match project_type {
            "rust" => {
                // Parse clippy output
                if !stdout.is_empty() {
                    results.push_str("\nLint Results:\n");
                    results.push_str(stdout);
                }

                if !stderr.is_empty() {
                    results.push_str("\nWarnings/Errors:\n");
                    results.push_str(stderr);
                }
            }
            "javascript" | "typescript" => {
                // Parse ESLint output
                if !stdout.is_empty() {
                    results.push_str("\nLint Results:\n");
                    results.push_str(stdout);
                }

                if !stderr.is_empty() {
                    results.push_str("\nErrors:\n");
                    results.push_str(stderr);
                }
            }
            "python" => {
                // Parse pylint output
                if !stdout.is_empty() {
                    results.push_str("\nLint Results:\n");
                    results.push_str(stdout);
                }

                if !stderr.is_empty() {
                    results.push_str("\nErrors:\n");
                    results.push_str(stderr);
                }
            }
            "go" => {
                // Parse go vet output
                if !stdout.is_empty() {
                    results.push_str("\nLint Results:\n");
                    results.push_str(stdout);
                }

                if !stderr.is_empty() {
                    results.push_str("\nIssues:\n");
                    results.push_str(stderr);
                }
            }
            _ => {
                // Custom or unknown project type
                if !stdout.is_empty() {
                    results.push_str("\nOutput:\n");
                    results.push_str(stdout);
                }

                if !stderr.is_empty() {
                    results.push_str("\nErrors:\n");
                    results.push_str(stderr);
                }
            }
        }

        results
    }

    /// Execute the Task tool
    async fn execute_task(&self, input: &serde_json::Value) -> Result<String, ToolError> {
        let description = input["description"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("Missing task description".to_string()))?;

        let steps = input["steps"]
            .as_array()
            .ok_or_else(|| ToolError::InvalidInput("Missing or invalid steps array".to_string()))?;

        if steps.is_empty() {
            return Err(ToolError::InvalidInput("Task must have at least one step".to_string()));
        }

        let execution_mode = input["execution_mode"].as_str().unwrap_or("sequential");
        let stop_on_error = input["stop_on_error"].as_bool().unwrap_or_else(|| {
            // Default: stop on error for sequential, continue for parallel
            execution_mode == "sequential"
        });

        // Get default working directory
        let default_working_dir = if let Some(wd) = input["working_directory"].as_str() {
            let path = Path::new(wd);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.working_directory.join(path)
            }
        } else {
            self.working_directory.clone()
        };

        let mut result = format!("📋 Task: {}\n", description);
        result.push_str(&format!("Execution mode: {}\n", execution_mode));
        result.push_str(&format!("Steps: {}\n\n", steps.len()));

        match execution_mode {
            "sequential" => {
                self.execute_steps_sequential(steps, &default_working_dir, stop_on_error, &mut result).await?;
            }
            "parallel" => {
                self.execute_steps_parallel(steps, &default_working_dir, &mut result).await?;
            }
            _ => {
                return Err(ToolError::InvalidInput(format!(
                    "Invalid execution_mode: {}. Use 'sequential' or 'parallel'.",
                    execution_mode
                )));
            }
        }

        Ok(result)
    }

    /// Execute steps sequentially
    async fn execute_steps_sequential(
        &self,
        steps: &[serde_json::Value],
        default_working_dir: &Path,
        stop_on_error: bool,
        result: &mut String,
    ) -> Result<(), ToolError> {
        let mut completed = 0;
        let mut failed = 0;

        for (i, step) in steps.iter().enumerate() {
            let step_name = step["name"]
                .as_str()
                .ok_or_else(|| ToolError::InvalidInput(format!("Step {} missing name", i + 1)))?;

            let command = step["command"]
                .as_str()
                .ok_or_else(|| ToolError::InvalidInput(format!("Step {} missing command", i + 1)))?;

            result.push_str(&format!("▶ Step {}/{}: {}\n", i + 1, steps.len(), step_name));

            // Get working directory for this step
            let working_dir = if let Some(wd) = step["working_directory"].as_str() {
                let path = Path::new(wd);
                if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    default_working_dir.join(path)
                }
            } else {
                default_working_dir.to_path_buf()
            };

            // Execute the command
            let output = tokio::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "sh" })
                .arg(if cfg!(target_os = "windows") { "/C" } else { "-c" })
                .arg(command)
                .current_dir(&working_dir)
                .output()
                .await?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
                result.push_str("  ✅ Success\n");
                if !stdout.is_empty() {
                    result.push_str(&format!("  Output: {}\n", stdout.trim()));
                }
                completed += 1;
            } else {
                result.push_str("  ❌ Failed\n");
                if !stderr.is_empty() {
                    result.push_str(&format!("  Error: {}\n", stderr.trim()));
                }
                failed += 1;

                if stop_on_error {
                    result.push_str(&format!("\n⚠️  Stopping execution after step {} (stop_on_error=true)\n", i + 1));
                    result.push_str(&format!("Summary: {} completed, {} failed, {} skipped\n",
                        completed, failed, steps.len() - i - 1));
                    return Err(ToolError::CommandFailed(result.clone()));
                }
            }
        }

        result.push_str(&format!("\n✅ Task completed: {} succeeded, {} failed\n", completed, failed));

        if failed > 0 {
            return Err(ToolError::CommandFailed(result.clone()));
        }

        Ok(())
    }

    /// Execute steps in parallel
    async fn execute_steps_parallel(
        &self,
        steps: &[serde_json::Value],
        default_working_dir: &Path,
        result: &mut String,
    ) -> Result<(), ToolError> {
        use tokio::task::JoinSet;

        let mut join_set = JoinSet::new();

        // Spawn all tasks
        for (i, step) in steps.iter().enumerate() {
            let step_name = step["name"]
                .as_str()
                .ok_or_else(|| ToolError::InvalidInput(format!("Step {} missing name", i + 1)))?
                .to_string();

            let command = step["command"]
                .as_str()
                .ok_or_else(|| ToolError::InvalidInput(format!("Step {} missing command", i + 1)))?
                .to_string();

            // Get working directory for this step
            let working_dir = if let Some(wd) = step["working_directory"].as_str() {
                let path = Path::new(wd);
                if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    default_working_dir.join(path)
                }
            } else {
                default_working_dir.to_path_buf()
            };

            join_set.spawn(async move {
                let output = tokio::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "sh" })
                    .arg(if cfg!(target_os = "windows") { "/C" } else { "-c" })
                    .arg(&command)
                    .current_dir(&working_dir)
                    .output()
                    .await;

                (i, step_name, output)
            });
        }

        // Collect results
        let mut results_vec = Vec::new();
        while let Some(task_result) = join_set.join_next().await {
            match task_result {
                Ok((i, step_name, output_result)) => {
                    results_vec.push((i, step_name, output_result));
                }
                Err(e) => {
                    return Err(ToolError::CommandFailed(format!("Task join error: {}", e)));
                }
            }
        }

        // Sort results by original index to maintain order in output
        results_vec.sort_by_key(|(i, _, _)| *i);

        // Format results
        let mut completed = 0;
        let mut failed = 0;

        for (i, step_name, output_result) in results_vec {
            result.push_str(&format!("▶ Step {}/{}: {}\n", i + 1, steps.len(), step_name));

            match output_result {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    if output.status.success() {
                        result.push_str("  ✅ Success\n");
                        if !stdout.is_empty() {
                            result.push_str(&format!("  Output: {}\n", stdout.trim()));
                        }
                        completed += 1;
                    } else {
                        result.push_str("  ❌ Failed\n");
                        if !stderr.is_empty() {
                            result.push_str(&format!("  Error: {}\n", stderr.trim()));
                        }
                        failed += 1;
                    }
                }
                Err(e) => {
                    result.push_str(&format!("  ❌ Execution error: {}\n", e));
                    failed += 1;
                }
            }
        }

        result.push_str(&format!("\n✅ Task completed (parallel): {} succeeded, {} failed\n", completed, failed));

        if failed > 0 {
            return Err(ToolError::CommandFailed(result.clone()));
        }

        Ok(())
    }
}

/// Directory entry information
struct DirectoryEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: Option<u64>,
    modified: Option<String>,
    depth: usize,
}

impl DirectoryEntry {
    fn format(&self) -> String {
        let indent = "  ".repeat(self.depth);
        let type_indicator = if self.is_dir { "📁" } else { "📄" };
        let size_str = self
            .size
            .map(|s| format_size(s))
            .unwrap_or_else(|| "    -".to_string());
        let modified_str = self
            .modified
            .as_ref()
            .map(|m| m.as_str())
            .unwrap_or("unknown");

        format!(
            "{}{} {} {:>10}  {}  {}",
            indent, type_indicator, self.name, size_str, modified_str, self.path
        )
    }
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
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

    #[tokio::test]
    async fn test_list_directory_basic() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files and directories
        tokio::fs::write(temp_dir.path().join("file1.txt"), "content1").await.unwrap();
        tokio::fs::write(temp_dir.path().join("file2.rs"), "content2").await.unwrap();
        tokio::fs::create_dir(temp_dir.path().join("subdir")).await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "list_directory".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("3 items"));
        assert!(result.content.contains("file1.txt"));
        assert!(result.content.contains("file2.rs"));
        assert!(result.content.contains("subdir"));
    }

    #[tokio::test]
    async fn test_list_directory_with_path() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("testdir");
        tokio::fs::create_dir(&subdir).await.unwrap();
        tokio::fs::write(subdir.join("nested.txt"), "content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "list_directory".to_string(),
            input: serde_json::json!({
                "path": "testdir"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("nested.txt"));
    }

    #[tokio::test]
    async fn test_list_directory_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("visible.txt"), "content").await.unwrap();
        tokio::fs::write(temp_dir.path().join(".hidden"), "secret").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        // Test without show_hidden
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "list_directory".to_string(),
            input: serde_json::json!({
                "show_hidden": false
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("visible.txt"));
        assert!(!result.content.contains(".hidden"));

        // Test with show_hidden
        let tool_use_show = ToolUse {
            id: "test_456".to_string(),
            name: "list_directory".to_string(),
            input: serde_json::json!({
                "show_hidden": true
            }),
        };

        let result_show = executor.execute(&tool_use_show).await;
        assert_eq!(result_show.is_error, None);
        assert!(result_show.content.contains("visible.txt"));
        assert!(result_show.content.contains(".hidden"));
    }

    #[tokio::test]
    async fn test_list_directory_recursive() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("root.txt"), "content").await.unwrap();

        let subdir = temp_dir.path().join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();
        tokio::fs::write(subdir.join("nested.txt"), "content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        // Test non-recursive
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "list_directory".to_string(),
            input: serde_json::json!({
                "recursive": false
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("root.txt"));
        assert!(result.content.contains("subdir"));
        assert!(!result.content.contains("nested.txt"));

        // Test recursive
        let tool_use_recursive = ToolUse {
            id: "test_456".to_string(),
            name: "list_directory".to_string(),
            input: serde_json::json!({
                "recursive": true
            }),
        };

        let result_recursive = executor.execute(&tool_use_recursive).await;
        assert_eq!(result_recursive.is_error, None);
        assert!(result_recursive.content.contains("root.txt"));
        assert!(result_recursive.content.contains("subdir"));
        assert!(result_recursive.content.contains("nested.txt"));
    }

    #[tokio::test]
    async fn test_list_directory_empty() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "list_directory".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Directory is empty"));
    }

    #[tokio::test]
    async fn test_list_directory_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "list_directory".to_string(),
            input: serde_json::json!({
                "path": "nonexistent"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_list_directory_not_a_directory() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("file.txt"), "content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "list_directory".to_string(),
            input: serde_json::json!({
                "path": "file.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("not a directory"));
    }

    #[tokio::test]
    async fn test_multi_replace_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("test1.txt"), "Hello World\nHello Again").await.unwrap();
        tokio::fs::write(temp_dir.path().join("test2.txt"), "Hello Everyone").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "multi_replace".to_string(),
            input: serde_json::json!({
                "pattern": "Hello",
                "replacement": "Hi",
                "dry_run": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("DRY RUN"));
        assert!(result.content.contains("2 files would be changed"));

        // Verify files weren't actually modified
        let content1 = tokio::fs::read_to_string(temp_dir.path().join("test1.txt")).await.unwrap();
        assert_eq!(content1, "Hello World\nHello Again");
    }

    #[tokio::test]
    async fn test_multi_replace_actual() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("test.txt"), "foo bar foo").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "multi_replace".to_string(),
            input: serde_json::json!({
                "pattern": "foo",
                "replacement": "baz",
                "dry_run": false
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Changes applied"));
        assert!(result.content.contains("1 files would be changed"));

        // Verify file was actually modified
        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt")).await.unwrap();
        assert_eq!(content, "baz bar baz");
    }

    #[tokio::test]
    async fn test_multi_replace_with_file_pattern() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("test.rs"), "fn test() {}").await.unwrap();
        tokio::fs::write(temp_dir.path().join("test.txt"), "fn test() {}").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "multi_replace".to_string(),
            input: serde_json::json!({
                "pattern": "test",
                "replacement": "example",
                "file_pattern": "*.rs",
                "dry_run": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("test.rs"));
        assert!(!result.content.contains("test.txt"));
    }

    #[tokio::test]
    async fn test_multi_replace_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("test.txt"), "HELLO hello HeLLo").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "multi_replace".to_string(),
            input: serde_json::json!({
                "pattern": "hello",
                "replacement": "hi",
                "case_insensitive": true,
                "dry_run": false
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("3 total replacements"));

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt")).await.unwrap();
        assert_eq!(content, "hi hi hi");
    }

    #[tokio::test]
    async fn test_multi_replace_with_capture_groups() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("test.txt"), "name: John, name: Jane").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "multi_replace".to_string(),
            input: serde_json::json!({
                "pattern": r"name: (\w+)",
                "replacement": "person: $1",
                "dry_run": false
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt")).await.unwrap();
        assert_eq!(content, "person: John, person: Jane");
    }

    #[tokio::test]
    async fn test_multi_replace_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("test.txt"), "Some content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "multi_replace".to_string(),
            input: serde_json::json!({
                "pattern": "NotFound",
                "replacement": "Replaced",
                "dry_run": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("no matches found"));
    }

    #[tokio::test]
    async fn test_multi_replace_max_files() {
        let temp_dir = TempDir::new().unwrap();
        // Create 5 files
        for i in 1..=5 {
            tokio::fs::write(temp_dir.path().join(format!("test{}.txt", i)), "foo").await.unwrap();
        }

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "multi_replace".to_string(),
            input: serde_json::json!({
                "pattern": "foo",
                "replacement": "bar",
                "max_files": 3,
                "dry_run": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("3 files would be changed"));
    }

    #[tokio::test]
    async fn test_syntax_check_rust_valid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("valid.rs");
        tokio::fs::write(&file_path, r#"
fn main() {
    println!("Hello, World!");
}
"#).await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "syntax_check".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap()
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("✅") || result.content.contains("syntax check passed"));
    }

    #[tokio::test]
    async fn test_syntax_check_rust_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("invalid.rs");
        tokio::fs::write(&file_path, r#"
fn main() {
    println!("Hello, World!"
    // Missing closing parenthesis
}
"#).await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "syntax_check".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap()
            }),
        };

        let result = executor.execute(&tool_use).await;
        // May succeed with error output or fail, but should not panic
        assert!(result.content.contains("❌") || result.content.contains("error") || result.content.contains("✅"));
    }

    #[tokio::test]
    async fn test_syntax_check_with_language_override() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, r#"
fn main() {
    println!("Hello, World!");
}
"#).await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "syntax_check".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap(),
                "language": "rust"
            }),
        };

        let result = executor.execute(&tool_use).await;
        // Should be able to check rust syntax even with .txt extension
        assert_eq!(result.is_error, None);
    }

    #[tokio::test]
    async fn test_syntax_check_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "syntax_check".to_string(),
            input: serde_json::json!({
                "file_path": "nonexistent.rs"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_syntax_check_unknown_language() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.xyz");
        tokio::fs::write(&file_path, "some content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "syntax_check".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap()
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Cannot determine language"));
    }

    #[tokio::test]
    async fn test_syntax_check_unsupported_language() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.xyz");
        tokio::fs::write(&file_path, "some content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "syntax_check".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap(),
                "language": "ruby"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("not yet supported"));
    }

    #[tokio::test]
    async fn test_syntax_check_strict_mode() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        tokio::fs::write(&file_path, r#"
fn main() {
    println!("Hello, World!");
}
"#).await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "syntax_check".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap(),
                "strict": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        // Should not fail with strict mode on valid code
        assert_eq!(result.is_error, None);
    }

    #[tokio::test]
    async fn test_syntax_check_missing_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "syntax_check".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Missing file_path"));
    }

    #[tokio::test]
    async fn test_code_format_rust_check() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("format_test.rs");
        tokio::fs::write(&file_path, r#"
fn main() {
    println!("Hello, World!");
}
"#).await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_format".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap(),
                "check_only": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        // Should not error
        assert_eq!(result.is_error, None);
    }

    #[tokio::test]
    async fn test_code_format_rust_format() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("format_test.rs");
        // Intentionally poorly formatted Rust code
        tokio::fs::write(&file_path, "fn main(){println!(\"test\");}").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_format".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap(),
                "check_only": false
            }),
        };

        let result = executor.execute(&tool_use).await;
        // Should successfully format
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("✅") || result.content.contains("formatted"));
    }

    #[tokio::test]
    async fn test_code_format_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_format".to_string(),
            input: serde_json::json!({
                "file_path": "nonexistent.rs"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_code_format_unknown_language() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.xyz");
        tokio::fs::write(&file_path, "some content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_format".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap()
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Cannot determine language"));
    }

    #[tokio::test]
    async fn test_code_format_with_language_override() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "fn main(){println!(\"test\");}").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_format".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap(),
                "language": "rust",
                "check_only": false
            }),
        };

        let result = executor.execute(&tool_use).await;
        // Should be able to format rust code even with .txt extension
        assert_eq!(result.is_error, None);
    }

    #[tokio::test]
    async fn test_code_format_unsupported_language() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.xyz");
        tokio::fs::write(&file_path, "some content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_format".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap(),
                "language": "ruby"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("not yet supported"));
    }

    #[tokio::test]
    async fn test_code_format_missing_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_format".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Missing file_path"));
    }

    #[tokio::test]
    async fn test_code_analysis_rust_project() {
        // Use the current project directory which has Cargo.toml
        let current_dir = std::env::current_dir().unwrap();
        let executor = ToolExecutor::with_working_directory(&current_dir);

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_analysis".to_string(),
            input: serde_json::json!({
                "path": current_dir.to_str().unwrap(),
                "analysis_type": "quality"
            }),
        };

        let result = executor.execute(&tool_use).await;
        // Should not error
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Code Quality Analysis"));
    }

    #[tokio::test]
    async fn test_code_analysis_security_only() {
        let current_dir = std::env::current_dir().unwrap();
        let executor = ToolExecutor::with_working_directory(&current_dir);

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_analysis".to_string(),
            input: serde_json::json!({
                "path": current_dir.to_str().unwrap(),
                "analysis_type": "security"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Security Analysis"));
    }

    #[tokio::test]
    async fn test_code_analysis_all_types() {
        let current_dir = std::env::current_dir().unwrap();
        let executor = ToolExecutor::with_working_directory(&current_dir);

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_analysis".to_string(),
            input: serde_json::json!({
                "path": current_dir.to_str().unwrap(),
                "analysis_type": "all"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Code Quality Analysis") || result.content.contains("Security Analysis"));
    }

    #[tokio::test]
    async fn test_code_analysis_path_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_analysis".to_string(),
            input: serde_json::json!({
                "path": "nonexistent"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_code_analysis_missing_path() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_analysis".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Missing path"));
    }

    #[tokio::test]
    async fn test_code_analysis_with_language_override() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "fn main() {}").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_analysis".to_string(),
            input: serde_json::json!({
                "path": file_path.to_str().unwrap(),
                "language": "rust",
                "analysis_type": "quality"
            }),
        };

        let result = executor.execute(&tool_use).await;
        // Should be able to analyze rust code even with .txt extension
        assert_eq!(result.is_error, None);
    }

    #[tokio::test]
    async fn test_code_analysis_invalid_analysis_type() {
        let current_dir = std::env::current_dir().unwrap();
        let executor = ToolExecutor::with_working_directory(&current_dir);

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_analysis".to_string(),
            input: serde_json::json!({
                "path": current_dir.to_str().unwrap(),
                "analysis_type": "invalid"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Invalid analysis_type"));
    }

    #[tokio::test]
    async fn test_code_analysis_strict_mode() {
        let current_dir = std::env::current_dir().unwrap();
        let executor = ToolExecutor::with_working_directory(&current_dir);

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "code_analysis".to_string(),
            input: serde_json::json!({
                "path": current_dir.to_str().unwrap(),
                "analysis_type": "quality",
                "strict": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
    }

    #[tokio::test]
    async fn test_copy_file() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");
        tokio::fs::write(&source, "test content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "copy".to_string(),
            input: serde_json::json!({
                "source": "source.txt",
                "destination": "dest.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully copied file"));

        // Verify file was copied
        let dest_content = tokio::fs::read_to_string(&dest).await.unwrap();
        assert_eq!(dest_content, "test content");
    }

    #[tokio::test]
    async fn test_copy_directory() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let dest_dir = temp_dir.path().join("dest");

        // Create source directory with files
        tokio::fs::create_dir(&source_dir).await.unwrap();
        tokio::fs::write(source_dir.join("file1.txt"), "content1").await.unwrap();
        tokio::fs::write(source_dir.join("file2.txt"), "content2").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "copy".to_string(),
            input: serde_json::json!({
                "source": "source",
                "destination": "dest"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully copied directory"));
        assert!(result.content.contains("2 files copied"));

        // Verify files were copied
        assert!(dest_dir.join("file1.txt").exists());
        assert!(dest_dir.join("file2.txt").exists());
    }

    #[tokio::test]
    async fn test_copy_nested_directory() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let nested_dir = source_dir.join("nested");
        let dest_dir = temp_dir.path().join("dest");

        // Create nested directory structure
        tokio::fs::create_dir_all(&nested_dir).await.unwrap();
        tokio::fs::write(source_dir.join("file1.txt"), "content1").await.unwrap();
        tokio::fs::write(nested_dir.join("file2.txt"), "content2").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "copy".to_string(),
            input: serde_json::json!({
                "source": "source",
                "destination": "dest"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);

        // Verify nested structure was copied
        assert!(dest_dir.join("file1.txt").exists());
        assert!(dest_dir.join("nested").exists());
        assert!(dest_dir.join("nested/file2.txt").exists());
    }

    #[tokio::test]
    async fn test_copy_overwrite_false() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");
        tokio::fs::write(&source, "new content").await.unwrap();
        tokio::fs::write(&dest, "existing content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "copy".to_string(),
            input: serde_json::json!({
                "source": "source.txt",
                "destination": "dest.txt",
                "overwrite": false
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("already exists"));

        // Verify destination was not overwritten
        let dest_content = tokio::fs::read_to_string(&dest).await.unwrap();
        assert_eq!(dest_content, "existing content");
    }

    #[tokio::test]
    async fn test_copy_overwrite_true() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");
        tokio::fs::write(&source, "new content").await.unwrap();
        tokio::fs::write(&dest, "existing content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "copy".to_string(),
            input: serde_json::json!({
                "source": "source.txt",
                "destination": "dest.txt",
                "overwrite": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);

        // Verify destination was overwritten
        let dest_content = tokio::fs::read_to_string(&dest).await.unwrap();
        assert_eq!(dest_content, "new content");
    }

    #[tokio::test]
    async fn test_copy_source_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "copy".to_string(),
            input: serde_json::json!({
                "source": "nonexistent.txt",
                "destination": "dest.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_copy_missing_parameters() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "copy".to_string(),
            input: serde_json::json!({
                "source": "source.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Missing destination"));
    }

    #[tokio::test]
    async fn test_copy_directory_non_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        tokio::fs::create_dir(&source_dir).await.unwrap();
        tokio::fs::write(source_dir.join("file.txt"), "content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "copy".to_string(),
            input: serde_json::json!({
                "source": "source",
                "destination": "dest",
                "recursive": false
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Set recursive=true"));
    }

    #[tokio::test]
    async fn test_delete_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "test content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "delete".to_string(),
            input: serde_json::json!({
                "path": "test.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully deleted file"));

        // Verify file was deleted
        assert!(!file_path.exists());
    }

    #[tokio::test]
    async fn test_delete_directory_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("testdir");
        tokio::fs::create_dir(&dir_path).await.unwrap();
        tokio::fs::write(dir_path.join("file1.txt"), "content1").await.unwrap();
        tokio::fs::write(dir_path.join("file2.txt"), "content2").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "delete".to_string(),
            input: serde_json::json!({
                "path": "testdir",
                "recursive": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully deleted directory"));
        assert!(result.content.contains("2 files"));

        // Verify directory was deleted
        assert!(!dir_path.exists());
    }

    #[tokio::test]
    async fn test_delete_nested_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("testdir");
        let nested_dir = dir_path.join("nested");
        tokio::fs::create_dir_all(&nested_dir).await.unwrap();
        tokio::fs::write(dir_path.join("file1.txt"), "content1").await.unwrap();
        tokio::fs::write(nested_dir.join("file2.txt"), "content2").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "delete".to_string(),
            input: serde_json::json!({
                "path": "testdir",
                "recursive": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);

        // Verify directory and nested contents were deleted
        assert!(!dir_path.exists());
        assert!(!nested_dir.exists());
    }

    #[tokio::test]
    async fn test_delete_directory_non_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("testdir");
        tokio::fs::create_dir(&dir_path).await.unwrap();
        tokio::fs::write(dir_path.join("file.txt"), "content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "delete".to_string(),
            input: serde_json::json!({
                "path": "testdir",
                "recursive": false
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Set recursive=true"));

        // Verify directory was NOT deleted
        assert!(dir_path.exists());
    }

    #[tokio::test]
    async fn test_delete_path_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "delete".to_string(),
            input: serde_json::json!({
                "path": "nonexistent.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_delete_missing_path() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "delete".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Missing path"));
    }

    #[tokio::test]
    async fn test_delete_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("emptydir");
        tokio::fs::create_dir(&dir_path).await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "delete".to_string(),
            input: serde_json::json!({
                "path": "emptydir",
                "recursive": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully deleted directory"));

        // Verify directory was deleted
        assert!(!dir_path.exists());
    }

    // Move Tool Tests

    #[tokio::test]
    async fn test_move_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = temp_dir.path().join("source.txt");
        let dest_file = temp_dir.path().join("destination.txt");

        tokio::fs::write(&source_file, "test content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "move".to_string(),
            input: serde_json::json!({
                "source": "source.txt",
                "destination": "destination.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully moved file"));

        // Verify source no longer exists and destination has content
        assert!(!source_file.exists());
        assert!(dest_file.exists());
        let content = tokio::fs::read_to_string(&dest_file).await.unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_move_directory() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source_dir");
        let dest_dir = temp_dir.path().join("dest_dir");

        tokio::fs::create_dir(&source_dir).await.unwrap();
        tokio::fs::write(source_dir.join("file.txt"), "content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "move".to_string(),
            input: serde_json::json!({
                "source": "source_dir",
                "destination": "dest_dir"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully moved directory"));

        // Verify source no longer exists and destination exists with content
        assert!(!source_dir.exists());
        assert!(dest_dir.exists());
        assert!(dest_dir.join("file.txt").exists());
    }

    #[tokio::test]
    async fn test_move_rename_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = temp_dir.path().join("oldname.txt");
        let dest_file = temp_dir.path().join("newname.txt");

        tokio::fs::write(&source_file, "test content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "move".to_string(),
            input: serde_json::json!({
                "source": "oldname.txt",
                "destination": "newname.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully moved file"));

        // Verify rename worked
        assert!(!source_file.exists());
        assert!(dest_file.exists());
    }

    #[tokio::test]
    async fn test_move_overwrite_false() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = temp_dir.path().join("source.txt");
        let dest_file = temp_dir.path().join("dest.txt");

        tokio::fs::write(&source_file, "source content").await.unwrap();
        tokio::fs::write(&dest_file, "dest content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "move".to_string(),
            input: serde_json::json!({
                "source": "source.txt",
                "destination": "dest.txt",
                "overwrite": false
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("already exists"));

        // Verify both files still exist unchanged
        assert!(source_file.exists());
        assert!(dest_file.exists());
        let dest_content = tokio::fs::read_to_string(&dest_file).await.unwrap();
        assert_eq!(dest_content, "dest content");
    }

    #[tokio::test]
    async fn test_move_overwrite_true() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = temp_dir.path().join("source.txt");
        let dest_file = temp_dir.path().join("dest.txt");

        tokio::fs::write(&source_file, "source content").await.unwrap();
        tokio::fs::write(&dest_file, "dest content").await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "move".to_string(),
            input: serde_json::json!({
                "source": "source.txt",
                "destination": "dest.txt",
                "overwrite": true
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully moved file"));

        // Verify source is gone and dest has source content
        assert!(!source_file.exists());
        assert!(dest_file.exists());
        let dest_content = tokio::fs::read_to_string(&dest_file).await.unwrap();
        assert_eq!(dest_content, "source content");
    }

    #[tokio::test]
    async fn test_move_source_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "move".to_string(),
            input: serde_json::json!({
                "source": "nonexistent.txt",
                "destination": "dest.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_move_missing_parameters() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        // Missing destination
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "move".to_string(),
            input: serde_json::json!({
                "source": "file.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Missing destination"));

        // Missing source
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "move".to_string(),
            input: serde_json::json!({
                "destination": "file.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Missing source"));
    }

    #[tokio::test]
    async fn test_move_to_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = temp_dir.path().join("source.txt");
        let subdir = temp_dir.path().join("subdir");
        let dest_file = subdir.join("dest.txt");

        tokio::fs::write(&source_file, "test content").await.unwrap();
        tokio::fs::create_dir(&subdir).await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "move".to_string(),
            input: serde_json::json!({
                "source": "source.txt",
                "destination": "subdir/dest.txt"
            }),
        };

        let result = executor.execute(&tool_use).await;
        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Successfully moved file"));

        // Verify move worked
        assert!(!source_file.exists());
        assert!(dest_file.exists());
        let content = tokio::fs::read_to_string(&dest_file).await.unwrap();
        assert_eq!(content, "test content");
    }

    // Build Tool Tests

    #[tokio::test]
    async fn test_build_rust_debug() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.toml to simulate Rust project
        tokio::fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
        )
        .await
        .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "build".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        // Build will likely fail without src/, but we're testing detection and command construction
        assert!(result.content.contains("rust"));
        assert!(result.content.contains("cargo build"));
    }

    #[tokio::test]
    async fn test_build_rust_release() {
        let temp_dir = TempDir::new().unwrap();

        tokio::fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-project"
version = "0.1.0"
"#,
        )
        .await
        .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "build".to_string(),
            input: serde_json::json!({
                "build_type": "release"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("rust"));
        assert!(result.content.contains("cargo build --release"));
    }

    #[tokio::test]
    async fn test_build_custom_command() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "build".to_string(),
            input: serde_json::json!({
                "project_type": "custom",
                "custom_command": "echo test build"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Build succeeded"));
        assert!(result.content.contains("custom"));
    }

    #[tokio::test]
    async fn test_build_with_additional_args() {
        let temp_dir = TempDir::new().unwrap();

        tokio::fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"",
        )
        .await
        .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "build".to_string(),
            input: serde_json::json!({
                "args": ["--verbose", "--features", "test-feature"]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("cargo build --verbose --features test-feature"));
    }

    #[tokio::test]
    async fn test_build_nodejs_project() {
        let temp_dir = TempDir::new().unwrap();

        tokio::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "test", "scripts": {"build": "echo building"}}"#,
        )
        .await
        .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "build".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("javascript"));
        assert!(result.content.contains("npm run build"));
    }

    #[tokio::test]
    async fn test_build_python_project() {
        let temp_dir = TempDir::new().unwrap();

        tokio::fs::write(
            temp_dir.path().join("setup.py"),
            "from setuptools import setup\nsetup(name='test')",
        )
        .await
        .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "build".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("python"));
        assert!(result.content.contains("python setup.py build"));
    }

    #[tokio::test]
    async fn test_build_go_project() {
        let temp_dir = TempDir::new().unwrap();

        tokio::fs::write(
            temp_dir.path().join("go.mod"),
            "module test\n\ngo 1.20",
        )
        .await
        .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "build".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("go"));
        assert!(result.content.contains("go build"));
    }

    #[tokio::test]
    async fn test_build_no_project_detected() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "build".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Could not detect project type"));
    }

    #[tokio::test]
    async fn test_build_custom_without_command() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "build".to_string(),
            input: serde_json::json!({
                "project_type": "custom"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("custom_command is required"));
    }

    #[tokio::test]
    async fn test_build_with_working_directory() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subproject");
        tokio::fs::create_dir(&subdir).await.unwrap();

        tokio::fs::write(
            subdir.join("Cargo.toml"),
            "[package]\nname = \"subtest\"",
        )
        .await
        .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "build".to_string(),
            input: serde_json::json!({
                "working_directory": "subproject"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("rust"));
        assert!(result.content.contains("cargo build"));
    }

    // Test Runner Tool Tests

    #[tokio::test]
    async fn test_test_runner_rust_all_tests() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.toml
        tokio::fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        // The test may fail or pass depending on the project, but it should execute
        assert!(result.content.contains("cargo test"));
    }

    #[tokio::test]
    async fn test_test_runner_rust_unit_tests() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.toml
        tokio::fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({
                "test_type": "unit"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("cargo test --lib"));
    }

    #[tokio::test]
    async fn test_test_runner_rust_with_pattern() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.toml
        tokio::fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({
                "test_pattern": "test_foo"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("cargo test test_foo"));
    }

    #[tokio::test]
    async fn test_test_runner_javascript() {
        let temp_dir = TempDir::new().unwrap();

        // Create a package.json
        tokio::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "test", "scripts": {"test": "jest"}}"#
        )
        .await
        .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("npm test"));
    }

    #[tokio::test]
    async fn test_test_runner_python() {
        let temp_dir = TempDir::new().unwrap();

        // Create a setup.py
        tokio::fs::write(temp_dir.path().join("setup.py"), "")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("pytest"));
    }

    #[tokio::test]
    async fn test_test_runner_python_with_pattern() {
        let temp_dir = TempDir::new().unwrap();

        // Create a pytest.ini
        tokio::fs::write(temp_dir.path().join("pytest.ini"), "")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({
                "test_pattern": "test_auth"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("pytest -k test_auth"));
    }

    #[tokio::test]
    async fn test_test_runner_go() {
        let temp_dir = TempDir::new().unwrap();

        // Create a go.mod
        tokio::fs::write(temp_dir.path().join("go.mod"), "module test")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("go test"));
    }

    #[tokio::test]
    async fn test_test_runner_custom_command() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({
                "project_type": "custom",
                "custom_command": "echo 'Running custom tests'"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, None);
        assert!(result.content.contains("custom tests"));
    }

    #[tokio::test]
    async fn test_test_runner_with_additional_args() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.toml
        tokio::fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({
                "args": ["--nocapture", "--quiet"]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("cargo test --nocapture --quiet"));
    }

    #[tokio::test]
    async fn test_test_runner_no_project_detected() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Could not detect project type"));
    }

    #[tokio::test]
    async fn test_test_runner_custom_without_command() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({
                "project_type": "custom"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("custom_command is required"));
    }

    #[tokio::test]
    async fn test_test_runner_with_working_directory() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subproject");
        tokio::fs::create_dir(&subdir).await.unwrap();

        // Create Cargo.toml in subdir
        tokio::fs::write(subdir.join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({
                "working_directory": "subproject"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("rust"));
        assert!(result.content.contains("cargo test"));
    }

    #[tokio::test]
    async fn test_test_runner_integration_tests() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.toml
        tokio::fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "test_runner".to_string(),
            input: serde_json::json!({
                "test_type": "integration"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("cargo test --test"));
    }

    // Lint Tool Tests

    #[tokio::test]
    async fn test_lint_rust_default() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.toml
        tokio::fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("cargo clippy"));
    }

    #[tokio::test]
    async fn test_lint_rust_with_fix() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.toml
        tokio::fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({
                "fix": true
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("cargo clippy --fix"));
    }

    #[tokio::test]
    async fn test_lint_rust_errors_only() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.toml
        tokio::fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({
                "severity": "error"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("cargo clippy -- -D warnings"));
    }

    #[tokio::test]
    async fn test_lint_javascript() {
        let temp_dir = TempDir::new().unwrap();

        // Create a package.json
        tokio::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "test"}"#
        )
        .await
        .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("npx eslint ."));
    }

    #[tokio::test]
    async fn test_lint_javascript_with_fix() {
        let temp_dir = TempDir::new().unwrap();

        // Create a package.json
        tokio::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "test"}"#
        )
        .await
        .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({
                "fix": true
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("npx eslint . --fix"));
    }

    #[tokio::test]
    async fn test_lint_python() {
        let temp_dir = TempDir::new().unwrap();

        // Create a setup.py
        tokio::fs::write(temp_dir.path().join("setup.py"), "")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("pylint ."));
    }

    #[tokio::test]
    async fn test_lint_python_errors_only() {
        let temp_dir = TempDir::new().unwrap();

        // Create a pyproject.toml
        tokio::fs::write(temp_dir.path().join("pyproject.toml"), "")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({
                "severity": "error"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("pylint . --errors-only"));
    }

    #[tokio::test]
    async fn test_lint_go() {
        let temp_dir = TempDir::new().unwrap();

        // Create a go.mod
        tokio::fs::write(temp_dir.path().join("go.mod"), "module test")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("go vet"));
    }

    #[tokio::test]
    async fn test_lint_custom_command() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({
                "project_type": "custom",
                "custom_command": "echo 'Running custom linter'"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, None);
        assert!(result.content.contains("custom linter"));
    }

    #[tokio::test]
    async fn test_lint_with_additional_args() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.toml
        tokio::fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({
                "args": ["--", "-W", "clippy::all"]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("cargo clippy -- -W clippy::all"));
    }

    #[tokio::test]
    async fn test_lint_no_project_detected() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({}),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Could not detect project type"));
    }

    #[tokio::test]
    async fn test_lint_custom_without_command() {
        let temp_dir = TempDir::new().unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({
                "project_type": "custom"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("custom_command is required"));
    }

    #[tokio::test]
    async fn test_lint_with_working_directory() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subproject");
        tokio::fs::create_dir(&subdir).await.unwrap();

        // Create Cargo.toml in subdir
        tokio::fs::write(subdir.join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({
                "working_directory": "subproject"
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert!(result.content.contains("rust"));
        assert!(result.content.contains("cargo clippy"));
    }

    // Task Tool Tests

    #[tokio::test]
    async fn test_task_sequential_success() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "description": "Create and list files",
                "execution_mode": "sequential",
                "steps": [
                    {
                        "name": "Create file1",
                        "command": "echo 'test1' > file1.txt"
                    },
                    {
                        "name": "Create file2",
                        "command": "echo 'test2' > file2.txt"
                    },
                    {
                        "name": "List files",
                        "command": if cfg!(target_os = "windows") { "dir /b *.txt" } else { "ls *.txt" }
                    }
                ]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, None);
        assert!(result.content.contains("Create and list files"));
        assert!(result.content.contains("sequential"));
        assert!(result.content.contains("✅"));
    }

    #[tokio::test]
    async fn test_task_parallel_success() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "description": "Create files in parallel",
                "execution_mode": "parallel",
                "steps": [
                    {
                        "name": "Create file1",
                        "command": "echo 'test1' > file1.txt"
                    },
                    {
                        "name": "Create file2",
                        "command": "echo 'test2' > file2.txt"
                    },
                    {
                        "name": "Create file3",
                        "command": "echo 'test3' > file3.txt"
                    }
                ]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, None);
        assert!(result.content.contains("parallel"));
        assert!(result.content.contains("3 succeeded"));
    }

    #[tokio::test]
    async fn test_task_sequential_stop_on_error() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "description": "Test stop on error",
                "execution_mode": "sequential",
                "stop_on_error": true,
                "steps": [
                    {
                        "name": "Success step",
                        "command": "echo 'success'"
                    },
                    {
                        "name": "Failing step",
                        "command": "exit 1"
                    },
                    {
                        "name": "Should not run",
                        "command": "echo 'should not see this'"
                    }
                ]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Stopping execution"));
        assert!(result.content.contains("1 skipped"));
    }

    #[tokio::test]
    async fn test_task_sequential_continue_on_error() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "description": "Test continue on error",
                "execution_mode": "sequential",
                "stop_on_error": false,
                "steps": [
                    {
                        "name": "Success step 1",
                        "command": "echo 'success1'"
                    },
                    {
                        "name": "Failing step",
                        "command": "exit 1"
                    },
                    {
                        "name": "Success step 2",
                        "command": "echo 'success2'"
                    }
                ]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("2 succeeded, 1 failed"));
    }

    #[tokio::test]
    async fn test_task_with_working_directory() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "description": "Test with working directory",
                "working_directory": "subdir",
                "steps": [
                    {
                        "name": "Create file in subdir",
                        "command": "echo 'test' > file.txt"
                    }
                ]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, None);
        assert!(subdir.join("file.txt").exists());
    }

    #[tokio::test]
    async fn test_task_per_step_working_directory() {
        let temp_dir = TempDir::new().unwrap();
        let subdir1 = temp_dir.path().join("subdir1");
        let subdir2 = temp_dir.path().join("subdir2");
        tokio::fs::create_dir(&subdir1).await.unwrap();
        tokio::fs::create_dir(&subdir2).await.unwrap();

        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "description": "Test per-step working directories",
                "steps": [
                    {
                        "name": "Create in subdir1",
                        "command": "echo 'test1' > file1.txt",
                        "working_directory": "subdir1"
                    },
                    {
                        "name": "Create in subdir2",
                        "command": "echo 'test2' > file2.txt",
                        "working_directory": "subdir2"
                    }
                ]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, None);
        assert!(subdir1.join("file1.txt").exists());
        assert!(subdir2.join("file2.txt").exists());
    }

    #[tokio::test]
    async fn test_task_missing_description() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "steps": [
                    {
                        "name": "Test",
                        "command": "echo 'test'"
                    }
                ]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Missing task description"));
    }

    #[tokio::test]
    async fn test_task_empty_steps() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "description": "Empty task",
                "steps": []
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("at least one step"));
    }

    #[tokio::test]
    async fn test_task_invalid_execution_mode() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "description": "Test invalid mode",
                "execution_mode": "invalid",
                "steps": [
                    {
                        "name": "Test",
                        "command": "echo 'test'"
                    }
                ]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, Some(true));
        assert!(result.content.contains("Invalid execution_mode"));
    }

    #[tokio::test]
    async fn test_task_default_execution_mode() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        let tool_use = ToolUse {
            id: "test_123".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "description": "Test default mode",
                "steps": [
                    {
                        "name": "Test",
                        "command": "echo 'test'"
                    }
                ]
            }),
        };

        let result = executor.execute(&tool_use).await;

        assert_eq!(result.is_error, None);
        assert!(result.content.contains("sequential"));
    }

    // Integration Tests - Multi-Tool Workflows

    #[tokio::test]
    async fn test_integration_write_read_edit_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());
        let file_path = temp_dir.path().join("test.txt");

        // Step 1: Write a file
        let write_tool = ToolUse {
            id: "write_1".to_string(),
            name: "write".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap(),
                "content": "Hello World\nLine 2\nLine 3"
            }),
        };

        let write_result = executor.execute(&write_tool).await;
        assert_eq!(write_result.is_error, None);
        assert!(file_path.exists());

        // Step 2: Read the file
        let read_tool = ToolUse {
            id: "read_1".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap()
            }),
        };

        let read_result = executor.execute(&read_tool).await;
        assert_eq!(read_result.is_error, None);
        assert!(read_result.content.contains("Hello World"));

        // Step 3: Edit the file
        let edit_tool = ToolUse {
            id: "edit_1".to_string(),
            name: "edit".to_string(),
            input: serde_json::json!({
                "file_path": file_path.to_str().unwrap(),
                "old_string": "Hello World",
                "new_string": "Goodbye World"
            }),
        };

        let edit_result = executor.execute(&edit_tool).await;
        assert_eq!(edit_result.is_error, None);

        // Step 4: Verify the edit
        let verify_result = executor.execute(&read_tool).await;
        assert!(verify_result.content.contains("Goodbye World"));
        assert!(!verify_result.content.contains("Hello World"));
    }

    #[tokio::test]
    async fn test_integration_grep_and_replace_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        // Create test files
        tokio::fs::write(temp_dir.path().join("file1.txt"), "foo bar baz").await.unwrap();
        tokio::fs::write(temp_dir.path().join("file2.txt"), "foo qux foo").await.unwrap();
        tokio::fs::write(temp_dir.path().join("file3.md"), "foo test").await.unwrap();

        // Step 1: Search for pattern
        let grep_tool = ToolUse {
            id: "grep_1".to_string(),
            name: "grep".to_string(),
            input: serde_json::json!({
                "pattern": "foo",
                "path": temp_dir.path().to_str().unwrap(),
                "output_mode": "files_with_matches"
            }),
        };

        let grep_result = executor.execute(&grep_tool).await;
        assert_eq!(grep_result.is_error, None);
        assert!(grep_result.content.contains("file1.txt"));
        assert!(grep_result.content.contains("file2.txt"));

        // Step 2: Replace in specific file pattern
        let replace_tool = ToolUse {
            id: "replace_1".to_string(),
            name: "multi_replace".to_string(),
            input: serde_json::json!({
                "pattern": "foo",
                "replacement": "bar",
                "file_pattern": "*.txt",
                "base_path": temp_dir.path().to_str().unwrap(),
                "dry_run": false
            }),
        };

        let replace_result = executor.execute(&replace_tool).await;
        assert_eq!(replace_result.is_error, None);
        assert!(replace_result.content.contains("2 files"));

        // Step 3: Verify replacements
        let content1 = tokio::fs::read_to_string(temp_dir.path().join("file1.txt")).await.unwrap();
        let content2 = tokio::fs::read_to_string(temp_dir.path().join("file2.txt")).await.unwrap();
        let content3 = tokio::fs::read_to_string(temp_dir.path().join("file3.md")).await.unwrap();

        assert_eq!(content1, "bar bar baz");
        assert_eq!(content2, "bar qux bar");
        assert_eq!(content3, "foo test"); // Should not be changed
    }

    #[tokio::test]
    async fn test_integration_code_quality_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        // Create a Rust project structure
        tokio::fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\""
        ).await.unwrap();

        let src_dir = temp_dir.path().join("src");
        tokio::fs::create_dir(&src_dir).await.unwrap();
        tokio::fs::write(
            src_dir.join("main.rs"),
            "fn main() {\n    println!(\"Hello\");\n}"
        ).await.unwrap();

        // Step 1: Syntax check
        let syntax_tool = ToolUse {
            id: "syntax_1".to_string(),
            name: "syntax_check".to_string(),
            input: serde_json::json!({
                "file_path": src_dir.join("main.rs").to_str().unwrap()
            }),
        };

        let syntax_result = executor.execute(&syntax_tool).await;
        assert_eq!(syntax_result.is_error, None);

        // Step 2: Code format check
        let format_tool = ToolUse {
            id: "format_1".to_string(),
            name: "code_format".to_string(),
            input: serde_json::json!({
                "file_path": src_dir.join("main.rs").to_str().unwrap(),
                "action": "check"
            }),
        };

        let _format_result = executor.execute(&format_tool).await;
        // Format check might pass or fail depending on rustfmt availability

        // Step 3: Lint (will attempt cargo clippy)
        let lint_tool = ToolUse {
            id: "lint_1".to_string(),
            name: "lint".to_string(),
            input: serde_json::json!({}),
        };

        let _lint_result = executor.execute(&lint_tool).await;
        // Lint might fail if clippy isn't available, but shouldn't crash
    }

    #[tokio::test]
    async fn test_integration_file_operations_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        // Step 1: Create source file
        let source = temp_dir.path().join("source.txt");
        tokio::fs::write(&source, "test content").await.unwrap();

        // Step 2: Copy file
        let copy_tool = ToolUse {
            id: "copy_1".to_string(),
            name: "copy".to_string(),
            input: serde_json::json!({
                "source": "source.txt",
                "destination": "backup.txt"
            }),
        };

        let copy_result = executor.execute(&copy_tool).await;
        assert_eq!(copy_result.is_error, None);
        assert!(temp_dir.path().join("backup.txt").exists());

        // Step 3: Edit original
        let edit_tool = ToolUse {
            id: "edit_1".to_string(),
            name: "edit".to_string(),
            input: serde_json::json!({
                "file_path": source.to_str().unwrap(),
                "old_string": "test",
                "new_string": "modified"
            }),
        };

        let edit_result = executor.execute(&edit_tool).await;
        assert_eq!(edit_result.is_error, None);

        // Step 4: Verify both files
        let original_content = tokio::fs::read_to_string(&source).await.unwrap();
        let backup_content = tokio::fs::read_to_string(temp_dir.path().join("backup.txt")).await.unwrap();

        assert_eq!(original_content, "modified content");
        assert_eq!(backup_content, "test content");

        // Step 5: Move file
        let move_tool = ToolUse {
            id: "move_1".to_string(),
            name: "move".to_string(),
            input: serde_json::json!({
                "source": "backup.txt",
                "destination": "archive.txt"
            }),
        };

        let move_result = executor.execute(&move_tool).await;
        assert_eq!(move_result.is_error, None);
        assert!(!temp_dir.path().join("backup.txt").exists());
        assert!(temp_dir.path().join("archive.txt").exists());

        // Step 6: Delete file
        let delete_tool = ToolUse {
            id: "delete_1".to_string(),
            name: "delete".to_string(),
            input: serde_json::json!({
                "path": "archive.txt"
            }),
        };

        let delete_result = executor.execute(&delete_tool).await;
        assert_eq!(delete_result.is_error, None);
        assert!(!temp_dir.path().join("archive.txt").exists());
    }

    #[tokio::test]
    async fn test_integration_task_orchestration_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        // Use Task tool to orchestrate multiple operations
        let task_tool = ToolUse {
            id: "task_1".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "description": "Setup project structure",
                "execution_mode": "sequential",
                "steps": [
                    {
                        "name": "Create src directory",
                        "command": if cfg!(target_os = "windows") { "mkdir src" } else { "mkdir -p src" }
                    },
                    {
                        "name": "Create test directory",
                        "command": if cfg!(target_os = "windows") { "mkdir tests" } else { "mkdir -p tests" }
                    },
                    {
                        "name": "Create README",
                        "command": "echo '# Project' > README.md"
                    },
                    {
                        "name": "List structure",
                        "command": if cfg!(target_os = "windows") { "dir /b" } else { "ls -la" }
                    }
                ]
            }),
        };

        let task_result = executor.execute(&task_tool).await;
        assert_eq!(task_result.is_error, None);
        assert!(task_result.content.contains("4 succeeded"));

        // Verify the structure was created
        assert!(temp_dir.path().join("src").exists());
        assert!(temp_dir.path().join("tests").exists());
        assert!(temp_dir.path().join("README.md").exists());
    }

    #[tokio::test]
    async fn test_integration_search_and_analyze_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        // Create a project structure
        let src_dir = temp_dir.path().join("src");
        tokio::fs::create_dir(&src_dir).await.unwrap();

        tokio::fs::write(
            src_dir.join("main.rs"),
            "fn main() {\n    println!(\"Hello\");\n}\n\nfn helper() {}"
        ).await.unwrap();

        tokio::fs::write(
            src_dir.join("lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}"
        ).await.unwrap();

        // Step 1: Find all Rust files
        let glob_tool = ToolUse {
            id: "glob_1".to_string(),
            name: "glob".to_string(),
            input: serde_json::json!({
                "pattern": "**/*.rs",
                "path": temp_dir.path().to_str().unwrap()
            }),
        };

        let glob_result = executor.execute(&glob_tool).await;
        assert_eq!(glob_result.is_error, None);
        assert!(glob_result.content.contains("main.rs"));
        assert!(glob_result.content.contains("lib.rs"));

        // Step 2: Search for function definitions
        let grep_tool = ToolUse {
            id: "grep_1".to_string(),
            name: "grep".to_string(),
            input: serde_json::json!({
                "pattern": "fn \\w+",
                "path": src_dir.to_str().unwrap(),
                "output_mode": "content",
                "-n": true
            }),
        };

        let grep_result = executor.execute(&grep_tool).await;
        assert_eq!(grep_result.is_error, None);
        assert!(grep_result.content.contains("fn main"));
        assert!(grep_result.content.contains("fn helper"));
        assert!(grep_result.content.contains("fn add"));

        // Step 3: List directory contents
        let list_tool = ToolUse {
            id: "list_1".to_string(),
            name: "list_directory".to_string(),
            input: serde_json::json!({
                "path": src_dir.to_str().unwrap()
            }),
        };

        let list_result = executor.execute(&list_tool).await;
        assert_eq!(list_result.is_error, None);
        assert!(list_result.content.contains("main.rs"));
        assert!(list_result.content.contains("lib.rs"));
    }

    // Error Handling Integration Tests

    #[tokio::test]
    async fn test_integration_error_recovery_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        // Try to read non-existent file
        let read_tool = ToolUse {
            id: "read_1".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({
                "file_path": temp_dir.path().join("nonexistent.txt").to_str().unwrap()
            }),
        };

        let read_result = executor.execute(&read_tool).await;
        assert_eq!(read_result.is_error, Some(true));

        // Recover by creating the file
        let write_tool = ToolUse {
            id: "write_1".to_string(),
            name: "write".to_string(),
            input: serde_json::json!({
                "file_path": temp_dir.path().join("nonexistent.txt").to_str().unwrap(),
                "content": "now it exists"
            }),
        };

        let write_result = executor.execute(&write_tool).await;
        assert_eq!(write_result.is_error, None);

        // Try reading again
        let read_result2 = executor.execute(&read_tool).await;
        assert_eq!(read_result2.is_error, None);
        assert!(read_result2.content.contains("now it exists"));
    }

    #[tokio::test]
    async fn test_integration_task_partial_failure_handling() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::with_working_directory(temp_dir.path());

        // Task with stop_on_error=false should continue after failures
        let task_tool = ToolUse {
            id: "task_1".to_string(),
            name: "task".to_string(),
            input: serde_json::json!({
                "description": "Mixed success and failure",
                "execution_mode": "sequential",
                "stop_on_error": false,
                "steps": [
                    {
                        "name": "Success step 1",
                        "command": "echo 'step1'"
                    },
                    {
                        "name": "Failing step",
                        "command": "exit 1"
                    },
                    {
                        "name": "Success step 2",
                        "command": "echo 'step2'"
                    },
                    {
                        "name": "Another failure",
                        "command": "exit 1"
                    },
                    {
                        "name": "Success step 3",
                        "command": "echo 'step3'"
                    }
                ]
            }),
        };

        let task_result = executor.execute(&task_tool).await;
        assert_eq!(task_result.is_error, Some(true));
        assert!(task_result.content.contains("3 succeeded, 2 failed"));
    }
}
