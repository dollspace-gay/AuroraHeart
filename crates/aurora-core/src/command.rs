//! Slash command system for workflow automation
//!
//! This module provides command parsing, execution, and management for
//! slash commands in the AuroraHeart IDE. Commands are loaded from
//! .AuroraHeart/commands/*.md files and can be invoked using /command syntax.

use crate::plugin::{CommandDefinition, PluginManager};
use regex::Regex;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during command operations
#[derive(Error, Debug)]
pub enum CommandError {
    /// Command not found
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    /// Invalid command syntax
    #[error("Invalid command syntax: {0}")]
    InvalidSyntax(String),

    /// Command parsing error
    #[error("Command parse error: {0}")]
    ParseError(String),

    /// Missing required parameter
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),
}

pub type Result<T> = std::result::Result<T, CommandError>;

/// Parsed slash command with name and arguments
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    /// Command name (without the / prefix)
    pub name: String,
    /// Command arguments (everything after the command name)
    pub args: Option<String>,
    /// Original full text
    pub original: String,
}

impl ParsedCommand {
    /// Create a new parsed command
    pub fn new(name: impl Into<String>, args: Option<String>, original: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args,
            original: original.into(),
        }
    }

    /// Get the command name
    pub fn command_name(&self) -> &str {
        &self.name
    }

    /// Get the command arguments
    pub fn arguments(&self) -> Option<&str> {
        self.args.as_deref()
    }

    /// Check if command has arguments
    pub fn has_arguments(&self) -> bool {
        self.args.is_some()
    }
}

/// Command executor that manages and executes slash commands
#[derive(Debug, Clone)]
pub struct CommandExecutor {
    /// Available commands (name -> definition)
    commands: HashMap<String, CommandDefinition>,
}

impl CommandExecutor {
    /// Create a new command executor
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Create a command executor from a plugin manager
    pub fn from_plugin_manager(plugin_manager: &PluginManager) -> Self {
        let commands = plugin_manager
            .get_all_commands()
            .into_iter()
            .map(|(name, def)| (name, def.clone()))
            .collect();

        Self { commands }
    }

    /// Add a command definition
    pub fn add_command(&mut self, command: CommandDefinition) {
        self.commands.insert(command.name.clone(), command);
    }

    /// Add multiple commands
    pub fn add_commands(&mut self, commands: Vec<CommandDefinition>) {
        for command in commands {
            self.add_command(command);
        }
    }

    /// Get a command by name
    pub fn get_command(&self, name: &str) -> Option<&CommandDefinition> {
        self.commands.get(name)
    }

    /// List all available command names
    pub fn list_commands(&self) -> Vec<String> {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.sort();
        names
    }

    /// Check if a command exists
    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// Parse a slash command from user input
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let cmd = CommandExecutor::parse_command("/code-review")?;
    /// assert_eq!(cmd.name, "code-review");
    ///
    /// let cmd = CommandExecutor::parse_command("/explain this function")?;
    /// assert_eq!(cmd.name, "explain");
    /// assert_eq!(cmd.args, Some("this function".to_string()));
    /// ```
    pub fn parse_command(input: &str) -> Result<ParsedCommand> {
        let input = input.trim();

        // Check if input starts with /
        if !input.starts_with('/') {
            return Err(CommandError::InvalidSyntax(
                "Command must start with /".to_string(),
            ));
        }

        // Remove the / prefix
        let without_slash = &input[1..];

        // Split at first whitespace to separate command name from args
        let parts: Vec<&str> = without_slash.splitn(2, |c: char| c.is_whitespace()).collect();

        if parts.is_empty() || parts[0].is_empty() {
            return Err(CommandError::InvalidSyntax(
                "Command name cannot be empty".to_string(),
            ));
        }

        let name = parts[0].to_string();
        let args = if parts.len() > 1 {
            let arg_str = parts[1].trim();
            if arg_str.is_empty() {
                None
            } else {
                Some(arg_str.to_string())
            }
        } else {
            None
        };

        Ok(ParsedCommand::new(name, args, input))
    }

    /// Check if input contains a slash command
    pub fn contains_command(input: &str) -> bool {
        let input = input.trim();
        input.starts_with('/')
    }

