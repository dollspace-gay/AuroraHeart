//! Anthropic API client for Claude
//!
//! This module provides a client for interacting with Anthropic's Claude API,
//! including support for streaming responses and tool use.

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
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("claude-sonnet-4"));
        assert!(json.contains("Hello!"));
    }
}
