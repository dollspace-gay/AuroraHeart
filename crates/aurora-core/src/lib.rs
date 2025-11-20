//! Aurora Core - Core types, configuration, and utilities for AuroraHeart IDE
//!
//! This crate provides the foundational types and utilities used across the AuroraHeart
//! IDE, including:
//! - Configuration management
//! - Encrypted credential storage
//! - Shared types and error handling
//! - File I/O utilities
//! - Project detection and analysis

pub mod config;
pub mod crypto;
pub mod error;
pub mod file_io;
pub mod plugin;
pub mod project;
pub mod types;
pub mod command;
pub mod hooks;

pub use command::{CommandError, CommandExecutor, ParsedCommand, find_commands_in_text};
pub use hooks::{
    HookError, HookExecutor, HookResult, SessionStartContext, SessionEndContext,
    ToolCallContext, AfterToolCallContext,
};
pub use config::{Config, ConfigError};
pub use crypto::{CredentialStore, CredentialStoreError};
pub use error::{AuroraCoreError, Result};
pub use file_io::{FileIoError, read_file, read_file_content, write_file, write_file_content};
pub use plugin::{
    AgentDefinition, AgentInfo, CommandDefinition, Hook, HookType, Plugin, PluginError,
    PluginManager, PluginMetadata,
};
pub use project::{Language, ProjectError, detect_language, find_project_root, get_project_name};
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_imports() {
        // Verify that basic types are accessible
        let _result: Result<()> = Ok(());
    }
}
