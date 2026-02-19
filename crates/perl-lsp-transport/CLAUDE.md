# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

`perl-lsp-transport` is a **Tier 2** crate providing LSP Base Protocol message framing over stdio.

**Purpose**: Synchronous Content-Length based message framing for reading JSON-RPC requests and writing responses/notifications, used by the `perl-lsp` server binary.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-lsp-transport        # Build this crate
cargo test -p perl-lsp-transport         # Run tests
cargo clippy -p perl-lsp-transport       # Lint
cargo doc -p perl-lsp-transport --open   # View documentation
```

## Architecture

### Dependencies

- `perl-lsp-protocol` -- `JsonRpcRequest` and `JsonRpcResponse` types
- `serde_json` -- JSON serialization/deserialization

### Modules

| Module | File | Purpose |
|--------|------|---------|
| (root) | `src/lib.rs` | Re-exports public API from `framing` |
| `framing` | `src/framing.rs` | All implementation: header parsing, body reading, frame writing |

### Public Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `read_message` | `(&mut dyn BufRead) -> io::Result<Option<JsonRpcRequest>>` | Reads headers, parses Content-Length, reads body, deserializes JSON-RPC request. Returns `Ok(None)` on EOF or parse errors (recoverable). |
| `write_message` | `(&mut W: Write, &JsonRpcResponse) -> io::Result<()>` | Serializes response to JSON, writes Content-Length header and body, flushes. |
| `write_notification` | `(&mut W: Write, &str, Value) -> io::Result<()>` | Builds a JSON-RPC 2.0 notification from method name and params, writes with framing. |
| `log_response` | `(&JsonRpcResponse)` | Logs response metadata (id, has_result, has_error, length) to stderr. |

### LSP Message Format

```
Content-Length: 123\r\n
\r\n
{"jsonrpc":"2.0","id":1,"method":"...","params":{...}}
```

## Usage

```rust
use std::io::{BufReader, stdin, stdout};
use perl_lsp_transport::{read_message, write_message, write_notification};
use perl_lsp_protocol::JsonRpcResponse;

let mut reader = BufReader::new(stdin());
let mut writer = stdout();

// Read an incoming request
if let Ok(Some(request)) = read_message(&mut reader) {
    let response = JsonRpcResponse::null(request.id);
    write_message(&mut writer, &response)?;
}

// Send a notification
write_notification(&mut writer, "window/logMessage", serde_json::json!({
    "type": 3,
    "message": "Server started"
}))?;
```

## Important Notes

- All I/O is **synchronous**; async is handled at higher layers (`perl-lsp`).
- `read_message` recovers gracefully from malformed JSON (logs to stderr, returns `Ok(None)`).
- Content-Length header is mandatory per the LSP Base Protocol spec.
- Message bodies are encoded as UTF-8.
- No `unwrap()` or `expect()` in production code per workspace lint policy.
