# perl-lsp-transport

LSP transport layer with Content-Length message framing for perl-lsp.

## Overview

This crate provides transport layer implementations for the Perl Language Server Protocol, handling message framing according to the LSP Base Protocol specification. It supports stdio and TCP transports with proper Content-Length based framing.

## Features

- **Message Framing**: Content-Length based message framing per LSP specification
- **Read/Write Operations**: Async-ready message reading and writing
- **Protocol Compliance**: Full LSP Base Protocol transport layer support
- **Stdio Transport**: Standard input/output transport for editor integration
- **TCP Transport**: Network-based transport for remote LSP servers

## Usage

```rust
use std::io::{BufReader, stdin, stdout};
use perl_lsp_transport::{read_message, write_message};
use perl_lsp_protocol::JsonRpcResponse;

let mut reader = BufReader::new(stdin());
let mut writer = stdout();

// Read an incoming LSP message
if let Ok(Some(request)) = read_message(&mut reader) {
    // Process request and create response
    let response = JsonRpcResponse::null(request.id);

    // Write the response with proper framing
    write_message(&mut writer, &response).unwrap();
}
```

## API

- `read_message` - Read and parse an LSP message with Content-Length framing
- `write_message` - Write an LSP response with proper framing
- `write_notification` - Write an LSP notification with proper framing
- `log_response` - Debug logging for outgoing responses

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
