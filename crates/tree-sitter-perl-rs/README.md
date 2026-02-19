# tree-sitter-perl

Internal pure-Rust Perl parser and validation harness for the
[tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp)
workspace. **Not published to crates.io.**

## Purpose

Provides a Pest-based Perl 5 parser that emits tree-sitter-compatible
S-expressions, plus a comparison harness for benchmarking against the
native v3 recursive-descent parser (`perl-parser`).

## Public API (feature-gated)

| Export | Feature | Description |
|--------|---------|-------------|
| `PureRustPerlParser` | `pure-rust` | Main Pest-based parser |
| `PerlParser`, `AstNode` | `pure-rust` | Grammar rule parser and AST nodes |
| `EnhancedPerlParser`, `FullPerlParser` | `pure-rust` | Advanced parser variants |
| `ComparisonHarness` | `pure-rust` / `test-utils` | C-vs-Rust parser comparison runner |
| `language()`, `parse()` | `c-parser` | Tree-sitter C FFI (benchmarking only) |

## Commands

```bash
cargo build -p tree-sitter-perl                           # build
cargo test  -p tree-sitter-perl                           # test
cargo run   -p tree-sitter-perl --bin ts_test_parsers --features pure-rust  # parser comparison
cargo run   -p tree-sitter-perl --bin ts_benchmark_parsers --features pure-rust  # benchmarks
```

## License

MIT OR Apache-2.0
