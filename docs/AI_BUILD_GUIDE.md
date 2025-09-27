# AI Tool Build Guide for tree-sitter-perl

This guide helps AI tools (and humans) understand how to build and test this project correctly.

## Default Build Configuration

The project has automatic configuration in `.cargo/config.toml` that applies to all `cargo` commands:

- **Optimized dev builds**: `opt-level = 1` for better parser performance testing
- **Full release optimization**: `lto = true`, `codegen-units = 1` for benchmarks
- **Automatic backtraces**: `RUST_BACKTRACE=1` for debugging
- **Sparse registry**: Faster dependency updates

## Quick Commands for AI Tools

```bash
# Standard build (automatically uses correct features)
cargo build

# Build the recommended v3 parser
cargo build -p perl-lexer -p perl-parser

# Build the LSP server
cargo build -p perl-parser --bin perl-lsp

# Run all tests
cargo test --all

# Run benchmarks
cargo bench

# Use xtask for complex operations
cargo xtask test
cargo xtask check --all
cargo xtask bench
```

## Feature Flags

The v2 Pest-based parser (`tree-sitter-perl-rs`) has these features:
- `pure-rust` (default): Enable the Pest-based parser
- `test-utils`: Enable testing utilities

**Important**: Most AI tools should focus on the v3 parser (`perl-lexer` + `perl-parser`) which doesn't require feature flags.

## Parser Selection Guide

1. **v3 Native Parser** (RECOMMENDED):
   ```bash
   cargo build -p perl-lexer -p perl-parser
   cargo test -p perl-lexer -p perl-parser
   ```

2. **v2 Pest Parser**:
   ```bash
   cargo build -p tree-sitter-perl --features pure-rust
   cargo test -p tree-sitter-perl --features pure-rust
   ```

3. **LSP Server**:
   ```bash
   cargo build -p perl-parser --bin perl-lsp --release
   ```

## Common AI Tool Scenarios

### "Build the project"
```bash
cargo build --all
```

### "Run tests"
```bash
cargo test --all
```

### "Check code quality"
```bash
cargo xtask check --all
```

### "Parse a Perl file"
```bash
# Using v3 parser (via example)
cargo run -p perl-parser --example test_parser -- file.pl

# Using v2 parser
cargo run -p tree-sitter-perl --bin parse-rust -- file.pl
```

### "Benchmark parsers"
```bash
cargo xtask compare
```

## Environment Variables

These are automatically set by `.cargo/config.toml`:
- `RUST_BACKTRACE=1`: Full error traces
- Target directory: `./target` (shared across all crates)

## Notes for AI Tools

1. **Always use `cargo xtask` for complex operations** - it handles multi-step workflows correctly
2. **The v3 parser is recommended** - it's faster and more complete
3. **Don't manually set feature flags** - the defaults are configured correctly
4. **Release builds are optimized** - use `--release` for performance testing

## Troubleshooting

If builds fail:
1. Check Rust version: `rustc --version` (requires stable Rust)
2. Update dependencies: `cargo update`
3. Clean build: `cargo clean && cargo build`

For parser-specific issues, see `CLAUDE.md` for detailed architecture information.