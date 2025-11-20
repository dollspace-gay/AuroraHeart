//! Plugin system for AuroraHeart IDE
//!
//! This module provides the plugin architecture that allows extending AuroraHeart
//! with custom agents, commands, and session hooks through `.AuroraHeart/plugins/`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Plugin system errors
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin directory not found: {0}")]
    PluginDirNotFound(PathBuf),

    #[error("Failed to read plugin.toml: {0}")]
    PluginTomlReadError(String),

    #[error("Failed to parse plugin.toml: {0}")]
    PluginTomlParseError(String),

    #[error("Missing required field in plugin.toml: {0}")]
    MissingRequiredField(String),

    #[error("Plugin dependency not met: {0}")]
    DependencyNotMet(String),

    #[error("Plugin conflict detected: {0}")]
    PluginConflict(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, PluginError>;

/// Plugin metadata from plugin.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub plugin: PluginInfo,
}

/// Core plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,

    #[serde(default)]
    pub dependencies: PluginDependencies,
}

/// Plugin dependencies
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginDependencies {
    #[serde(default)]
    pub required_tools: Vec<String>,

    #[serde(default)]
    pub required_plugins: Vec<String>,
}

/// Agent definition from agents/*.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub agent: AgentInfo,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub description: String,

    #[serde(default = "default_model")]
    pub model: String,

    pub system_prompt: AgentSystemPrompt,

    #[serde(default)]
    pub tools: AgentTools,

    #[serde(default)]
    pub behavior: AgentBehavior,
}

fn default_model() -> String {
    "sonnet".to_string()
}

/// Agent system prompt configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSystemPrompt {
    pub role: String,
    pub instructions: String,
}

/// Agent tool permissions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentTools {
    #[serde(default)]
    pub allowed: Vec<String>,

    #[serde(default)]
    pub denied: Vec<String>,
}

/// Agent behavioral settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBehavior {
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,

    #[serde(default)]
    pub stop_on_error: bool,
}

fn default_max_turns() -> u32 {
    10
}

impl Default for AgentBehavior {
    fn default() -> Self {
        Self {
            max_turns: default_max_turns(),
            stop_on_error: false,
        }
    }
}

/// Slash command definition
#[derive(Debug, Clone)]
pub struct CommandDefinition {
    pub name: String,
    pub content: String,
    pub path: PathBuf,
}

/// Session hook definition
#[derive(Debug, Clone)]
pub struct Hook {
    pub name: String,
    pub hook_type: HookType,
    pub script_path: PathBuf,
}

/// Hook types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HookType {
    SessionStart,
    SessionEnd,
    BeforeToolCall,
    AfterToolCall,
}

/// Loaded plugin with all its components
#[derive(Debug, Clone)]
pub struct Plugin {
    pub metadata: PluginMetadata,
    pub path: PathBuf,
    pub agents: HashMap<String, AgentDefinition>,
    pub commands: HashMap<String, CommandDefinition>,
    pub hooks: Vec<Hook>,
    pub enabled: bool,
}

impl Plugin {
    /// Load a plugin from a directory
    pub fn load<P: AsRef<Path>>(plugin_dir: P) -> Result<Self> {
        let plugin_dir = plugin_dir.as_ref();

        if !plugin_dir.exists() {
            return Err(PluginError::PluginDirNotFound(plugin_dir.to_path_buf()));
        }

        // Load plugin.toml
        let plugin_toml_path = plugin_dir.join("plugin.toml");
        if !plugin_toml_path.exists() {
            return Err(PluginError::MissingRequiredField("plugin.toml".to_string()));
        }

        let plugin_toml_content = std::fs::read_to_string(&plugin_toml_path)
            .map_err(|e| PluginError::PluginTomlReadError(e.to_string()))?;

        let metadata: PluginMetadata = toml::from_str(&plugin_toml_content)?;

        // Load agents
        let agents_dir = plugin_dir.join("agents");
        let mut agents = HashMap::new();
        if agents_dir.exists() {
            for entry in std::fs::read_dir(&agents_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    let content = std::fs::read_to_string(&path)?;
                    let agent_def: AgentDefinition = toml::from_str(&content)?;
                    agents.insert(agent_def.agent.name.clone(), agent_def);
                }
            }
        }

