//! Configuration management for AuroraHeart
//!
//! This module handles loading, saving, and managing project-specific configuration
//! stored in `.AuroraHeart/config.toml` files.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during configuration operations
#[derive(Error, Debug)]
pub enum ConfigError {
    /// IO error while reading or writing config file
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML parsing error
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// TOML serialization error
    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// Config directory not found
    #[error("Config directory not found: {0}")]
    ConfigDirNotFound(PathBuf),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

/// Main configuration structure for AuroraHeart
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Project-specific settings
    #[serde(default)]
    pub project: ProjectConfig,

    /// AI agent settings
    #[serde(default)]
    pub agent: AgentConfig,

    /// Editor settings
    #[serde(default)]
    pub editor: EditorConfig,
}

/// Project-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectConfig {
    /// Project name
    pub name: Option<String>,

    /// Primary programming language
    pub language: Option<String>,

    /// Project root directory
    #[serde(skip)]
    pub root: Option<PathBuf>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: None,
            language: None,
            root: None,
        }
    }
}

/// AI agent configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    /// Model to use (e.g., "claude-sonnet-4")
    #[serde(default = "default_model")]
    pub model: String,

    /// Maximum tokens for context
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,

    /// Enabled directive modules
    #[serde(default)]
    pub enabled_directives: Vec<String>,
}

fn default_model() -> String {
    "claude-sonnet-4".to_string()
}

fn default_max_tokens() -> usize {
    200000
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            max_tokens: default_max_tokens(),
            enabled_directives: Vec::new(),
        }
    }
}

/// Editor configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorConfig {
    /// Tab size in spaces
    #[serde(default = "default_tab_size")]
    pub tab_size: usize,

    /// Use spaces instead of tabs
    #[serde(default = "default_use_spaces")]
    pub use_spaces: bool,

    /// Show line numbers
    #[serde(default = "default_show_line_numbers")]
    pub show_line_numbers: bool,
}

fn default_tab_size() -> usize {
    4
}

fn default_use_spaces() -> bool {
    true
}

fn default_show_line_numbers() -> bool {
    true
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: default_tab_size(),
            use_spaces: default_use_spaces(),
            show_line_numbers: default_show_line_numbers(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project: ProjectConfig::default(),
            agent: AgentConfig::default(),
            editor: EditorConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from a .AuroraHeart directory
    pub fn load<P: AsRef<Path>>(project_root: P) -> Result<Self, ConfigError> {
        let config_path = project_root.as_ref().join(".AuroraHeart").join("config.toml");

        if !config_path.exists() {
            // Return default config if file doesn't exist
            let mut config = Config::default();
            config.project.root = Some(project_root.as_ref().to_path_buf());
            return Ok(config);
        }

        let contents = std::fs::read_to_string(&config_path)?;
        let mut config: Config = toml::from_str(&contents)?;
        config.project.root = Some(project_root.as_ref().to_path_buf());

        Ok(config)
    }

    /// Save configuration to a .AuroraHeart directory
    pub fn save<P: AsRef<Path>>(&self, project_root: P) -> Result<(), ConfigError> {
        let config_dir = project_root.as_ref().join(".AuroraHeart");
        let config_path = config_dir.join("config.toml");

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, contents)?;

        Ok(())
    }

    /// Get the config directory path
    pub fn config_dir(&self) -> Option<PathBuf> {
        self.project.root.as_ref().map(|root| root.join(".AuroraHeart"))
    }

    /// Get the directives directory path
    pub fn directives_dir(&self) -> Option<PathBuf> {
        self.config_dir().map(|dir| dir.join("directives"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.agent.model, "claude-sonnet-4");
        assert_eq!(config.agent.max_tokens, 200000);
        assert_eq!(config.editor.tab_size, 4);
        assert!(config.editor.use_spaces);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create and save config
        let mut config = Config::default();
        config.project.name = Some("TestProject".to_string());
        config.agent.model = "claude-opus-4".to_string();

        config.save(project_root).unwrap();

        // Load config
        let loaded_config = Config::load(project_root).unwrap();
        assert_eq!(loaded_config.project.name, Some("TestProject".to_string()));
        assert_eq!(loaded_config.agent.model, "claude-opus-4");
    }

    #[test]
    fn test_config_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Should return default config when file doesn't exist
        let config = Config::load(project_root).unwrap();
        assert_eq!(config.agent.model, "claude-sonnet-4");
        assert_eq!(config.project.root, Some(project_root.to_path_buf()));
    }

    #[test]
    fn test_config_directories() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.project.root = Some(temp_dir.path().to_path_buf());

        let config_dir = config.config_dir().unwrap();
        assert_eq!(config_dir, temp_dir.path().join(".AuroraHeart"));

        let directives_dir = config.directives_dir().unwrap();
        assert_eq!(directives_dir, temp_dir.path().join(".AuroraHeart").join("directives"));
    }
}
