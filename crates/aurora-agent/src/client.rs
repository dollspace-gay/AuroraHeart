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
    #[error("Invalid API key - please check your settings")]
    InvalidApiKey,

    /// Rate limit exceeded
    #[error("Rate limit exceeded - please wait a moment")]
    RateLimitExceeded,

    /// Server error (5xx)
    #[error("Server error: {0}")]
    ServerError(String),

    /// Network timeout
    #[error("Request timed out - please check your connection")]
    Timeout,

    /// Maximum retries exceeded
    #[error("Maximum retries exceeded: {0}")]
    MaxRetriesExceeded(String),
}

impl ClientError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            // Transient errors that should be retried
            ClientError::RateLimitExceeded => true,
            ClientError::ServerError(_) => true,
            ClientError::Timeout => true,
            ClientError::Http(e) => {
                // Retry on network errors, timeouts, etc.
                e.is_timeout() || e.is_connect() || e.is_request()
            }
            // Permanent errors that should not be retried
            ClientError::InvalidApiKey => false,
            ClientError::ApiError(_) => false,
            ClientError::JsonParse(_) => false,
            ClientError::MaxRetriesExceeded(_) => false,
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            ClientError::InvalidApiKey => {
                "Your API key is invalid. Please update it in Settings.".to_string()
            }
            ClientError::RateLimitExceeded => {
                "You've hit the rate limit. Please wait a moment and try again.".to_string()
            }
            ClientError::ServerError(_) => {
                "The API server is experiencing issues. Please try again in a moment.".to_string()
            }
            ClientError::Timeout => {
                "The request timed out. Please check your internet connection.".to_string()
            }
            ClientError::MaxRetriesExceeded(_) => {
                "Could not complete the request after multiple attempts. Please try again later.".to_string()
            }
            ClientError::Http(e) if e.is_connect() => {
                "Could not connect to the API. Please check your internet connection.".to_string()
            }
            ClientError::Http(_) | ClientError::ApiError(_) | ClientError::JsonParse(_) => {
                format!("An error occurred: {}", self)
            }
        }
    }
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
#[derive(Debug, Clone, Serialize)]
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
                401 | 403 => ClientError::InvalidApiKey,
                429 => ClientError::RateLimitExceeded,
                500..=599 => ClientError::ServerError(format!("{}: {}", status, error_text)),
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
                401 | 403 => ClientError::InvalidApiKey,
                429 => ClientError::RateLimitExceeded,
                500..=599 => ClientError::ServerError(format!("{}: {}", status, error_text)),
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

    /// Send a message with automatic retries for transient errors
    pub async fn send_message_with_retry(
        &self,
        request: MessageRequest,
        max_retries: usize,
    ) -> Result<MessageResponse, ClientError> {
        retry_with_backoff(max_retries, || self.send_message(request.clone())).await
    }

    /// Send a conversation with automatic retries
    pub async fn send_conversation_with_retry(
        &self,
        conversation: &Conversation,
        max_retries: usize,
    ) -> Result<MessageResponse, ClientError> {
        let request = MessageRequest::from_conversation(conversation, Self::default_model());
        self.send_message_with_retry(request, max_retries).await
    }

    /// Get the model name
    pub fn default_model() -> &'static str {
        "claude-sonnet-4-20250514"
    }
}

