//! Aurora Agent - AI agent core for AuroraHeart IDE
//!
//! This crate provides the AI agent functionality for AuroraHeart, including:
//! - Anthropic API client with streaming support
//! - Tool system (Read, Write, Edit, Bash, Grep, Glob, Task)
//! - Conversation management
//! - Directive loading and assembly

pub mod client;
pub mod tools;
pub mod conversation;
pub mod directives;

pub use client::{AnthropicClient, ClientError};
pub use conversation::{Conversation, Message, Role};
pub use directives::DirectiveManager;

/// Result type alias for agent operations
pub type Result<T> = std::result::Result<T, anyhow::Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_imports() {
        // Verify that basic types are accessible
        let _result: Result<()> = Ok(());
    }
}
