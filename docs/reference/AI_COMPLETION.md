# AI Inline Completion Reference

AI inline completions provide context-aware code suggestions as ghost text
while you type. The feature is **opt-in** and **disabled by default**. When
enabled, the server sends the surrounding code context to an
OpenAI-compatible API endpoint and streams back completion candidates. If the
AI backend is unavailable, slow, or disabled, the server falls back to
deterministic pattern-based completions automatically.

## VS Code Settings

These settings appear under the "Perl LSP: AI Completion" section in the VS
Code settings UI.

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `perl-lsp.aiCompletion.enabled` | boolean | `false` | Enable AI-powered inline completions. |
| `perl-lsp.aiCompletion.streaming.enabled` | boolean | `true` | Enable progressive streaming (ghost text updates as tokens arrive). Requires `aiCompletion.enabled`. |

All other AI parameters (endpoint, model, timeout, rate limits) are
configured via the LSP server config or the project config file, not VS Code
settings. The API key is always read from an environment variable -- it is
never stored in settings.

## Server Configuration Fields

The server reads AI completion settings from the `aiCompletion` section of LSP
`initializationOptions` or `didChangeConfiguration`. These are the full fields
accepted:

| JSON Key | Type | Default | Description |
|----------|------|---------|-------------|
| `enabled` | boolean | `false` | Master toggle for AI completions. |
| `provider` | string | `"openai_compat"` | Provider type. Currently only `openai_compat` is supported. |
| `endpoint` | string | `""` (empty) | API endpoint URL (e.g. `https://api.openai.com/v1/chat/completions`). |
| `model` | string | `"gpt-4o-mini"` | Model identifier sent in the request body. |
| `apiKeyEnv` | string | `"OPENAI_API_KEY"` | Name of the environment variable containing the API key. |
| `timeoutMs` | integer | `1800` | Per-request timeout in milliseconds. |
| `maxOutputTokens` | integer | `64` | Maximum tokens the model may generate per request. |
| `rateLimitRps` | float | `1.0` | Maximum requests per second (token-bucket rate). |
| `maxInflight` | integer | `1` | Maximum concurrent in-flight requests (burst size). |
| `fallback` | boolean | `true` | Fall back to deterministic completions on AI failure. |
| `streaming.enabled` | boolean | `true` | Enable streaming mode (progressive ghost text). |
| `streaming.updateDebounceMs` | integer | `60` | Minimum milliseconds between streamed ghost text updates. |

## Project Config File (`.perl-lsp.toml`)

For editor-agnostic, team-wide defaults, add an `[ai_completion]` section to
`.perl-lsp.toml` at the workspace root. These values serve as the base layer;
LSP client settings always override them.

```toml
[ai_completion]
enabled = true
provider = "openai_compat"
endpoint = "https://api.openai.com/v1/chat/completions"
model = "gpt-4o-mini"
api_key_env = "OPENAI_API_KEY"
```

Only the fields you set will override defaults. Omitted fields retain their
built-in default values.

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `OPENAI_API_KEY` | Default API key variable. The server reads the key from whatever variable name is configured in `apiKeyEnv`. |

The API key is resolved at runtime via `std::env::var`. The variable must be
set in the environment where the LSP server process runs. If the variable is
empty or unset, the server logs a warning and disables AI completions (falling
back to deterministic rules if `fallback` is true).

## Supported Providers

The only supported provider type is `openai_compat`, which supports both
OpenAI-compatible wire formats:

- **Responses API** (recommended when available)
- **Chat Completions API** (legacy compatibility)

The server auto-selects the request/stream format from your configured endpoint:

- Endpoints containing `/responses` use Responses payload and event parsing
- Other endpoints use Chat Completions payload and event parsing

