//! Claude API client for AI-powered features.

use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone, Error)]
pub enum ClaudeApiError {
    #[error("network error: {0}")]
    Transport(String),
    #[error("timeout")]
    Timeout,
    #[error("http {status}: {body}")]
    Http { status: u16, body: String },
    #[error("rate limited")]
    RateLimited,
    #[error("invalid api key")]
    InvalidApiKey,
    #[error("json error: {0}")]
    Serde(String),
    #[error("missing api key: ANTHROPIC_API_KEY environment variable not set")]
    MissingApiKey,
}

impl ClaudeApiError {
    /// Returns true if the error is transient and should be retried.
    pub fn should_retry(&self) -> bool {
        match self {
            Self::Transport(_) | Self::Timeout | Self::RateLimited => true,
            Self::Http { status, .. } => (500..=599).contains(status),
            _ => false,
        }
    }
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Request body for Claude API
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

/// Content block in response
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
}

/// Response from Claude API
#[derive(Debug, Deserialize)]
pub struct ClaudeResponse {
    pub id: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub usage: Usage,
}

impl ClaudeResponse {
    /// Extract the text content from the response
    pub fn text(&self) -> Option<&str> {
        self.content.iter().find_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
        })
    }
}

/// Token usage information
#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Claude API client
#[derive(Debug, Clone)]
pub struct ClaudeApiClient {
    http: Client,
    api_key: String,
    model: String,
}

impl ClaudeApiClient {
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

    /// Create a new client using the ANTHROPIC_API_KEY environment variable
    pub fn from_env() -> Result<Self, ClaudeApiError> {
        let api_key =
            std::env::var("ANTHROPIC_API_KEY").map_err(|_| ClaudeApiError::MissingApiKey)?;
        Self::new(api_key, None)
    }

    /// Create a new client with the given API key
    pub fn new(api_key: String, model: Option<String>) -> Result<Self, ClaudeApiError> {
        let http = Client::builder()
            .timeout(Self::REQUEST_TIMEOUT)
            .user_agent(concat!("vibe-kanban-raid/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| ClaudeApiError::Transport(e.to_string()))?;

        Ok(Self {
            http,
            api_key,
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        })
    }

    /// Send a completion request to Claude
    pub async fn complete(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
        max_tokens: u32,
    ) -> Result<ClaudeResponse, ClaudeApiError> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens,
            messages,
            system,
        };

        (|| async { self.send_request(&request).await })
            .retry(
                &ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(30))
                    .with_max_times(3)
                    .with_jitter(),
            )
            .when(|e: &ClaudeApiError| e.should_retry())
            .notify(|e, dur| {
                warn!(
                    "Claude API call failed, retrying after {:.2}s: {}",
                    dur.as_secs_f64(),
                    e
                )
            })
            .await
    }

    async fn send_request(&self, request: &ClaudeRequest) -> Result<ClaudeResponse, ClaudeApiError> {
        let res = self
            .http
            .post(CLAUDE_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(map_reqwest_error)?;

        match res.status() {
            s if s.is_success() => {
                res.json::<ClaudeResponse>()
                    .await
                    .map_err(|e| ClaudeApiError::Serde(e.to_string()))
            }
            StatusCode::UNAUTHORIZED => Err(ClaudeApiError::InvalidApiKey),
            StatusCode::TOO_MANY_REQUESTS => Err(ClaudeApiError::RateLimited),
            s => {
                let status = s.as_u16();
                let body = res.text().await.unwrap_or_default();
                Err(ClaudeApiError::Http { status, body })
            }
        }
    }

    /// Simple helper to send a single user message and get a response
    pub async fn ask(
        &self,
        prompt: &str,
        system: Option<String>,
    ) -> Result<String, ClaudeApiError> {
        let response = self
            .complete(vec![Message::user(prompt)], system, 4096)
            .await?;

        response
            .text()
            .map(|s| s.to_string())
            .ok_or_else(|| ClaudeApiError::Serde("No text content in response".to_string()))
    }

    /// Send a prompt expecting JSON in the response
    pub async fn ask_json<T: for<'de> Deserialize<'de>>(
        &self,
        prompt: &str,
        system: Option<String>,
    ) -> Result<T, ClaudeApiError> {
        self.ask_json_with_max_tokens(prompt, system, 4096).await
    }

    /// Send a prompt expecting JSON in the response with custom max_tokens
    pub async fn ask_json_with_max_tokens<T: for<'de> Deserialize<'de>>(
        &self,
        prompt: &str,
        system: Option<String>,
        max_tokens: u32,
    ) -> Result<T, ClaudeApiError> {
        let response = self
            .complete(vec![Message::user(prompt)], system, max_tokens)
            .await?
            .text()
            .map(|s| s.to_string())
            .ok_or_else(|| ClaudeApiError::Serde("No text content in response".to_string()))?;

        if response.trim().is_empty() {
            tracing::error!("Claude returned an empty response");
            return Err(ClaudeApiError::Serde("Empty response from Claude".to_string()));
        }

        // Try to extract JSON from the response (it might be wrapped in markdown code blocks)
        let json_str = extract_json(&response);

        if json_str.trim().is_empty() {
            tracing::error!(
                response = %response,
                "Failed to extract JSON from response"
            );
            return Err(ClaudeApiError::Serde(format!("Could not extract JSON from response: {}", response)));
        }

        serde_json::from_str(json_str).map_err(|e| {
            tracing::error!(
                json_error = %e,
                response_length = response.len(),
                extracted_json_preview = %json_str.chars().take(500).collect::<String>(),
                "Failed to parse JSON response from Claude"
            );
            ClaudeApiError::Serde(format!("{} (response preview: {})", e, json_str.chars().take(500).collect::<String>()))
        })
    }
}

fn map_reqwest_error(e: reqwest::Error) -> ClaudeApiError {
    if e.is_timeout() {
        ClaudeApiError::Timeout
    } else {
        ClaudeApiError::Transport(e.to_string())
    }
}

/// Extract JSON from a string that might contain markdown code blocks
fn extract_json(text: &str) -> &str {
    let text = text.trim();

    // Try to find JSON in code blocks
    if let Some(start) = text.find("```json") {
        let content_start = start + 7;
        if let Some(end) = text[content_start..].find("```") {
            return text[content_start..content_start + end].trim();
        }
    }

    // Try generic code block
    if let Some(start) = text.find("```") {
        let content_start = start + 3;
        // Skip past any language identifier on the same line
        let content_start = text[content_start..]
            .find('\n')
            .map(|i| content_start + i + 1)
            .unwrap_or(content_start);
        if let Some(end) = text[content_start..].find("```") {
            return text[content_start..content_start + end].trim();
        }
    }

    // Return as-is if no code block found
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_plain() {
        let input = r#"{"key": "value"}"#;
        assert_eq!(extract_json(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_code_block() {
        let input = r#"Here's the JSON:
```json
{"key": "value"}
```"#;
        assert_eq!(extract_json(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_generic_code_block() {
        let input = r#"```
{"key": "value"}
```"#;
        assert_eq!(extract_json(input), r#"{"key": "value"}"#);
    }
}
