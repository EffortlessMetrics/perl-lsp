# perl-lsp-providers

LSP provider glue and tooling integrations for Perl.

## Overview

This crate provides Language Server Protocol feature implementations for the Perl LSP ecosystem, including:

- **Completion** - Context-aware code completion with workspace symbol resolution
- **Definition** - Go-to-definition with dual indexing (qualified/bare names)
- **References** - Find all references with cross-file workspace navigation
- **Hover** - Documentation and type information on hover
- **Diagnostics** - Syntax validation and error reporting
- **Code Actions** - Quick fixes and pragma management
- **Semantic Tokens** - Syntax highlighting with LSP semantic tokens
- **Inlay Hints** - Inline type and parameter hints
- **Folding Ranges** - Code folding support
- **Document Highlights** - Symbol highlighting in current file
- **Signature Help** - Function signature assistance
- **Call Hierarchy** - Function call graph navigation
- **Selection Range** - Smart selection expansion
- **Document Links** - Module and file path linking

## Features

- `lsp-compat` (default) - LSP protocol type compatibility via `lsp-types`

## Tooling Integrations

- **perltidy** - Code formatting via external perltidy integration
- **perlcritic** - Linting and policy checking via external perlcritic integration

## Usage

```rust
use perl_parser_core::Parser;
use perl_lsp_providers::completion::CompletionProvider;

let source = "my $value = 1; $";
let mut parser = Parser::new(source);
let ast = parser.parse().expect("valid source");

let provider = CompletionProvider::new(&ast);
let completions = provider.get_completions_with_path(source, source.len(), None);
assert!(!completions.is_empty());
```

## Microcrate Compatibility

The providers crate now exposes two stable import styles:

- Preferred top-level re-exports (microcrate-first):
  - `perl_lsp_providers::completion::CompletionProvider`
  - `perl_lsp_providers::diagnostics::DiagnosticsProvider`
  - `perl_lsp_providers::formatting::FormattingProvider`
  - `perl_lsp_providers::tooling::SubprocessRuntime`
- Deprecated legacy shims (kept for compatibility):
  - `perl_lsp_providers::ide::lsp_compat::completion::CompletionProvider`
  - `perl_lsp_providers::ide::lsp_compat::diagnostics::DiagnosticsProvider`
  - `perl_lsp_providers::ide::lsp_compat::formatting::FormattingProvider`

## Architecture

Providers leverage the Perl LSP parsing pipeline:

1. **Parse** - Syntax analysis via `perl-parser-core`
2. **Index** - Symbol indexing via `perl-workspace-index`
3. **Analyze** - Semantic analysis via `perl-semantic-analyzer`
4. **Navigate** - Cross-file navigation with dual indexing patterns
5. **Complete** - Context-aware completion with workspace symbols

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

See the main [perl-lsp](https://github.com/EffortlessMetrics/tree-sitter-perl-rs) repository for contribution guidelines.
