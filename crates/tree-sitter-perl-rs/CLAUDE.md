# CLAUDE.md

Guidance for Claude Code when working in `crates/tree-sitter-perl-rs/`.

## Crate Overview

- **Package name**: `tree-sitter-perl` (lib: `tree_sitter_perl`)
- **Tier**: 7 (legacy/testing)
- **Version**: 0.9.1 (workspace)
- **Publish**: false (internal only)
- **Purpose**: Pure-Rust Pest-based Perl 5 parser that emits tree-sitter-compatible S-expressions. Also serves as a comparison/benchmark harness against the native v3 parser and the legacy C tree-sitter parser.

## Commands

```bash
cargo build -p tree-sitter-perl                    # Build
cargo test  -p tree-sitter-perl                    # Test
cargo clippy -p tree-sitter-perl                   # Lint
cargo doc   -p tree-sitter-perl --open             # Docs

# Parser comparison and benchmarking binaries
cargo run -p tree-sitter-perl --bin ts_test_parsers      --features pure-rust
cargo run -p tree-sitter-perl --bin ts_benchmark_parsers --features pure-rust
cargo run -p tree-sitter-perl --bin bench_parser         --features test-utils
cargo run -p tree-sitter-perl --bin compare_parsers      --features test-utils
```

## Architecture

### Dependencies

- **perl-lexer**, **perl-parser** -- used for native parser comparison (excluded under `pure-rust-standalone`)
- **perl-parser-pest** (optional, default via `v2-pest-microcrate`) -- supplies `PureRustPerlParser`, `PerlParser`, `Rule` through bridge modules
- **perl-ts-heredoc-analysis**, **perl-ts-logos-lexer**, **perl-ts-heredoc-parser**, **perl-ts-partial-ast**, **perl-ts-advanced-parsers** -- optional `pure-rust` sub-crates re-exported from `lib.rs`
- **tree-sitter** 0.26 -- used only under the `c-parser` feature for FFI benchmarking

### Key Types and Modules

| Item | Path / Feature | Role |
|------|----------------|------|
| `PureRustPerlParser` | `pure_rust_parser` (`pure-rust`) | Main Pest parser; `parse()` returns AST, `to_sexp()` emits S-expressions |
| `PerlParser`, `AstNode` | `pure_rust_parser` (`pure-rust`) | Pest `Rule`-level parser and AST node type |
| `EnhancedPerlParser` | re-export from `perl-ts-advanced-parsers` | Context-aware parser variant |
| `FullPerlParser`, `EnhancedFullParser` | re-export from `perl-ts-advanced-parsers` | Full-coverage parser variants |
| `ComparisonHarness` | `comparison_harness` (`pure-rust` or `test-utils`) | Runs both C and Rust parsers, collects timing |
| `language()`, `parse()`, `create_ts_parser()` | lib.rs (`c-parser`) | Tree-sitter C FFI entry points |
| `PerlScanner` trait | `scanner` module | Scanning interface (legacy, used by C/Rust scanner benchmarks) |
| `error::ParseError` | `error` module | Central error enum (thiserror-based) |
| `unicode` | `unicode` module | Unicode normalization and validation |

### Feature Flags

| Flag | Default | Effect |
|------|---------|--------|
| `pure-rust` | yes | Enables Pest parser and all five `perl-ts-*` sub-crate re-exports |
| `v2-pest-microcrate` | yes | Routes `pure_rust_parser`, `pratt_parser`, `sexp_formatter` through `perl-parser-pest` bridges |
| `c-parser` | no | Compiles C tree-sitter parser via `cc`; enables `language()` / `parse()` |
| `test-utils` | no | Enables `test_utils` module and `bench_parser`/`compare_parsers` binaries |
| `token-parser` | no | Enables logos-based token parser modules |
| `pure-rust-standalone` | no | Pest-only parser without `perl-lexer` dependency (for isolated benchmarks) |

### Bridge Pattern (v2 bundle sync)

When `v2-pest-microcrate` is enabled (default), `pure_rust_parser.rs`, `pratt_parser.rs`, and `sexp_formatter.rs` are replaced by thin `*_bridge.rs` files that re-export from `perl-parser-pest`. Editing any shared source requires syncing both copies; verify with `just ci-v2-bundle-sync`.

## Usage

```rust
use tree_sitter_perl::PureRustPerlParser;

let mut parser = PureRustPerlParser::new();
let ast = parser.parse("my $x = 42;")?;
let sexp = parser.to_sexp(&ast);
println!("{sexp}");
```

## Important Notes

- The cargo package name is `tree-sitter-perl`, **not** `tree-sitter-perl-rs` (the directory name).
- `publish = false` -- this crate is workspace-internal.
- The v2 bundle sync rule applies: changes to `grammar.pest`, `pure_rust_parser.rs`, `pratt_parser.rs`, `sexp_formatter.rs`, or `error.rs` must be mirrored in `perl-parser-pest`. Run `just ci-v2-bundle-sync` to verify.
- The `c-parser` and `bindings` features require a C toolchain / `libclang`.
