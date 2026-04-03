//! AI completion providers for perl-lsp.
//!
//! This crate provides pluggable AI backends for inline code completion.
//! The primary provider is OpenAI-compatible, supporting any endpoint that
//! implements the OpenAI chat completions API with SSE streaming.

pub mod openai;
pub mod prompt;
pub mod rate_limiter;
pub mod sse;

pub use openai::{OpenAiConfig, OpenAiProvider};
pub use rate_limiter::RateLimiter;
