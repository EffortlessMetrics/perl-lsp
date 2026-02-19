# perl-lsp-transport

LSP transport layer with Content-Length message framing for
[perl-lsp](https://github.com/EffortlessMetrics/tree-sitter-perl-rs).

## Overview

Implements the LSP Base Protocol message framing over stdio. Provides
synchronous functions for reading JSON-RPC requests and writing responses
or notifications, each wrapped with the required `Content-Length` header.

## Public API

| Function             | Description                                      |
|----------------------|--------------------------------------------------|
| `read_message`       | Read and parse a `JsonRpcRequest` from a reader  |
| `write_message`      | Write a `JsonRpcResponse` with Content-Length framing |
| `write_notification` | Write a JSON-RPC notification with framing       |
| `log_response`       | Log an outgoing response to stderr for debugging |

## Usage

```rust
use std::io::{BufReader, stdin, stdout};
use perl_lsp_transport::{read_message, write_message};
use perl_lsp_protocol::JsonRpcResponse;

let mut reader = BufReader::new(stdin());
let mut writer = stdout();

if let Ok(Some(request)) = read_message(&mut reader) {
    let response = JsonRpcResponse::null(request.id);
    write_message(&mut writer, &response).unwrap();
}
```

## License

Licensed under either of Apache License 2.0 or MIT license, at your option.
