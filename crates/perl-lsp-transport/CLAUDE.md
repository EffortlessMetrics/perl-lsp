# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-transport` is a **Tier 2 lower-level transport crate** providing LSP message framing over stdio.

**Purpose**: LSP transport layer with Content-Length message framing for perl-lsp — handles JSON-RPC message encoding/decoding.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lsp-transport        # Build this crate
cargo test -p perl-lsp-transport         # Run tests
cargo clippy -p perl-lsp-transport       # Lint
cargo doc -p perl-lsp-transport --open   # View documentation
```

## Architecture

### Dependencies

- `perl-lsp-protocol` - Protocol types
- `serde_json` - JSON serialization

### LSP Message Format

```
Content-Length: 123\r\n
\r\n
{"jsonrpc":"2.0","id":1,"method":"...","params":{...}}
```

### Key Types

| Type | Purpose |
|------|---------|
| `Transport` | Message read/write abstraction |
| `Message` | Parsed JSON-RPC message |
| `FrameReader` | Content-Length frame parsing |
| `FrameWriter` | Content-Length frame encoding |

## Usage

```rust
use perl_lsp_transport::{Transport, Message};
use std::io::{stdin, stdout};

// Create transport over stdio
let transport = Transport::new(stdin(), stdout());

// Read message
let message = transport.read()?;

match message {
    Message::Request { id, method, params } => {
        // Handle request
        transport.write(Message::Response { id, result })?;
    },
    Message::Notification { method, params } => {
        // Handle notification
    },
    _ => {}
}
```

### Frame Parsing

```rust
use perl_lsp_transport::FrameReader;

let mut reader = FrameReader::new(input);

// Read Content-Length header
let content_length = reader.read_header()?;

// Read JSON body
let body = reader.read_body(content_length)?;
```

## Important Notes

- Transport is synchronous (async handled at higher level)
- Strict adherence to LSP message format
- Content-Length is mandatory per spec
- UTF-8 encoding for message bodies