        // Load commands
        let commands_dir = plugin_dir.join("commands");
        let mut commands = HashMap::new();
        if commands_dir.exists() {
            for entry in std::fs::read_dir(&commands_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    let content = std::fs::read_to_string(&path)?;
                    let name = path.file_stem()
                        .and_then(|s| s.to_str())
                        .ok_or_else(|| PluginError::PluginTomlParseError(
                            "Invalid command filename".to_string()
                        ))?
                        .to_string();

                    commands.insert(name.clone(), CommandDefinition {
                        name,
                        content,
                        path: path.clone(),
                    });
                }
            }
        }

        // Load hooks
        let hooks_dir = plugin_dir.join("hooks");
        let mut hooks = Vec::new();
        if hooks_dir.exists() {
            // Look for session-start hooks
            let session_start_path = hooks_dir.join("session-start.sh");
            if session_start_path.exists() {
                hooks.push(Hook {
                    name: format!("{}-session-start", metadata.plugin.name),
                    hook_type: HookType::SessionStart,
                    script_path: session_start_path,
                });
            }

            // Look for session-end hooks
            let session_end_path = hooks_dir.join("session-end.sh");
            if session_end_path.exists() {
                hooks.push(Hook {
                    name: format!("{}-session-end", metadata.plugin.name),
                    hook_type: HookType::SessionEnd,
                    script_path: session_end_path,
                });
            }
        }

        Ok(Plugin {
            metadata,
            path: plugin_dir.to_path_buf(),
            agents,
            commands,
            hooks,
            enabled: true,
        })
    }

    /// Validate plugin dependencies
    pub fn validate_dependencies(&self, available_tools: &[String]) -> Result<()> {
        for required_tool in &self.metadata.plugin.dependencies.required_tools {
            if !available_tools.contains(required_tool) {
                return Err(PluginError::DependencyNotMet(format!(
                    "Required tool '{}' not available",
                    required_tool
                )));
            }
        }
        Ok(())
    }
}

/// Plugin manager for loading and managing all plugins
#[derive(Debug, Clone)]
pub struct PluginManager {
    pub plugins: HashMap<String, Plugin>,
    pub plugins_dir: PathBuf,
}

impl PluginManager {
    /// Create a new plugin manager for a project
    pub fn new<P: AsRef<Path>>(project_root: P) -> Self {
        let plugins_dir = project_root.as_ref().join(".AuroraHeart").join("plugins");
        Self {
            plugins: HashMap::new(),
            plugins_dir,
        }
    }

