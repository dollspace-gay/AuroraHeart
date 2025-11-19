//! Anthropic API client for Claude
//!
//! This module provides a client for interacting with Anthropic's Claude API,
//! including support for streaming responses and tool use.

use crate::conversation::{Conversation, Message, Role};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during API client operations
#[derive(Error, Debug)]
pub enum ClientError {
    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON parsing error
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// API error response
    #[error("API error: {0}")]
    ApiError(String),

    /// Invalid API key
    #[error("Invalid API key")]
    InvalidApiKey,

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

/// Anthropic API client
pub struct AnthropicClient {
    /// API key for authentication
    api_key: String,
    /// HTTP client
    client: reqwest::Client,
    /// API base URL
    base_url: String,
}

/// Request to send to Claude
#[derive(Debug, Serialize)]
pub struct MessageRequest {
    /// Model to use (e.g., "claude-sonnet-4")
    pub model: String,
    /// Maximum tokens to generate
    pub max_tokens: usize,
    /// Messages in the conversation
    pub messages: Vec<ApiMessage>,
    /// System prompt (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Temperature for sampling (0.0 to 1.0, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<usize>,
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    /// Role of the message sender
    pub role: String,
    /// Content of the message
    pub content: String,
}

/// Response from Claude
#[derive(Debug, Deserialize)]
pub struct MessageResponse {
    /// Generated content
    pub content: Vec<ContentBlock>,
    /// Model used
    pub model: String,
    /// Stop reason
    pub stop_reason: Option<String>,
}

/// Content block in a response
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
}

/// Streaming event from Claude API
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStart },

    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ContentBlockStart
    },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: usize,
        delta: Delta
    },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },

    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDeltaInfo },

    #[serde(rename = "message_stop")]
    MessageStop,

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "error")]
    Error { error: ApiError },
}

/// Message start event
#[derive(Debug, Deserialize)]
pub struct MessageStart {
    pub id: String,
    pub model: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub role: String,
}

/// Content block start
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlockStart {
    #[serde(rename = "text")]
    Text { text: String },
}

/// Delta in streaming response
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Delta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
}

/// Message delta information
#[derive(Debug, Deserialize)]
pub struct MessageDeltaInfo {
    pub stop_reason: Option<String>,
}

/// API error
#[derive(Debug, Deserialize)]
pub struct ApiError {
    #[serde(rename = "type")]
    pub type_: String,
    pub message: String,
}

impl MessageRequest {
    /// Create a new message request with default settings
    pub fn new(model: impl Into<String>, messages: Vec<ApiMessage>) -> Self {
        Self {
            model: model.into(),
            max_tokens: 4096,
            messages,
            system: None,
            temperature: None,
            top_p: None,
            top_k: None,
        }
    }

    /// Create a request from a Conversation
    pub fn from_conversation(conversation: &Conversation, model: impl Into<String>) -> Self {
        let messages = conversation
            .messages()
            .iter()
            .map(|msg| ApiMessage::from_message(msg))
            .collect();

        Self {
            model: model.into(),
            max_tokens: 4096,
            messages,
            system: conversation.system_prompt.clone(),
            temperature: None,
            top_p: None,
            top_k: None,
        }
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the top-p value
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set the top-k value
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }
}

impl ApiMessage {
    /// Create an ApiMessage from a conversation Message
    pub fn from_message(message: &Message) -> Self {
        Self {
            role: match message.role {
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            content: message.content.clone(),
        }
    }
}

impl AnthropicClient {
    /// Create a new Anthropic API client
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }

    /// Send a message to Claude and get a response
    pub async fn send_message(
        &self,
        request: MessageRequest,
    ) -> Result<MessageResponse, ClientError> {
        let url = format!("{}/messages", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;

            return Err(match status.as_u16() {
                401 => ClientError::InvalidApiKey,
                429 => ClientError::RateLimitExceeded,
                _ => ClientError::ApiError(format!("{}: {}", status, error_text)),
            });
        }

        let message_response: MessageResponse = response.json().await?;
        Ok(message_response)
    }

