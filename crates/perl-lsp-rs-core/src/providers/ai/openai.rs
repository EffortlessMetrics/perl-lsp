//! OpenAI-compatible completion provider.

use super::prompt::build_fim_prompt;
use super::rate_limiter::RateLimiter;
use super::sse::SseParser;
use crate::config::{
    DEFAULT_AI_API_KEY_HEADER, DEFAULT_AI_API_KEY_PREFIX, is_safe_http_header_value_part,
    normalize_ai_api_key_header, normalize_ai_api_key_prefix,
};
use crate::providers::inline_completion::{
    BackendError, BackendRequest, InlineCompletionBackend, StreamChunk, StreamControl,
};
use std::io::BufReader;
use std::sync::Arc;

/// Configuration for the OpenAI-compatible provider.
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    /// The API endpoint URL (e.g. `https://api.openai.com/v1/chat/completions`).
    pub endpoint: String,
    /// The model name to use (e.g. `gpt-4o`).
    pub model: String,
    /// API key for authentication.
    pub api_key: String,
    /// HTTP header that carries the API key.
    pub api_key_header: String,
    /// Optional authentication scheme prepended before the API key.
    pub api_key_prefix: Option<String>,
    /// Global timeout in milliseconds.
    pub timeout_ms: u64,
}

/// An OpenAI-compatible completion provider using ureq for HTTP.
pub struct OpenAiProvider {
    config: OpenAiConfig,
    limiter: Arc<RateLimiter>,
}

impl OpenAiConfig {
    /// Build a default bearer-token configuration for OpenAI-compatible web APIs.
    pub fn new(endpoint: String, model: String, api_key: String, timeout_ms: u64) -> Self {
        Self {
            endpoint,
            model,
            api_key,
            api_key_header: DEFAULT_AI_API_KEY_HEADER.to_string(),
            api_key_prefix: Some(DEFAULT_AI_API_KEY_PREFIX.to_string()),
            timeout_ms,
        }
    }
}

impl OpenAiProvider {
    /// Create a new provider with the given config and rate limiter.
    pub fn new(config: OpenAiConfig, limiter: Arc<RateLimiter>) -> Self {
        Self { config, limiter }
    }

    fn auth_header_name(&self) -> &str {
        if normalize_ai_api_key_header(&self.config.api_key_header).is_some() {
            self.config.api_key_header.as_str()
        } else {
            DEFAULT_AI_API_KEY_HEADER
        }
    }

    fn auth_header_value(&self) -> Result<String, BackendError> {
        if !is_safe_http_header_value_part(&self.config.api_key) {
            return Err(BackendError::Auth(
                "AI API key contains unsupported HTTP header characters".to_string(),
            ));
        }

        let prefix =
            self.config.api_key_prefix.as_deref().and_then(normalize_ai_api_key_prefix).flatten();

        Ok(match prefix.as_deref() {
            Some(prefix) => format!("{prefix} {}", self.config.api_key),
            None => self.config.api_key.clone(),
        })
    }

