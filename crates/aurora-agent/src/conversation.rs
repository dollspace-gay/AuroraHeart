//! Conversation management for the AI agent

use serde::{Deserialize, Serialize};

/// Role of a message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User message
    User,
    /// Assistant (AI) message
    Assistant,
}

/// Content of a message - either simple text or structured content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content
    Text(String),
    /// Structured content blocks (for tool use/results)
    Blocks(Vec<crate::client::ContentBlock>),
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        MessageContent::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        MessageContent::Text(s.to_string())
    }
}

impl From<Vec<crate::client::ContentBlock>> for MessageContent {
    fn from(blocks: Vec<crate::client::ContentBlock>) -> Self {
        MessageContent::Blocks(blocks)
    }
}

impl PartialEq<String> for MessageContent {
    fn eq(&self, other: &String) -> bool {
        match self {
            MessageContent::Text(text) => text == other,
            MessageContent::Blocks(_) => false,
        }
    }
}

impl PartialEq<&str> for MessageContent {
    fn eq(&self, other: &&str) -> bool {
        match self {
            MessageContent::Text(text) => text == other,
            MessageContent::Blocks(_) => false,
        }
    }
}

impl MessageContent {
    /// Get text content if this is a text message, None otherwise
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(text) => Some(text),
            MessageContent::Blocks(_) => None,
        }
    }
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the sender
    pub role: Role,
    /// Content of the message
    pub content: MessageContent,
}

impl Message {
    /// Create a new user message with text content
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a new assistant message with text content
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a new user message with content blocks
    pub fn user_with_blocks(blocks: Vec<crate::client::ContentBlock>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Blocks(blocks),
        }
    }

    /// Create a new assistant message with content blocks
    pub fn assistant_with_blocks(blocks: Vec<crate::client::ContentBlock>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Blocks(blocks),
        }
    }

    /// Get the text content if this is a text message
    pub fn as_text(&self) -> Option<&str> {
        match &self.content {
            MessageContent::Text(text) => Some(text),
            MessageContent::Blocks(_) => None,
        }
    }

    /// Get the content blocks if this is a blocks message
    pub fn as_blocks(&self) -> Option<&[crate::client::ContentBlock]> {
        match &self.content {
            MessageContent::Text(_) => None,
            MessageContent::Blocks(blocks) => Some(blocks),
        }
    }

    /// Get the character count of this message
    pub fn char_count(&self) -> usize {
        match &self.content {
            MessageContent::Text(text) => text.len(),
            MessageContent::Blocks(blocks) => blocks.iter().map(|b| match b {
                crate::client::ContentBlock::Text { text } => text.len(),
                crate::client::ContentBlock::ToolUse { .. } => 50,  // Rough estimate
                crate::client::ContentBlock::ToolResult { content, .. } => content.len(),
            }).sum(),
        }
    }
}

/// A conversation with the AI agent
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    /// System prompt
    pub system_prompt: Option<String>,
    /// Messages in the conversation
    pub messages: Vec<Message>,
}

impl Conversation {
    /// Create a new empty conversation
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a conversation with a system prompt
    pub fn with_system_prompt(system_prompt: impl Into<String>) -> Self {
        Self {
            system_prompt: Some(system_prompt.into()),
            messages: Vec::new(),
        }
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Add a user message
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.add_message(Message::user(content));
    }

    /// Add an assistant message
    pub fn add_assistant_message(&mut self, content: impl Into<String>) {
        self.add_message(Message::assistant(content));
    }

    /// Get all messages
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Clear all messages (keeps system prompt)
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Get the total number of characters in the conversation
    pub fn total_chars(&self) -> usize {
        self.system_prompt.as_ref().map(|s| s.len()).unwrap_or(0)
            + self.messages.iter().map(|m| m.char_count()).sum::<usize>()
    }

    /// Get the number of messages in the conversation
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Truncate conversation to stay within a character limit
    /// Removes oldest messages while preserving recent context
    ///
    /// # Arguments
    /// * `max_chars` - Maximum number of characters to keep (excluding system prompt)
    ///
    /// # Returns
    /// Number of messages removed
    pub fn truncate_to_limit(&mut self, max_chars: usize) -> usize {
        let system_chars = self.system_prompt.as_ref().map(|s| s.len()).unwrap_or(0);
        let mut current_chars: usize = self.messages.iter().map(|m| m.char_count()).sum();

        if current_chars + system_chars <= max_chars {
            return 0; // No truncation needed
        }

        let mut removed_count = 0;
        let target_chars = max_chars.saturating_sub(system_chars);

        // Remove messages from the beginning until we're under the limit
        while current_chars > target_chars && !self.messages.is_empty() {
            if let Some(msg) = self.messages.first() {
                current_chars = current_chars.saturating_sub(msg.char_count());
            }
            self.messages.remove(0);
            removed_count += 1;
        }

        removed_count
    }