- **OpenAI** -- `https://api.openai.com/v1/chat/completions`
- **OpenAI (Responses)** -- `https://api.openai.com/v1/responses`
- **Azure OpenAI** -- `https://<resource>.openai.azure.com/openai/deployments/<deployment>/chat/completions?api-version=<version>`
- **Local servers** -- Any OpenAI-compatible local inference server (e.g. llama.cpp, vLLM, Ollama with OpenAI compatibility layer)
- **Other providers** -- Any service that accepts the same request format and returns SSE `data:` lines with `choices[].delta.content`

For Chat Completions endpoints, the request format uses `"stream": true`,
`messages` (system + user with fill-in-the-middle context), and the configured
`model` and `max_tokens`.

For Responses endpoints, the request format uses `"stream": true`,
`instructions`, `input`, `model`, and `max_output_tokens`.

## Example Configuration (VS Code `settings.json`)

```jsonc
{
  // Enable the feature
  "perl-lsp.aiCompletion.enabled": true,

  // Server-side config passed via initializationOptions
  "perl-lsp.serverConfig": {
    "aiCompletion": {
      "enabled": true,
      "endpoint": "https://api.openai.com/v1/chat/completions",
      "model": "gpt-4o-mini",
      "apiKeyEnv": "OPENAI_API_KEY",
      "timeoutMs": 2000,
      "maxOutputTokens": 64,
      "rateLimitRps": 1.0,
      "maxInflight": 1,
      "streaming": {
        "enabled": true,
        "updateDebounceMs": 60
      }
    }
  }
}
```

Set the API key in your shell profile or VS Code terminal environment:

```bash
export OPENAI_API_KEY="sk-..."
```

## Streaming vs Buffered Behavior

The server supports two completion delivery modes:

**Streaming (default, `streaming.enabled: true`)**:
The server opens an SSE connection to the provider and delivers partial
completions as ghost text via `$/progress` notifications. Each update
contains the cumulative text so far (not a delta). Updates are debounced
by `updateDebounceMs` (default 60ms) to avoid flooding the client. A
session manager tracks active streams with cancel-previous semantics --
when the user types or moves the cursor, the stale stream is cancelled
and a new one starts.

**Buffered (`streaming.enabled: false`)**:
The server waits for the full response before returning the completion
item. Ghost text appears all at once after the request completes (or
times out). This mode is simpler but has higher perceived latency.

In both modes, if the AI backend fails (timeout, auth error, rate limit),
the server falls back to deterministic pattern-based completions when
`fallback` is `true`.

## Troubleshooting

**No AI completions appearing**

1. Verify `aiCompletion.enabled` is `true` in your settings.
2. Check that `endpoint` is set to a valid URL (it defaults to empty).
3. Confirm the API key environment variable is set and non-empty in the
   LSP server's process environment. Check the server stderr log for
   `AI completion enabled but <VAR> is empty or unset`.

**Authentication errors (401/403)**

- The API key is read from the environment variable named in `apiKeyEnv`.
  Verify the variable name matches and the key is valid.
- For Azure OpenAI, ensure the endpoint URL includes the required
  `api-version` query parameter.

**Completions are slow or missing**

- Increase `timeoutMs` if the provider is slow (default is 1800ms).
- Check `rateLimitRps` and `maxInflight`. With defaults of 1.0 rps and 1
  concurrent request, rapid typing will hit the rate limiter. The server
  returns `RateLimited` errors silently and falls back to deterministic
  completions.
- Enable streaming (`streaming.enabled: true`) for lower perceived latency.

**Rate limiting from the provider**

- Reduce `rateLimitRps` to stay within your API plan's limits.
- Reduce `maxOutputTokens` to lower per-request cost and latency.

**Wrong endpoint format**

- The endpoint must be a full API resource URL, not just the base URL.
- Use `.../v1/chat/completions` for Chat Completions format.
- Use `.../v1/responses` for Responses format (recommended for Codex Desktop
  and modern OpenAI-compatible stacks).

**Fallback completions appearing instead of AI**

- This is expected when AI is rate-limited, timed out, or returns an
  error. Set `fallback` to `false` to suppress deterministic completions
  entirely (not recommended -- you will get no completions on AI failure).
