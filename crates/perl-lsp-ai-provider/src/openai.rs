//! OpenAI-compatible completion provider.

use crate::prompt::build_fim_prompt;
use crate::rate_limiter::RateLimiter;
use crate::sse::SseParser;
use perl_lsp_inline_completion::{
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
    /// Global timeout in milliseconds.
    pub timeout_ms: u64,
}

/// An OpenAI-compatible completion provider using ureq for HTTP.
pub struct OpenAiProvider {
    config: OpenAiConfig,
    limiter: Arc<RateLimiter>,
}

impl OpenAiProvider {
    /// Create a new provider with the given config and rate limiter.
    pub fn new(config: OpenAiConfig, limiter: Arc<RateLimiter>) -> Self {
        Self { config, limiter }
    }

    fn build_request_body(&self, req: &BackendRequest) -> serde_json::Value {
        let (system, user) = build_fim_prompt(&req.context);

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

    fn extract_content_delta(data: &str) -> Option<String> {
        let parsed: serde_json::Value = serde_json::from_str(data).ok()?;
        let choices = parsed.get("choices")?.as_array()?;
        let delta = choices.first()?.get("delta")?.get("content")?.as_str()?;
        Some(delta.to_string())
    }

    fn extract_finish_reason(data: &str) -> Option<String> {
        let parsed: serde_json::Value = serde_json::from_str(data).ok()?;
        let choices = parsed.get("choices")?.as_array()?;
        choices
            .first()?
            .get("finish_reason")?
            .as_str()
            .map(|s| s.to_string())
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

        let config = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .build();
        let agent = ureq::Agent::new_with_config(config);

        let response = agent
            .post(&self.config.endpoint)
            .header("Authorization", &format!("Bearer {}", self.config.api_key))
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

                        let control = sink(StreamChunk {
                            text: cumulative.clone(),
                            is_final,
                        });

                        if control == StreamControl::Stop || is_final {
                            break;
                        }
                    }
                }
                Ok(None) => {
                    // Stream ended -- emit final chunk if we have content
                    if !cumulative.is_empty() {
                        sink(StreamChunk {
                            text: cumulative,
                            is_final: true,
                        });
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
