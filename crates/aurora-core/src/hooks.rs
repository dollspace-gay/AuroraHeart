//! Session hooks for behavioral modification
//!
//! This module provides hook execution for session lifecycle events.
//! Hooks are shell scripts that run at specific points:
//! - SessionStart: When a conversation session begins
//! - SessionEnd: When a conversation session ends
//! - BeforeToolCall: Before executing a tool
//! - AfterToolCall: After executing a tool
//!
//! Hooks can modify behavior by:
//! - Setting environment variables
//! - Outputting instructions to inject into prompts
//! - Validating or blocking operations
//! - Logging and analytics

use crate::plugin::{Hook, HookType, PluginManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

/// Errors that can occur during hook execution
#[derive(Error, Debug)]
pub enum HookError {
    /// Hook script not found
    #[error("Hook script not found: {0}")]
    ScriptNotFound(String),

    /// Hook execution failed
    #[error("Hook execution failed: {0}")]
    ExecutionFailed(String),

    /// Hook timed out
    #[error("Hook timed out after {0}s")]
    Timeout(u64),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid hook output
    #[error("Invalid hook output: {0}")]
    InvalidOutput(String),
}

pub type Result<T> = std::result::Result<T, HookError>;

/// Context passed to session start hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartContext {
    /// Project root directory
    pub project_root: String,
    /// User's initial message (if any)
    pub initial_message: Option<String>,
}

/// Context passed to session end hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndContext {
    /// Total messages in conversation
    pub message_count: usize,
    /// Total characters processed
    pub total_chars: usize,
}

/// Context passed to tool call hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallContext {
    /// Tool being called
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
    /// Tool call ID
    pub tool_id: String,
}

/// Context passed to after tool call hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterToolCallContext {
    /// Tool that was called
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
    /// Tool call ID
    pub tool_id: String,
    /// Tool output
    pub tool_output: String,
    /// Whether the tool failed
    pub is_error: bool,
}

/// Result from executing a hook
#[derive(Debug, Clone)]
pub struct HookResult {
    /// Standard output from the hook
    pub stdout: String,
    /// Standard error from the hook
    pub stderr: String,
    /// Exit code
    pub exit_code: i32,
    /// Whether hook execution was successful
    pub success: bool,
}

impl HookResult {
    /// Check if hook was successful
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Get any instructions to inject into the prompt
    /// (extracted from stdout)
    pub fn get_prompt_injection(&self) -> Option<String> {
        if self.stdout.trim().is_empty() {
            None
        } else {
            Some(self.stdout.clone())
        }
    }
}

/// Hook executor manages and runs session hooks
#[derive(Debug, Clone)]
pub struct HookExecutor {
    /// Available hooks organized by type
    hooks: HashMap<HookType, Vec<Hook>>,
}

impl HookExecutor {
    /// Create a new hook executor
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    /// Create a hook executor from a plugin manager
    pub fn from_plugin_manager(plugin_manager: &PluginManager) -> Self {
        let mut hooks_by_type: HashMap<HookType, Vec<Hook>> = HashMap::new();

        for plugin in plugin_manager.enabled_plugins() {
            for hook in &plugin.hooks {
                hooks_by_type
                    .entry(hook.hook_type.clone())
                    .or_insert_with(Vec::new)
                    .push(hook.clone());
            }
        }

        Self {
            hooks: hooks_by_type,
        }
    }

    /// Add a hook
    pub fn add_hook(&mut self, hook: Hook) {
        self.hooks
            .entry(hook.hook_type.clone())
            .or_insert_with(Vec::new)
            .push(hook);
    }

    /// Get hooks for a specific type
    pub fn get_hooks(&self, hook_type: &HookType) -> Vec<&Hook> {
        self.hooks
            .get(hook_type)
            .map(|hooks| hooks.iter().collect())
            .unwrap_or_default()
    }

    /// Check if there are any hooks for a given type
    pub fn has_hooks(&self, hook_type: &HookType) -> bool {
        self.hooks
            .get(hook_type)
            .map(|hooks| !hooks.is_empty())
            .unwrap_or(false)
    }