    /// Execute a command and get its content
    ///
    /// Returns the command content that should be injected into the conversation
    pub fn execute(&self, parsed: &ParsedCommand) -> Result<String> {
        let command_def = self
            .get_command(&parsed.name)
            .ok_or_else(|| CommandError::CommandNotFound(parsed.name.clone()))?;

        let mut content = command_def.content.clone();

        // If command has arguments, append them to the content
        if let Some(args) = &parsed.args {
            content.push_str("\n\n");
            content.push_str("User input: ");
            content.push_str(args);
        }

        Ok(content)
    }

    /// Execute a command from raw input string
    ///
    /// Combines parsing and execution in one step
    pub fn execute_from_input(&self, input: &str) -> Result<String> {
        let parsed = Self::parse_command(input)?;
        self.execute(&parsed)
    }

    /// Get command suggestions based on partial input
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let suggestions = executor.suggest_commands("/cod");
    /// // Returns ["code-review"] if that command exists
    /// ```
    pub fn suggest_commands(&self, partial: &str) -> Vec<String> {
        let partial = partial.trim();

        // Remove leading / if present
        let search_term = if partial.starts_with('/') {
            &partial[1..]
        } else {
            partial
        };

        let mut suggestions: Vec<String> = self
            .commands
            .keys()
            .filter(|name| name.starts_with(search_term))
            .cloned()
            .collect();

        suggestions.sort();
        suggestions
    }

    /// Get help text for a specific command
    pub fn get_command_help(&self, command_name: &str) -> Result<String> {
        let command = self
            .get_command(command_name)
            .ok_or_else(|| CommandError::CommandNotFound(command_name.to_string()))?;

        // Extract first paragraph or first 200 chars as help text
        let lines: Vec<&str> = command.content.lines().take(3).collect();
        let help = lines.join("\n");

        Ok(format!("/{} - {}", command.name, help))
    }

