# perl-lsp-tooling

External tool integration and performance infrastructure for the Perl LSP ecosystem.

## Features

- **Subprocess abstraction**: `SubprocessRuntime` trait with `OsSubprocessRuntime` (non-WASM) and test mocks
- **Perltidy integration**: `PerlTidyFormatter` for code formatting with caching, range formatting, and a `BuiltInFormatter` fallback
- **Perlcritic integration**: `CriticAnalyzer` for static analysis with `BuiltInAnalyzer` and `Policy` trait for custom policies
- **Performance**: `AstCache` (moka-based concurrent cache with TTL), `IncrementalParser`, `SymbolIndex` (trie + fuzzy), parallel file processing
- **LSP compatibility**: Optional `lsp-compat` feature for `lsp_types` diagnostic conversion

## Workspace Role

Tier 2 infrastructure crate in the `tree-sitter-perl-rs` workspace. Used by formatting, diagnostics, and code analysis layers of the Perl LSP server.

## Quick Start

```rust
use perl_lsp_tooling::{SubprocessRuntime, OsSubprocessRuntime};

let runtime = OsSubprocessRuntime::new();
let output = runtime.run_command("perltidy", &["-st"], Some(b"my $x=1;"))?;
```

## License

MIT OR Apache-2.0