    fn build_request_body(&self, req: &BackendRequest) -> serde_json::Value {
        let (system, user) = build_fim_prompt(&req.context);

        if self.uses_responses_api() {
            return serde_json::json!({
                "model": self.config.model,
                "max_output_tokens": req.max_output_tokens,
                "stream": true,
                "instructions": system,
                "input": user,
            });
        }

        serde_json::json!({
            "model": self.config.model,
            "max_tokens": req.max_output_tokens,
            "stream": true,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ]
        })
    }

    fn uses_responses_api(&self) -> bool {
        self.config.endpoint.contains("/responses")
    }

    fn extract_content_delta(data: &str) -> Option<String> {
        let parsed: serde_json::Value = serde_json::from_str(data).ok()?;

        if let Some(delta) = parsed
            .get("choices")
            .and_then(serde_json::Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("content"))
            .and_then(serde_json::Value::as_str)
        {
            return Some(delta.to_string());
        }

        let event_type = parsed.get("type").and_then(serde_json::Value::as_str)?;
        if event_type != "response.output_text.delta" {
            return None;
        }

        parsed.get("delta").and_then(serde_json::Value::as_str).map(str::to_string)
    }

    fn extract_finish_reason(data: &str) -> Option<String> {
        let parsed: serde_json::Value = serde_json::from_str(data).ok()?;

        if let Some(reason) = parsed
            .get("choices")
            .and_then(serde_json::Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("finish_reason"))
            .and_then(serde_json::Value::as_str)
        {
            return Some(reason.to_string());
        }

        match parsed.get("type").and_then(serde_json::Value::as_str) {
            Some("response.completed") => Some("stop".to_string()),
            Some("response.failed") | Some("response.incomplete") => Some("error".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{OpenAiConfig, OpenAiProvider};
    use crate::providers::ai::rate_limiter::RateLimiter;
    use std::sync::Arc;

    fn provider_with_endpoint(endpoint: &str) -> OpenAiProvider {
        OpenAiProvider::new(
            OpenAiConfig::new(
                endpoint.to_string(),
                "gpt-4o-mini".to_string(),
                "test-key".to_string(),
                1000,
            ),
            Arc::new(RateLimiter::new(1.0, 1)),
        )
    }

    #[test]
    fn detects_responses_api_endpoint() {
        let provider = provider_with_endpoint("https://api.openai.com/v1/responses");
        assert!(provider.uses_responses_api());

        let provider = provider_with_endpoint("https://api.openai.com/v1/chat/completions");
        assert!(!provider.uses_responses_api());
    }

    #[test]
    fn extracts_chat_completions_delta() {
        let data = r#"{"choices":[{"delta":{"content":"my $"},"finish_reason":null}]}"#;
        assert_eq!(OpenAiProvider::extract_content_delta(data), Some("my $".to_string()));
    }

    #[test]
    fn extracts_responses_delta() {
        let data = r#"{"type":"response.output_text.delta","delta":"my $"}"#;
        assert_eq!(OpenAiProvider::extract_content_delta(data), Some("my $".to_string()));
    }

    #[test]
    fn detects_responses_completion_event() {
        let data = r#"{"type":"response.completed"}"#;
        assert_eq!(OpenAiProvider::extract_finish_reason(data), Some("stop".to_string()));
    }

    #[test]
    fn default_config_uses_bearer_authorization_header() -> Result<(), Box<dyn std::error::Error>> {
        let provider = provider_with_endpoint("https://api.openai.com/v1/chat/completions");
        assert_eq!(provider.config.api_key_header, "Authorization");
        assert_eq!(provider.auth_header_name(), "Authorization");
        assert_eq!(provider.auth_header_value()?, "Bearer test-key");
        Ok(())
    }

    #[test]
    fn custom_web_connector_auth_header_can_send_raw_key() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut config = OpenAiConfig::new(
            "https://example.test/v1/chat/completions".to_string(),
            "custom-code-model".to_string(),
            "connector-key".to_string(),
            1000,
        );
        config.api_key_header = "x-api-key".to_string();
        config.api_key_prefix = None;
        let provider = OpenAiProvider::new(config, Arc::new(RateLimiter::new(1.0, 1)));

        assert_eq!(provider.config.api_key_header, "x-api-key");
        assert_eq!(provider.auth_header_name(), "x-api-key");
        assert_eq!(provider.auth_header_value()?, "connector-key");
        Ok(())
    }

    #[test]
    fn malformed_auth_header_name_falls_back_without_exposing_key()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut config = OpenAiConfig::new(
            "https://example.test/v1/chat/completions".to_string(),
            "custom-code-model".to_string(),
            "connector-key".to_string(),
            1000,
        );
        config.api_key_header = "x-api-key\r\nX-Injected".to_string();
        let provider = OpenAiProvider::new(config, Arc::new(RateLimiter::new(1.0, 1)));

        assert_eq!(provider.auth_header_name(), "Authorization");
        assert_eq!(provider.auth_header_value()?, "Bearer connector-key");
        Ok(())
    }

    #[test]
    fn malformed_auth_prefix_and_key_do_not_enter_header_text()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut config = OpenAiConfig::new(
            "https://example.test/v1/chat/completions".to_string(),
            "custom-code-model".to_string(),
            "connector-key".to_string(),
            1000,
        );
        config.api_key_prefix = Some("Token\r\nX-Injected".to_string());
        let provider = OpenAiProvider::new(config, Arc::new(RateLimiter::new(1.0, 1)));
        assert_eq!(provider.auth_header_value()?, "connector-key");

        let mut bad_key_config = OpenAiConfig::new(
            "https://example.test/v1/chat/completions".to_string(),
            "custom-code-model".to_string(),
            "connector-key\r\nX-Injected".to_string(),
            1000,
        );
        bad_key_config.api_key_prefix = None;
        let provider = OpenAiProvider::new(bad_key_config, Arc::new(RateLimiter::new(1.0, 1)));
        let Err(err) = provider.auth_header_value() else {
            return Err("invalid key must be rejected".into());
        };
        let message = err.to_string();
        assert!(message.contains("unsupported HTTP header characters"));
        assert!(!message.contains("connector-key"));
        Ok(())
    }
}

impl InlineCompletionBackend for OpenAiProvider {
    fn stream(
        &self,
        req: &BackendRequest,
        sink: &mut dyn FnMut(StreamChunk) -> StreamControl,
    ) -> Result<(), BackendError> {
        if !self.limiter.try_acquire() {
            return Err(BackendError::RateLimited);
        }

        let body = self.build_request_body(req);
        let timeout = std::time::Duration::from_millis(req.timeout_ms);

        let config = ureq::Agent::config_builder().timeout_global(Some(timeout)).build();
        let agent = ureq::Agent::new_with_config(config);

        let response = agent
            .post(&self.config.endpoint)
            .header(self.auth_header_name(), self.auth_header_value()?.as_str())
            .header("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("timed out") || msg.contains("timeout") {
                    BackendError::Timeout
                } else if msg.contains("401") || msg.contains("403") {
                    BackendError::Auth(msg)
                } else {
                    BackendError::Transport(msg)
                }
            })?;

        let reader = BufReader::new(response.into_body().into_reader());
        let mut parser = SseParser::new(reader);
        let mut cumulative = String::new();

        loop {
            match parser.next_event() {
                Ok(Some(event)) => {
                    if let Some(delta) = Self::extract_content_delta(&event.data) {
                        cumulative.push_str(&delta);

                        let is_final = Self::extract_finish_reason(&event.data)
                            .is_some_and(|r| r == "stop" || r == "length");

                        let control = sink(StreamChunk { text: cumulative.clone(), is_final });

                        if control == StreamControl::Stop || is_final {
                            break;
                        }
                    }
                }
                Ok(None) => {
                    // Stream ended -- emit final chunk if we have content
                    if !cumulative.is_empty() {
                        sink(StreamChunk { text: cumulative, is_final: true });
                    }
                    break;
                }
                Err(e) => {
                    return Err(BackendError::Transport(e.to_string()));
                }
            }
        }

        Ok(())
    }
}