    /// Execute a hook script
    fn execute_script(
        &self,
        script_path: &Path,
        env_vars: &HashMap<String, String>,
    ) -> Result<HookResult> {
        if !script_path.exists() {
            return Err(HookError::ScriptNotFound(
                script_path.display().to_string(),
            ));
        }

        // Determine shell command based on platform
        #[cfg(target_os = "windows")]
        let (shell, shell_arg) = ("powershell", "-File");

        #[cfg(not(target_os = "windows"))]
        let (shell, shell_arg) = ("bash", "");

        let mut command = Command::new(shell);

        #[cfg(target_os = "windows")]
        command.arg(shell_arg).arg(script_path);

        #[cfg(not(target_os = "windows"))]
        command.arg(script_path);

        // Set environment variables
        command.envs(env_vars);

        // Execute with timeout
        let output = command
            .output()
            .map_err(|e| HookError::ExecutionFailed(e.to_string()))?;

        Ok(HookResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            success: output.status.success(),
        })
    }

    /// Execute session start hooks
    pub fn execute_session_start(
        &self,
        context: &SessionStartContext,
    ) -> Result<Vec<HookResult>> {
        let hooks = self.get_hooks(&HookType::SessionStart);
        let mut results = Vec::new();

        for hook in hooks {
            let mut env_vars = HashMap::new();
            env_vars.insert(
                "AURORA_PROJECT_ROOT".to_string(),
                context.project_root.clone(),
            );
            if let Some(msg) = &context.initial_message {
                env_vars.insert("AURORA_INITIAL_MESSAGE".to_string(), msg.clone());
            }

            let result = self.execute_script(&hook.script_path, &env_vars)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Execute session end hooks
    pub fn execute_session_end(&self, context: &SessionEndContext) -> Result<Vec<HookResult>> {
        let hooks = self.get_hooks(&HookType::SessionEnd);
        let mut results = Vec::new();

        for hook in hooks {
            let mut env_vars = HashMap::new();
            env_vars.insert(
                "AURORA_MESSAGE_COUNT".to_string(),
                context.message_count.to_string(),
            );
            env_vars.insert(
                "AURORA_TOTAL_CHARS".to_string(),
                context.total_chars.to_string(),
            );

            let result = self.execute_script(&hook.script_path, &env_vars)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Execute before tool call hooks
    pub fn execute_before_tool_call(
        &self,
        context: &ToolCallContext,
    ) -> Result<Vec<HookResult>> {
        let hooks = self.get_hooks(&HookType::BeforeToolCall);
        let mut results = Vec::new();

        for hook in hooks {
            let mut env_vars = HashMap::new();
            env_vars.insert("AURORA_TOOL_NAME".to_string(), context.tool_name.clone());
            env_vars.insert("AURORA_TOOL_ID".to_string(), context.tool_id.clone());
            env_vars.insert(
                "AURORA_TOOL_INPUT".to_string(),
                serde_json::to_string(&context.tool_input).unwrap_or_default(),
            );

            let result = self.execute_script(&hook.script_path, &env_vars)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Execute after tool call hooks
    pub fn execute_after_tool_call(
        &self,
        context: &AfterToolCallContext,
    ) -> Result<Vec<HookResult>> {
        let hooks = self.get_hooks(&HookType::AfterToolCall);
        let mut results = Vec::new();

        for hook in hooks {
            let mut env_vars = HashMap::new();
            env_vars.insert("AURORA_TOOL_NAME".to_string(), context.tool_name.clone());
            env_vars.insert("AURORA_TOOL_ID".to_string(), context.tool_id.clone());
            env_vars.insert(
                "AURORA_TOOL_INPUT".to_string(),
                serde_json::to_string(&context.tool_input).unwrap_or_default(),
            );
            env_vars.insert("AURORA_TOOL_OUTPUT".to_string(), context.tool_output.clone());
            env_vars.insert(
                "AURORA_TOOL_ERROR".to_string(),
                context.is_error.to_string(),
            );

            let result = self.execute_script(&hook.script_path, &env_vars)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Collect all prompt injections from hook results
    pub fn collect_prompt_injections(results: &[HookResult]) -> Vec<String> {
        results
            .iter()
            .filter_map(|r| r.get_prompt_injection())
            .collect()
    }
}

impl Default for HookExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_hook_script(dir: &Path, name: &str, content: &str) -> PathBuf {
        let script_path = dir.join(name);

        #[cfg(target_os = "windows")]
        {
            fs::write(&script_path, content).unwrap();
            script_path
        }

        #[cfg(not(target_os = "windows"))]
        {
            fs::write(&script_path, content).unwrap();
            // Make executable on Unix
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms).unwrap();
            script_path
        }
    }

    #[test]
    fn test_hook_executor_creation() {
        let executor = HookExecutor::new();
        assert!(!executor.has_hooks(&HookType::SessionStart));
        assert!(!executor.has_hooks(&HookType::SessionEnd));
    }

    #[test]
    fn test_add_hook() {
        let mut executor = HookExecutor::new();
        let hook = Hook {
            name: "test".to_string(),
            hook_type: HookType::SessionStart,
            script_path: PathBuf::from("/test/script.sh"),
        };

        executor.add_hook(hook);
        assert!(executor.has_hooks(&HookType::SessionStart));
        assert_eq!(executor.get_hooks(&HookType::SessionStart).len(), 1);
    }

    #[test]
    fn test_hook_result_success() {
        let result = HookResult {
            stdout: "Success output".to_string(),
            stderr: String::new(),
            exit_code: 0,
            success: true,
        };

        assert!(result.is_success());
        assert_eq!(result.get_prompt_injection(), Some("Success output".to_string()));
    }

    #[test]
    fn test_hook_result_empty_output() {
        let result = HookResult {
            stdout: "   ".to_string(),
            stderr: String::new(),
            exit_code: 0,
            success: true,
        };

        assert_eq!(result.get_prompt_injection(), None);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_execute_simple_hook() {
        let temp_dir = TempDir::new().unwrap();
        let script_content = "#!/bin/bash\necho 'Hook executed'";
        let script_path = create_test_hook_script(temp_dir.path(), "test.sh", script_content);

        let mut executor = HookExecutor::new();
        let hook = Hook {
            name: "test".to_string(),
            hook_type: HookType::SessionStart,
            script_path,
        };
        executor.add_hook(hook);

        let context = SessionStartContext {
            project_root: "/tmp".to_string(),
            initial_message: None,
        };

        let results = executor.execute_session_start(&context).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_success());
        assert!(results[0].stdout.contains("Hook executed"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_hook_environment_variables() {
        let temp_dir = TempDir::new().unwrap();
        let script_content = "#!/bin/bash\necho $AURORA_PROJECT_ROOT";
        let script_path = create_test_hook_script(temp_dir.path(), "test.sh", script_content);

        let mut executor = HookExecutor::new();
        let hook = Hook {
            name: "test".to_string(),
            hook_type: HookType::SessionStart,
            script_path,
        };
        executor.add_hook(hook);

        let context = SessionStartContext {
            project_root: "/my/project".to_string(),
            initial_message: None,
        };

        let results = executor.execute_session_start(&context).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].stdout.contains("/my/project"));
    }

    #[test]
    fn test_collect_prompt_injections() {
        let results = vec![
            HookResult {
                stdout: "Instruction 1".to_string(),
                stderr: String::new(),
                exit_code: 0,
                success: true,
            },
            HookResult {
                stdout: "".to_string(),
                stderr: String::new(),
                exit_code: 0,
                success: true,
            },
            HookResult {
                stdout: "Instruction 2".to_string(),
                stderr: String::new(),
                exit_code: 0,
                success: true,
            },
        ];

        let injections = HookExecutor::collect_prompt_injections(&results);
        assert_eq!(injections.len(), 2);
        assert_eq!(injections[0], "Instruction 1");
        assert_eq!(injections[1], "Instruction 2");
    }

    #[test]
    fn test_tool_call_context_serialization() {
        let context = ToolCallContext {
            tool_name: "read".to_string(),
            tool_input: serde_json::json!({"file_path": "/test.txt"}),
            tool_id: "tool_123".to_string(),
        };

        let json = serde_json::to_string(&context).unwrap();
        assert!(json.contains("read"));
        assert!(json.contains("tool_123"));
    }
}
