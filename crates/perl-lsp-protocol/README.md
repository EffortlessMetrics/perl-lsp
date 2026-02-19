# perl-lsp-protocol

JSON-RPC 2.0 message types, LSP error codes, method name constants, and server capability configuration for `perl-lsp`.

## Public API

- **`jsonrpc`** -- `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError` structs for JSON-RPC 2.0 communication
- **`errors`** -- Standard JSON-RPC/LSP error codes (`PARSE_ERROR`, `METHOD_NOT_FOUND`, `REQUEST_CANCELLED`, etc.) and builder helpers (`cancelled_response`, `method_not_found`, `internal_error`, `req_uri`, `req_position`, `req_range`)
- **`methods`** -- `&str` constants for every LSP 3.17 method name (lifecycle, text document, workspace, window)
- **`capabilities`** -- `BuildFlags`, `AdvertisedFeatures`, `capabilities_for()`, `default_capabilities()` for constructing `lsp_types::ServerCapabilities`

## Features

- `lsp-ga-lock` -- Use conservative GA-lock capability set instead of the full production set

## Role in the workspace

Tier 1 leaf crate with no internal workspace dependencies. Consumed by `perl-lsp`, `perl-lsp-providers`, `perl-lsp-transport`, and other `perl-lsp-*` crates to share protocol definitions without pulling in the full LSP runtime.

## License

Licensed under MIT OR Apache-2.0 at your option. See the [repository](https://github.com/EffortlessMetrics/perl-lsp) for details.