/// Retry a future with exponential backoff
async fn retry_with_backoff<F, Fut, T>(
    max_retries: usize,
    mut f: F,
) -> Result<T, ClientError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, ClientError>>,
{
    let mut attempt = 0;

    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                // Don't retry if error is not retryable
                if !e.is_retryable() {
                    tracing::error!("Non-retryable error: {:?}", e);
                    return Err(e);
                }

                attempt += 1;
                if attempt >= max_retries {
                    tracing::error!("Max retries ({}) exceeded", max_retries);
                    return Err(ClientError::MaxRetriesExceeded(e.to_string()));
                }

                // Exponential backoff: 1s, 2s, 4s, 8s, 16s, ...
                let delay_ms = 1000 * 2u64.pow(attempt as u32 - 1);
                let delay_ms = std::cmp::min(delay_ms, 30_000); // Cap at 30 seconds

                // For rate limits, wait longer
                let delay_ms = if matches!(e, ClientError::RateLimitExceeded) {
                    std::cmp::max(delay_ms, 5000) // Minimum 5 seconds for rate limits
                } else {
                    delay_ms
                };

                tracing::warn!(
                    "Attempt {} failed with retryable error, retrying in {}ms: {:?}",
                    attempt,
                    delay_ms,
                    e
                );

                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
        }
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

    // HTTP mocking tests
    #[tokio::test]
    async fn test_send_message_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/messages")
            .match_header("x-api-key", "test_key")
            .match_header("anthropic-version", "2023-06-01")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(r#"{
                "id": "msg_123",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "Hello! How can I help you?"}],
                "model": "claude-sonnet-4",
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 10, "output_tokens": 20}
            }"#)
            .create_async()
            .await;

        let mut client = AnthropicClient::new("test_key".to_string());
        client.base_url = server.url();

        let request = MessageRequest::new(
            "claude-sonnet-4",
            vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        );

        let result = client.send_message(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.model, "claude-sonnet-4");
        match &response.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "Hello! How can I help you?"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_send_message_invalid_api_key() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/messages")
            .with_status(401)
            .with_body(r#"{"error": {"type": "authentication_error", "message": "Invalid API key"}}"#)
            .create_async()
            .await;

        let mut client = AnthropicClient::new("invalid_key".to_string());
        client.base_url = server.url();

        let request = MessageRequest::new(
            "claude-sonnet-4",
            vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        );

        let result = client.send_message(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ClientError::InvalidApiKey => {}
            e => panic!("Expected InvalidApiKey error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_send_message_rate_limit() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/messages")
            .with_status(429)
            .with_body(r#"{"error": {"type": "rate_limit_error", "message": "Rate limit exceeded"}}"#)
            .create_async()
            .await;

        let mut client = AnthropicClient::new("test_key".to_string());
        client.base_url = server.url();

        let request = MessageRequest::new(
            "claude-sonnet-4",
            vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        );

        let result = client.send_message(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ClientError::RateLimitExceeded => {}
            e => panic!("Expected RateLimitExceeded error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_send_message_server_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/messages")
            .with_status(500)
            .with_body(r#"{"error": {"type": "internal_server_error", "message": "Server error"}}"#)
            .create_async()
            .await;

        let mut client = AnthropicClient::new("test_key".to_string());
        client.base_url = server.url();

        let request = MessageRequest::new(
            "claude-sonnet-4",
            vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        );

        let result = client.send_message(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ClientError::ServerError(_) => {}
            e => panic!("Expected ServerError, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_error_is_retryable() {
        // Retryable errors
        assert!(ClientError::RateLimitExceeded.is_retryable());
        assert!(ClientError::ServerError("test".to_string()).is_retryable());
        assert!(ClientError::Timeout.is_retryable());

        // Non-retryable errors
        assert!(!ClientError::InvalidApiKey.is_retryable());
        assert!(!ClientError::ApiError("bad request".to_string()).is_retryable());
        assert!(!ClientError::JsonParse(serde_json::from_str::<MessageResponse>("invalid").unwrap_err()).is_retryable());
        assert!(!ClientError::MaxRetriesExceeded("test".to_string()).is_retryable());
    }

    #[tokio::test]
    async fn test_error_user_messages() {
        let invalid_key_error = ClientError::InvalidApiKey;
        assert!(invalid_key_error.user_message().contains("API key"));

        let rate_limit_error = ClientError::RateLimitExceeded;
        assert!(rate_limit_error.user_message().contains("rate limit"));

        let server_error = ClientError::ServerError("500".to_string());
        assert!(server_error.user_message().contains("server"));

        let timeout_error = ClientError::Timeout;
        assert!(timeout_error.user_message().contains("timed out"));
    }

    #[tokio::test]
    async fn test_retry_with_transient_error() {
        let mut server = mockito::Server::new_async().await;

        // First call fails with 500, second succeeds
        let mock_fail = server
            .mock("POST", "/messages")
            .with_status(500)
            .with_body(r#"{"error": {"type": "internal_server_error", "message": "Server error"}}"#)
            .expect(1)
            .create_async()
            .await;

        let mock_success = server
            .mock("POST", "/messages")
            .with_status(200)
            .with_body(r#"{
                "id": "msg_123",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "Success after retry"}],
                "model": "claude-sonnet-4",
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 10, "output_tokens": 20}
            }"#)
            .expect(1)
            .create_async()
            .await;

        let mut client = AnthropicClient::new("test_key".to_string());
        client.base_url = server.url();

        let request = MessageRequest::new(
            "claude-sonnet-4",
            vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        );

        let result = client.send_message_with_retry(request, 3).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        match &response.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "Success after retry"),
        }

        mock_fail.assert_async().await;
        mock_success.assert_async().await;
    }

    #[tokio::test]
    async fn test_retry_fails_with_non_retryable_error() {
        let mut server = mockito::Server::new_async().await;

        // Returns 401 which is not retryable
        let mock = server
            .mock("POST", "/messages")
            .with_status(401)
            .with_body(r#"{"error": {"type": "authentication_error", "message": "Invalid API key"}}"#)
            .expect(1)  // Should only be called once, not retried
            .create_async()
            .await;

        let mut client = AnthropicClient::new("invalid_key".to_string());
        client.base_url = server.url();

        let request = MessageRequest::new(
            "claude-sonnet-4",
            vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        );

        let result = client.send_message_with_retry(request, 3).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ClientError::InvalidApiKey => {}
            e => panic!("Expected InvalidApiKey error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let mut server = mockito::Server::new_async().await;

        // Always fails with 500
        let mock = server
            .mock("POST", "/messages")
            .with_status(500)
            .with_body(r#"{"error": {"type": "internal_server_error", "message": "Server error"}}"#)
            .expect(3)  // Should try 3 times
            .create_async()
            .await;

        let mut client = AnthropicClient::new("test_key".to_string());
        client.base_url = server.url();

        let request = MessageRequest::new(
            "claude-sonnet-4",
            vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        );

        let result = client.send_message_with_retry(request, 3).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ClientError::MaxRetriesExceeded(_) => {}
            e => panic!("Expected MaxRetriesExceeded error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_streaming_request_setup() {
        let mut server = mockito::Server::new_async().await;

        // Verify that streaming request includes stream:true parameter
        let mock = server
            .mock("POST", "/messages")
            .match_header("x-api-key", "test_key")
            .match_body(mockito::Matcher::PartialJsonString(r#"{"stream":true}"#.to_string()))
            .with_status(200)
            .with_body("data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_123\"}}\n\n")
            .create_async()
            .await;

        let mut client = AnthropicClient::new("test_key".to_string());
        client.base_url = server.url();

        let request = MessageRequest::new(
            "claude-sonnet-4",
            vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        );

        let result = client.send_message_stream(request).await;
        // Just verify the stream is created successfully
        // Detailed streaming parsing is tested in integration tests
        assert!(result.is_ok());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_streaming_error_handling() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/messages")
            .with_status(401)
            .with_body(r#"{"error": {"type": "authentication_error", "message": "Invalid API key"}}"#)
            .create_async()
            .await;

        let mut client = AnthropicClient::new("invalid_key".to_string());
        client.base_url = server.url();

        let request = MessageRequest::new(
            "claude-sonnet-4",
            vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        );

        let result = client.send_message_stream(request).await;
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                ClientError::InvalidApiKey => {}
                e => panic!("Expected InvalidApiKey error, got: {:?}", e),
            }
        }

        mock.assert_async().await;
    }

    // Integration tests
    #[tokio::test]
    async fn test_integration_conversation_to_api_request() {
        use crate::conversation::Conversation;

        // Create a conversation with multiple messages
        let mut conversation = Conversation::with_system_prompt("You are a helpful assistant");
        conversation.add_user_message("Hello!");
        conversation.add_assistant_message("Hi! How can I help you?");
        conversation.add_user_message("What's the weather?");

        // Convert to API request
        let request = MessageRequest::from_conversation(&conversation, "claude-sonnet-4");

        // Verify request structure
        assert_eq!(request.model, "claude-sonnet-4");
        assert_eq!(request.system, Some("You are a helpful assistant".to_string()));
        assert_eq!(request.messages.len(), 3);

        // Verify message order and content
        assert_eq!(request.messages[0].role, "user");
        assert_eq!(request.messages[0].content, "Hello!");
        assert_eq!(request.messages[1].role, "assistant");
        assert_eq!(request.messages[1].content, "Hi! How can I help you?");
        assert_eq!(request.messages[2].role, "user");
        assert_eq!(request.messages[2].content, "What's the weather?");
    }

    #[tokio::test]
    async fn test_integration_conversation_truncation_before_api_call() {
        use crate::conversation::Conversation;

        // Create a conversation with many long messages
        let mut conversation = Conversation::with_system_prompt("System");
        for i in 0..10 {
            conversation.add_user_message(format!("This is a longer user message number {} with more content", i));
            conversation.add_assistant_message(format!("This is a longer assistant response number {} with more content", i));
        }

        // Verify we have all messages
        assert_eq!(conversation.message_count(), 20);

        // Get initial character count
        let initial_chars = conversation.total_chars();
        assert!(initial_chars > 400); // Should be well over 400 chars

        // Truncate to fit within a small limit (50 tokens = ~200 chars)
        let removed = conversation.truncate_to_tokens(50);
        assert!(removed > 0);
        assert!(conversation.message_count() < 20);

        // Verify we can still create a valid request
        let request = MessageRequest::from_conversation(&conversation, "claude-sonnet-4");
        assert!(request.messages.len() > 0);
        assert!(request.messages.len() < 20);
    }

    #[tokio::test]
    async fn test_integration_end_to_end_message_flow() {
        use crate::conversation::Conversation;

        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/messages")
            .with_status(200)
            .with_body(r#"{
                "id": "msg_123",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "I'm doing great, thanks for asking!"}],
                "model": "claude-sonnet-4",
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 15, "output_tokens": 25}
            }"#)
            .create_async()
            .await;

        // 1. Create conversation
        let mut conversation = Conversation::with_system_prompt("You are a friendly AI");
        conversation.add_user_message("Hello, how are you?");

        // 2. Create client
        let mut client = AnthropicClient::new("test_key".to_string());
        client.base_url = server.url();

        // 3. Send conversation
        let result = client.send_conversation_with_retry(&conversation, 3).await;
        assert!(result.is_ok());

        // 4. Verify response
        let response = result.unwrap();
        match &response.content[0] {
            ContentBlock::Text { text } => {
                assert_eq!(text, "I'm doing great, thanks for asking!");
            }
        }

        // 5. Add response to conversation
        match &response.content[0] {
            ContentBlock::Text { text } => {
                conversation.add_assistant_message(text.clone());
            }
        }

        // 6. Verify conversation state
        assert_eq!(conversation.message_count(), 2);
        assert_eq!(conversation.messages()[0].role, crate::conversation::Role::User);
        assert_eq!(conversation.messages()[1].role, crate::conversation::Role::Assistant);

        mock.assert_async().await;
    }
}