    /// Get help text for all commands
    pub fn get_all_commands_help(&self) -> String {
        let mut help = String::from("Available slash commands:\n\n");

        for name in self.list_commands() {
            if let Some(command) = self.get_command(&name) {
                // Get first line as description
                let description = command
                    .content
                    .lines()
                    .next()
                    .unwrap_or("No description")
                    .trim_start_matches('#')
                    .trim();

                help.push_str(&format!("  /{} - {}\n", name, description));
            }
        }

        help.push_str("\nType /help <command> for more details on a specific command.");
        help
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect slash commands in a message and return their positions
pub fn find_commands_in_text(text: &str) -> Vec<(usize, usize, String)> {
    let re = Regex::new(r"/([a-zA-Z][a-zA-Z0-9-]*)").unwrap();
    re.captures_iter(text)
        .filter_map(|cap| {
            cap.get(0).map(|m| {
                let start = m.start();
                let end = m.end();
                let command = m.as_str().to_string();
                (start, end, command)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_command(name: &str, content: &str) -> CommandDefinition {
        CommandDefinition {
            name: name.to_string(),
            content: content.to_string(),
            path: PathBuf::from(format!("/test/{}.md", name)),
        }
    }

    #[test]
    fn test_parse_command_simple() {
        let parsed = CommandExecutor::parse_command("/code-review").unwrap();
        assert_eq!(parsed.name, "code-review");
        assert_eq!(parsed.args, None);
        assert_eq!(parsed.original, "/code-review");
    }

    #[test]
    fn test_parse_command_with_args() {
        let parsed = CommandExecutor::parse_command("/explain this function").unwrap();
        assert_eq!(parsed.name, "explain");
        assert_eq!(parsed.args, Some("this function".to_string()));
    }

    #[test]
    fn test_parse_command_with_trailing_spaces() {
        let parsed = CommandExecutor::parse_command("/test    ").unwrap();
        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.args, None);
    }

    #[test]
    fn test_parse_command_invalid_no_slash() {
        let result = CommandExecutor::parse_command("code-review");
        assert!(result.is_err());
        match result {
            Err(CommandError::InvalidSyntax(_)) => {}
            _ => panic!("Expected InvalidSyntax error"),
        }
    }

    #[test]
    fn test_parse_command_invalid_empty_name() {
        let result = CommandExecutor::parse_command("/");
        assert!(result.is_err());
    }

    #[test]
    fn test_contains_command() {
        assert!(CommandExecutor::contains_command("/test"));
        assert!(CommandExecutor::contains_command("  /test"));
        assert!(!CommandExecutor::contains_command("test"));
        assert!(!CommandExecutor::contains_command(""));
    }

    #[test]
    fn test_command_executor_add_command() {
        let mut executor = CommandExecutor::new();
        let cmd = create_test_command("test", "Test command content");

        executor.add_command(cmd);

        assert!(executor.has_command("test"));
        assert!(!executor.has_command("nonexistent"));
    }

    #[test]
    fn test_command_executor_list_commands() {
        let mut executor = CommandExecutor::new();
        executor.add_command(create_test_command("zebra", "Content"));
        executor.add_command(create_test_command("alpha", "Content"));
        executor.add_command(create_test_command("beta", "Content"));

        let commands = executor.list_commands();
        assert_eq!(commands, vec!["alpha", "beta", "zebra"]);
    }

    #[test]
    fn test_command_executor_execute() {
        let mut executor = CommandExecutor::new();
        let cmd = create_test_command("test", "# Test Command\n\nDo something cool");

        executor.add_command(cmd);

        let parsed = CommandExecutor::parse_command("/test").unwrap();
        let result = executor.execute(&parsed).unwrap();

        assert_eq!(result, "# Test Command\n\nDo something cool");
    }

    #[test]
    fn test_command_executor_execute_with_args() {
        let mut executor = CommandExecutor::new();
        let cmd = create_test_command("explain", "Explain the following:");

        executor.add_command(cmd);

        let parsed = CommandExecutor::parse_command("/explain this function").unwrap();
        let result = executor.execute(&parsed).unwrap();

        assert!(result.contains("Explain the following:"));
        assert!(result.contains("User input: this function"));
    }

    #[test]
    fn test_command_executor_execute_not_found() {
        let executor = CommandExecutor::new();
        let parsed = CommandExecutor::parse_command("/nonexistent").unwrap();
        let result = executor.execute(&parsed);

        assert!(result.is_err());
        match result {
            Err(CommandError::CommandNotFound(_)) => {}
            _ => panic!("Expected CommandNotFound error"),
        }
    }

    #[test]
    fn test_command_executor_execute_from_input() {
        let mut executor = CommandExecutor::new();
        executor.add_command(create_test_command("test", "Test content"));

        let result = executor.execute_from_input("/test").unwrap();
        assert_eq!(result, "Test content");
    }

    #[test]
    fn test_suggest_commands() {
        let mut executor = CommandExecutor::new();
        executor.add_command(create_test_command("code-review", "Content"));
        executor.add_command(create_test_command("code-format", "Content"));
        executor.add_command(create_test_command("commit", "Content"));
        executor.add_command(create_test_command("test", "Content"));

        let suggestions = executor.suggest_commands("/cod");
        assert_eq!(suggestions, vec!["code-format", "code-review"]);

        let suggestions = executor.suggest_commands("/c");
        assert_eq!(suggestions, vec!["code-format", "code-review", "commit"]);

        let suggestions = executor.suggest_commands("cod");
        assert_eq!(suggestions, vec!["code-format", "code-review"]);
    }

    #[test]
    fn test_get_command_help() {
        let mut executor = CommandExecutor::new();
        executor.add_command(create_test_command(
            "test",
            "# Test Command\n\nThis is a test.\nSecond line.",
        ));

        let help = executor.get_command_help("test").unwrap();
        assert!(help.contains("/test"));
        assert!(help.contains("Test Command"));
    }

    #[test]
    fn test_get_all_commands_help() {
        let mut executor = CommandExecutor::new();
        executor.add_command(create_test_command("alpha", "# Alpha Command\nFirst"));
        executor.add_command(create_test_command("beta", "# Beta Command\nSecond"));

        let help = executor.get_all_commands_help();
        assert!(help.contains("/alpha"));
        assert!(help.contains("/beta"));
        assert!(help.contains("Alpha Command"));
        assert!(help.contains("Beta Command"));
    }

    #[test]
    fn test_find_commands_in_text() {
        let text = "Try using /code-review and /explain for help";
        let commands = find_commands_in_text(text);

        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].2, "/code-review");
        assert_eq!(commands[1].2, "/explain");
    }

    #[test]
    fn test_find_commands_in_text_no_commands() {
        let text = "This is just plain text";
        let commands = find_commands_in_text(text);
        assert_eq!(commands.len(), 0);
    }

    #[test]
    fn test_parsed_command_methods() {
        let cmd = ParsedCommand::new("test", Some("args".to_string()), "/test args");

        assert_eq!(cmd.command_name(), "test");
        assert_eq!(cmd.arguments(), Some("args"));
        assert!(cmd.has_arguments());

        let cmd_no_args = ParsedCommand::new("test", None, "/test");
        assert!(!cmd_no_args.has_arguments());
    }
}