    /// Ensure conversation stays within token budget by truncating if needed
    /// Uses a rough approximation of 4 characters per token
    ///
    /// # Arguments
    /// * `max_tokens` - Maximum number of tokens to keep
    ///
    /// # Returns
    /// Number of messages removed
    pub fn truncate_to_tokens(&mut self, max_tokens: usize) -> usize {
        // Rough approximation: 4 chars per token
        let max_chars = max_tokens * 4;
        self.truncate_to_limit(max_chars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, Role::User);
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = Message::assistant("Hi there!");
        assert_eq!(assistant_msg.role, Role::Assistant);
        assert_eq!(assistant_msg.content, "Hi there!");
    }

    #[test]
    fn test_conversation_basics() {
        let mut conv = Conversation::new();
        assert_eq!(conv.messages().len(), 0);

        conv.add_user_message("Hello");
        assert_eq!(conv.messages().len(), 1);
        assert_eq!(conv.messages()[0].role, Role::User);

        conv.add_assistant_message("Hi!");
        assert_eq!(conv.messages().len(), 2);
        assert_eq!(conv.messages()[1].role, Role::Assistant);
    }

    #[test]
    fn test_conversation_with_system_prompt() {
        let conv = Conversation::with_system_prompt("You are helpful");
        assert_eq!(conv.system_prompt, Some("You are helpful".to_string()));
        assert_eq!(conv.messages().len(), 0);
    }

    #[test]
    fn test_conversation_clear() {
        let mut conv = Conversation::with_system_prompt("System");
        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi");
        assert_eq!(conv.messages().len(), 2);

        conv.clear();
        assert_eq!(conv.messages().len(), 0);
        assert_eq!(conv.system_prompt, Some("System".to_string()));
    }

    #[test]
    fn test_total_chars() {
        let mut conv = Conversation::with_system_prompt("12345");
        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi");

        // "12345" + "Hello" + "Hi" = 5 + 5 + 2 = 12
        assert_eq!(conv.total_chars(), 12);
    }

    #[test]
    fn test_message_count() {
        let mut conv = Conversation::new();
        assert_eq!(conv.message_count(), 0);

        conv.add_user_message("Hello");
        assert_eq!(conv.message_count(), 1);

        conv.add_assistant_message("Hi");
        assert_eq!(conv.message_count(), 2);
    }

    #[test]
    fn test_truncate_to_limit_no_truncation_needed() {
        let mut conv = Conversation::new();
        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi");

        let removed = conv.truncate_to_limit(100);
        assert_eq!(removed, 0);
        assert_eq!(conv.message_count(), 2);
    }

    #[test]
    fn test_truncate_to_limit_removes_oldest() {
        let mut conv = Conversation::new();
        conv.add_user_message("First message");  // 13 chars
        conv.add_assistant_message("Second");    // 6 chars
        conv.add_user_message("Third");          // 5 chars

        // Total: 24 chars, limit to 15 chars
        let removed = conv.truncate_to_limit(15);
        assert_eq!(removed, 1); // Should remove first message
        assert_eq!(conv.message_count(), 2);
        assert_eq!(conv.messages()[0].content, "Second");
        assert_eq!(conv.messages()[1].content, "Third");
    }

    #[test]
    fn test_truncate_to_limit_with_system_prompt() {
        let mut conv = Conversation::with_system_prompt("System prompt"); // 13 chars
        conv.add_user_message("Hello");  // 5 chars
        conv.add_assistant_message("Hi"); // 2 chars

        // Total: 13 + 5 + 2 = 20 chars, limit to 15 chars
        // Should remove "Hello" to get under limit
        let removed = conv.truncate_to_limit(15);
        assert_eq!(removed, 1);
        assert_eq!(conv.message_count(), 1);
        assert_eq!(conv.messages()[0].content, "Hi");
    }

    #[test]
    fn test_truncate_to_tokens() {
        let mut conv = Conversation::new();
        conv.add_user_message("a".repeat(20)); // 20 chars = ~5 tokens
        conv.add_assistant_message("b".repeat(20)); // 20 chars = ~5 tokens
        conv.add_user_message("c".repeat(20)); // 20 chars = ~5 tokens

        // Total: 60 chars = ~15 tokens
        // Limit to 6 tokens = ~24 chars, should remove first 2 messages (40 chars) to get to 20 chars
        let removed = conv.truncate_to_tokens(6);
        assert_eq!(removed, 2);
        assert_eq!(conv.message_count(), 1);
        assert_eq!(conv.messages()[0].content, "c".repeat(20));
    }
}