    /// Send a conversation to Claude and get a response
    pub async fn send_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<MessageResponse, ClientError> {
        let request = MessageRequest::from_conversation(conversation, Self::default_model());
        self.send_message(request).await
    }

    /// Send a message to Claude and get a streaming response
    pub async fn send_message_stream(
        &self,
        request: MessageRequest,
    ) -> Result<impl futures::Stream<Item = Result<StreamEvent, ClientError>>, ClientError> {
        use bytes::Buf;
        use futures::stream::StreamExt;

        // Add stream parameter to request
        let url = format!("{}/messages", self.base_url);

        // Create request body with stream: true
        let mut request_json = serde_json::to_value(&request)?;
        if let Some(obj) = request_json.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::Value::Bool(true));
        }

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_json)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;

            return Err(match status.as_u16() {
                401 => ClientError::InvalidApiKey,
                429 => ClientError::RateLimitExceeded,
                _ => ClientError::ApiError(format!("{}: {}", status, error_text)),
            });
        }

        // Convert response to stream of SSE events
        let stream = response.bytes_stream().map(move |chunk_result| {
            let chunk = chunk_result.map_err(ClientError::Http)?;

            // Parse SSE format: "data: {...}\n\n"
            let data = String::from_utf8_lossy(chunk.chunk());

            for line in data.lines() {
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if json_str.trim() == "[DONE]" {
                        // End of stream marker (some APIs use this)
                        continue;
                    }

                    match serde_json::from_str::<StreamEvent>(json_str) {
                        Ok(event) => return Ok(event),
                        Err(e) => {
                            tracing::warn!("Failed to parse stream event: {}", e);
                            continue;
                        }
                    }
                }
            }

            // If we couldn't parse anything, return a Ping event to keep stream alive
            Ok(StreamEvent::Ping)
        });

        Ok(stream)
    }

    /// Send a conversation to Claude and get a streaming response
    pub async fn send_conversation_stream(
        &self,
        conversation: &Conversation,
    ) -> Result<impl futures::Stream<Item = Result<StreamEvent, ClientError>>, ClientError> {
        let request = MessageRequest::from_conversation(conversation, Self::default_model());
        self.send_message_stream(request).await
    }

    /// Get the model name
    pub fn default_model() -> &'static str {
        "claude-sonnet-4-20250514"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = AnthropicClient::new("test_api_key".to_string());
        assert_eq!(client.api_key, "test_api_key");
        assert_eq!(client.base_url, "https://api.anthropic.com/v1");
    }

    #[test]
    fn test_message_request_serialization() {
        let request = MessageRequest {
            model: "claude-sonnet-4".to_string(),
            max_tokens: 1024,
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello!".to_string(),
            }],
            system: Some("You are a helpful assistant.".to_string()),
            temperature: Some(0.7),
            top_p: None,
            top_k: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("claude-sonnet-4"));
        assert!(json.contains("Hello!"));
        assert!(json.contains("0.7"));
    }

    #[test]
    fn test_message_request_builder() {
        let messages = vec![ApiMessage {
            role: "user".to_string(),
            content: "Test".to_string(),
        }];

        let request = MessageRequest::new("claude-sonnet-4", messages)
            .with_temperature(0.5)
            .with_top_k(40);

        assert_eq!(request.model, "claude-sonnet-4");
        assert_eq!(request.temperature, Some(0.5));
        assert_eq!(request.top_k, Some(40));
    }

    #[test]
    fn test_from_conversation() {
        use crate::conversation::Conversation;

        let mut conv = Conversation::with_system_prompt("You are helpful");
        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi there!");

        let request = MessageRequest::from_conversation(&conv, "claude-sonnet-4");

        assert_eq!(request.model, "claude-sonnet-4");
        assert_eq!(request.system, Some("You are helpful".to_string()));
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].role, "user");
        assert_eq!(request.messages[0].content, "Hello");
        assert_eq!(request.messages[1].role, "assistant");
        assert_eq!(request.messages[1].content, "Hi there!");
    }
}
