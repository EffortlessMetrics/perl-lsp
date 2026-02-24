# AGENTS.md

This file provides guidance to Claude Code when working with this repository.

**Latest Release**: v0.9.1 - Initial Public Alpha
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md)

## Project Overview

This repository contains an **80+ crate** Rust workspace forming a complete Perl development ecosystem with LSP, DAP, parser, and workspace tooling.

### Key Crates

| Crate | Path | Purpose |
|-------|------|---------|
| **perl-parser** | `/crates/perl-parser/` | Main parser library (v3 recursive descent) |
| **perl-lsp** | `/crates/perl-lsp/` | Standalone LSP server binary |
| **perl-dap** | `/crates/perl-dap/` | Debug Adapter Protocol (bridge mode) |
| **perl-lexer** | `/crates/perl-lexer/` | Context-aware tokenizer |
| **perl-parser-core** | `/crates/perl-parser-core/` | Core parsing infrastructure |
| **perl-semantic-analyzer** | `/crates/perl-semantic-analyzer/` | Semantic analysis |
| **perl-corpus** | `/crates/perl-corpus/` | Test corpus |

### Crate Families

| Family | Count | Purpose |
|--------|-------|---------|
| `perl-module-*` | 13 | Module resolution microcrates |
| `perl-lsp-*` | 21 | LSP feature providers |
| `perl-lsp-feature-*` | 7 | Feature governance subsystem (subset of `perl-lsp-*`) |
| `perl-dap-*` | 4 | Debug adapter components |
| `perl-ts-*` | 5 | Tree-sitter integration |
| `perl-workspace-*` | 4 | Workspace discovery and indexing |
| Core leaf crates | ~30 | Token, AST, quote, regex, heredoc, error, etc. |

## Quick Start

### Installation

```bash
# Install LSP server
cargo install perl-lsp

# Or from source
cargo install --path crates/perl-lsp
```

### Usage

```bash
# Run LSP server (for editors)
perl-lsp --stdio

# Run debug adapter
perl-dap

# Build parser from source
cargo build -p perl-parser --release

# Run tests
cargo test --workspace --lib
```

## Essential Commands

**AI tools can run bare `cargo build` and `cargo test`** - the `.cargo/config.toml` ensures correct behavior.

### Build & Test

```bash
cargo build -p perl-lsp --release        # LSP server
cargo build -p perl-parser --release     # Parser library

cargo test --workspace --lib             # All tests
cargo test -p perl-parser                # Parser tests
cargo test -p perl-lsp                   # LSP tests

# LSP tests with threading constraints
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2

# Semantic definition tests (resource-efficient mode)
just ci-lsp-def
```

### Lint & Format

```bash
cargo fmt --all                          # Format code
cargo clippy --workspace                 # Lint all crates
cargo clippy --workspace --lib           # Lint libraries only (faster)
```

### Local CI Gate (REQUIRED before push)

```bash
# Canonical local gate
nix develop -c just ci-gate

# Install pre-push hook (runs gate automatically)
bash scripts/install-githooks.sh
```

### Highlight Testing

```bash
# Run highlight tests with perl-parser AST integration
cd xtask && cargo run --no-default-features -- highlight
```

## Architecture

### Parser Versions
- **v3 (Native)**: Current - recursive descent parser with ~100% Perl 5 syntax coverage
- **v2 (Pest)**: Legacy - kept out of default gate
- **v1 (C-based)**: Benchmarking only

### Scanner Architecture

The scanner uses a unified Rust-based architecture with C compatibility wrapper:
- **Rust Scanner** (`RustScanner`): Core scanning implementation
- **C Scanner Wrapper** (`CScanner`): Compatibility wrapper delegating to `RustScanner`

### Key Design Patterns

**Dual Indexing**: Functions are indexed under both qualified (`Package::function`) and bare (`function`) names for 98% reference coverage.

```rust
// When indexing function calls, always index under both forms
let qualified = format!("{}::{}", package, bare_name);

// Index under bare name
file_index.references.entry(bare_name.to_string()).or_default().push(symbol_ref.clone());

// Index under qualified name
file_index.references.entry(qualified).or_default().push(symbol_ref);
```

**Adaptive Threading**: LSP tests use thread-aware timeout scaling for CI environments.

**Incremental Parsing**: <1ms LSP updates with 70-99% node reuse efficiency.

## Key Features

- **~100% Perl Syntax Coverage**: All modern Perl constructs including heredocs, regex, quotes, substitution operators, and enhanced builtin function parsing
- **LSP Server**: ~92% of LSP features functional with comprehensive workspace support
- **Debug Adapter Protocol (DAP)**: Full debugging support via bridge to Perl::LanguageServer
- **Semantic Analysis**: 100% AST node coverage with lexical scoping, package boundaries, and multi-symbol support
- **Cross-File Navigation**: Dual indexing with 98% reference coverage
- **Unicode-Safe**: Full UTF-8/UTF-16 handling with symmetric position conversion
- **Security**: Path traversal prevention, file completion safeguards, UTF-16 boundary safety

## Development Guidelines

### Choosing a Crate
1. **Perl Parsing**: Use `perl-parser`
2. **IDE Integration**: Install `perl-lsp`
3. **Debugging**: Use `perl-dap`
4. **Testing Parsers**: Use `perl-corpus`
5. **Legacy Migration**: Migrate from `perl-parser-pest` to `perl-parser`

### Development Locations
- **Parser**: `/crates/perl-parser/src/`
- **LSP Server**: `/crates/perl-lsp/src/`
- **LSP Providers**: `/crates/perl-lsp-*/src/`
- **DAP Server**: `/crates/perl-dap/src/`
- **Module Resolution**: `/crates/perl-module-*/src/`
- **Lexer**: `/crates/perl-lexer/src/`
- **Tests**: `/crates/*/tests/`

## Coding Standards

- Run `cargo clippy --workspace` before committing
- Use `cargo fmt` for consistent formatting
- **No fatal constructs in production code**: `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()` are banned - use `?`, `.ok_or_else()`, or pattern matching
- In tests: use `Result<()>` return types, or `perl_tdd_support::must`/`must_some` helpers
- Prefer `.first()` over `.get(0)`
- Use `.push(char)` instead of `.push_str("x")` for single chars
- Use `or_default()` instead of `or_insert_with(Vec::new)`
- Avoid unnecessary `.clone()` on Copy types

## Documentation

See the [docs/](docs/) directory for comprehensive documentation:

- **[Commands Reference](docs/COMMANDS_REFERENCE.md)** - Build/test commands
- **[LSP Implementation Guide](docs/LSP_IMPLEMENTATION_GUIDE.md)** - Server architecture
- **[Current Status](docs/CURRENT_STATUS.md)** - Computed project health metrics
- **[DAP User Guide](docs/DAP_USER_GUIDE.md)** - Debugger setup and usage
- **[Stability Policy](docs/STABILITY.md)** - API versioning and compatibility
- **[Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)** - System design and components

## Contributing

1. **Parser improvements** -> `/crates/perl-parser/src/`
2. **LSP features** -> `/crates/perl-lsp-*/src/`
3. **CLI enhancements** -> `/crates/perl-lsp/src/`
4. **DAP features** -> `/crates/perl-dap/src/`
5. **Module resolution** -> `/crates/perl-module-*/src/`
6. **Testing** -> `/crates/*/tests/`

See [CONTRIBUTING.md](CONTRIBUTING.md) for full guidelines.
