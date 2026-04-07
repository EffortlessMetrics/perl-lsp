# perl-lsp-ai-provider

AI completion providers for the [perl-lsp](https://github.com/EffortlessMetrics/perl-lsp) Language Server.

## Overview

This crate plugs an OpenAI-compatible HTTP/SSE streaming backend into the LSP server's inline-completion path. It is the production AI provider behind perl-lsp's `textDocument/inlineCompletion` feature, opt-in via configuration.

## Public API

| Type | Description |
|------|-------------|
| `OpenAiProvider` | Streaming completion provider that talks to any OpenAI-compatible API |
| `OpenAiConfig` | Endpoint, model, API key, and request-shape configuration |
| `RateLimiter` | Token-bucket rate limiter for outbound requests |

## Usage

```rust
use perl_lsp_ai_provider::{OpenAiConfig, OpenAiProvider, RateLimiter};
use std::sync::Arc;

let config = OpenAiConfig {
    endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
    model: "gpt-4o-mini".to_string(),
    api_key: std::env::var("OPENAI_API_KEY").ok(),
    ..Default::default()
};
let limiter = Arc::new(RateLimiter::new(60, 60));  // 60 req/min
let provider = OpenAiProvider::new(config, limiter);
```

The LSP server wires this provider when its AI inline-completion feature is enabled in the client configuration. End users do not call this crate directly; it is consumed by `perl-lsp-rs`.

## License

Dual-licensed under MIT or Apache-2.0 at your option. See `LICENSE-MIT` and `LICENSE-APACHE`.