    /// Discover and load all plugins
    pub fn discover_plugins(&mut self) -> Result<()> {
        if !self.plugins_dir.exists() {
            // Create plugins directory if it doesn't exist
            std::fs::create_dir_all(&self.plugins_dir)?;
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                match Plugin::load(&path) {
                    Ok(plugin) => {
                        let name = plugin.metadata.plugin.name.clone();
                        self.plugins.insert(name, plugin);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load plugin from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get all enabled plugins
    pub fn enabled_plugins(&self) -> Vec<&Plugin> {
        self.plugins.values().filter(|p| p.enabled).collect()
    }

    /// Get all agents from enabled plugins
    pub fn get_all_agents(&self) -> HashMap<String, &AgentDefinition> {
        let mut agents = HashMap::new();
        for plugin in self.enabled_plugins() {
            for (name, agent) in &plugin.agents {
                agents.insert(name.clone(), agent);
            }
        }
        agents
    }

    /// Get all commands from enabled plugins
    pub fn get_all_commands(&self) -> HashMap<String, &CommandDefinition> {
        let mut commands = HashMap::new();
        for plugin in self.enabled_plugins() {
            for (name, command) in &plugin.commands {
                commands.insert(name.clone(), command);
            }
        }
        commands
    }

    /// Get all session start hooks from enabled plugins
    pub fn get_session_start_hooks(&self) -> Vec<&Hook> {
        let mut hooks = Vec::new();
        for plugin in self.enabled_plugins() {
            for hook in &plugin.hooks {
                if hook.hook_type == HookType::SessionStart {
                    hooks.push(hook);
                }
            }
        }
        hooks
    }

    /// Enable a plugin
    pub fn enable_plugin(&mut self, plugin_name: &str) {
        if let Some(plugin) = self.plugins.get_mut(plugin_name) {
            plugin.enabled = true;
        }
    }

    /// Disable a plugin
    pub fn disable_plugin(&mut self, plugin_name: &str) {
        if let Some(plugin) = self.plugins.get_mut(plugin_name) {
            plugin.enabled = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_plugin(base_dir: &Path) -> Result<()> {
        let plugin_dir = base_dir.join(".AuroraHeart").join("plugins").join("test-plugin");
        fs::create_dir_all(&plugin_dir)?;

        // Create plugin.toml
        let plugin_toml = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"
description = "Test plugin"
author = "Test Author"

[plugin.dependencies]
required_tools = ["read", "write"]
"#;
        fs::write(plugin_dir.join("plugin.toml"), plugin_toml)?;

        // Create an agent
        let agents_dir = plugin_dir.join("agents");
        fs::create_dir_all(&agents_dir)?;
        let agent_toml = r#"
[agent]
name = "test-agent"
description = "Test agent"
model = "sonnet"

[agent.system_prompt]
role = "You are a test agent"
instructions = "Do test things"

[agent.tools]
allowed = ["read"]
denied = ["write"]

[agent.behavior]
max_turns = 5
stop_on_error = true
"#;
        fs::write(agents_dir.join("test-agent.toml"), agent_toml)?;

        // Create a command
        let commands_dir = plugin_dir.join("commands");
        fs::create_dir_all(&commands_dir)?;
        fs::write(commands_dir.join("test-command.md"), "# Test Command\nDo something")?;

        Ok(())
    }

    #[test]
    fn test_plugin_loading() {
        let temp_dir = std::env::temp_dir().join("aurora_test_plugin");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        setup_test_plugin(&temp_dir).unwrap();

        let plugin_dir = temp_dir.join(".AuroraHeart").join("plugins").join("test-plugin");
        let plugin = Plugin::load(&plugin_dir).unwrap();

        assert_eq!(plugin.metadata.plugin.name, "test-plugin");
        assert_eq!(plugin.metadata.plugin.version, "1.0.0");
        assert_eq!(plugin.agents.len(), 1);
        assert_eq!(plugin.commands.len(), 1);
        assert!(plugin.agents.contains_key("test-agent"));
        assert!(plugin.commands.contains_key("test-command"));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_plugin_manager() {
        let temp_dir = std::env::temp_dir().join("aurora_test_manager");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        setup_test_plugin(&temp_dir).unwrap();

        let mut manager = PluginManager::new(&temp_dir);
        manager.discover_plugins().unwrap();

        assert_eq!(manager.plugins.len(), 1);
        assert!(manager.plugins.contains_key("test-plugin"));

        let agents = manager.get_all_agents();
        assert_eq!(agents.len(), 1);
        assert!(agents.contains_key("test-agent"));

        let commands = manager.get_all_commands();
        assert_eq!(commands.len(), 1);
        assert!(commands.contains_key("test-command"));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_dependency_validation() {
        let temp_dir = std::env::temp_dir().join("aurora_test_deps");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        setup_test_plugin(&temp_dir).unwrap();

        let plugin_dir = temp_dir.join(".AuroraHeart").join("plugins").join("test-plugin");
        let plugin = Plugin::load(&plugin_dir).unwrap();

        // Should succeed with required tools
        let available_tools = vec!["read".to_string(), "write".to_string()];
        assert!(plugin.validate_dependencies(&available_tools).is_ok());

        // Should fail without required tools
        let insufficient_tools = vec!["read".to_string()];
        assert!(plugin.validate_dependencies(&insufficient_tools).is_err());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
