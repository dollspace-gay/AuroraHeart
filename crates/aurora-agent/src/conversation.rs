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

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the sender
    pub role: Role,
    /// Content of the message
    pub content: String,
}

impl Message {
    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
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
            + self.messages.iter().map(|m| m.content.len()).sum::<usize>()
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
}
