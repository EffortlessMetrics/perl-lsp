# perl-lsp-protocol

JSON-RPC/LSP protocol types and capability configuration for the Perl Language Server.

## Overview

This crate provides the foundational protocol layer for `perl-lsp`, including:

- **JSON-RPC 2.0** message types and serialization
- **LSP protocol** type definitions and extensions
- **Server capabilities** configuration and negotiation
- **Error handling** for protocol-level failures

## Usage

```rust
use perl_lsp_protocol::{
    capabilities::ServerCapabilities,
    jsonrpc::{Request, Response},
    methods::LspMethod,
};
```

This crate is primarily intended for use within the `perl-lsp` server implementation.

## Features

- `lsp-ga-lock`: Enable LSP global allocator locking (optional)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

See the main [perl-lsp repository](https://github.com/EffortlessMetrics/tree-sitter-perl-rs) for contribution guidelines.
