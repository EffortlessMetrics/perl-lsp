# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

**Latest Release**: 0.9.0
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md)
**Metrics**: See [docs/CURRENT_STATUS.md](docs/CURRENT_STATUS.md) for computed status

## Quick Reference

```bash
# Canonical local gate (REQUIRED before push)
nix develop -c just ci-gate

# Build and run LSP server
cargo build -p perl-lsp --release
./target/release/perl-lsp --stdio

# Run all tests
cargo test --workspace --lib
```

## Crate Structure

| Crate | Path | Purpose |
|-------|------|---------|
| **perl-parser** | `/crates/perl-parser/` | Main parser library (v3 recursive descent) |
| **perl-lsp** | `/crates/perl-lsp/` | Standalone LSP server binary |
| **perl-dap** | `/crates/perl-dap/` | Debug Adapter Protocol (bridge mode) |
| **perl-lexer** | `/crates/perl-lexer/` | Context-aware tokenizer |
| **perl-corpus** | `/crates/perl-corpus/` | Test corpus |
| **perl-parser-pest** | `/crates/perl-parser-pest/` | Legacy Pest parser |

## Essential Commands

### Build

```bash
cargo build -p perl-lsp --release     # LSP server
cargo build -p perl-parser --release  # Parser library
cargo install --path crates/perl-lsp  # Install from source
```

### Test

```bash
cargo test                            # All tests
cargo test -p perl-parser             # Parser tests
cargo test -p perl-lsp                # LSP tests

# LSP tests with threading constraints
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2

# Semantic definition tests (resource-efficient mode)
just ci-lsp-def
```

### Lint and Format

```bash
cargo fmt --all                       # Format code
cargo clippy --workspace              # Lint all crates
cargo clippy --workspace --lib        # Lint libraries only (faster)
```

### Health and Status

```bash
just health                           # Show codebase metrics
just status-check                     # Verify computed metrics are current
bash scripts/ignored-test-count.sh    # Show ignored test counts
just debt-report                      # Show technical debt status
just debt-check                       # Verify debt budget compliance
```

## Development Workflow

**Local-first development** - all gates run locally before CI:

```bash
# Install pre-push hook
bash scripts/install-githooks.sh

# Run gate before pushing (format, clippy, tests, policy)
nix develop -c just ci-gate
```

CI is optional/opt-in. The repo is local-first by design.

## Parser Versions

- **v3 (Native)**: Current - recursive descent parser
- **v2 (Pest)**: Legacy - kept out of default gate
- **v1 (C-based)**: Benchmarking only

## Key Paths

| What | Where |
|------|-------|
| Parser source | `crates/perl-parser/src/` |
| LSP providers | `crates/perl-parser/src/lsp/` |
| LSP server binary | `crates/perl-lsp/src/` |
| Tests | `crates/*/tests/` |
| Test corpus | `test_corpus/`, `tree-sitter-perl/test/corpus/` |
| Documentation | `docs/` |
| Features catalog | `features.toml` |
| CI gate policy | `.ci/gate-policy.yaml` |
| Technical debt ledger | `.ci/debt-ledger.yaml` |

## Architecture Patterns

### Dual Indexing (PR #122)

When implementing workspace indexing, index under both qualified and bare forms:

```rust
// Index under bare name
file_index.references.entry(bare_name.to_string()).or_default().push(symbol_ref.clone());

// Index under qualified name
file_index.references.entry(qualified).or_default().push(symbol_ref);
```

### Threading Configuration

LSP tests use adaptive threading. Key environment variables:

```bash
RUST_TEST_THREADS=2     # Limit test parallelism
CARGO_BUILD_JOBS=1      # Limit build parallelism
RUSTC_WRAPPER=""        # Disable rustc wrapper
```

## Documentation

- **[CURRENT_STATUS.md](docs/CURRENT_STATUS.md)** - Computed metrics and project health
- **[ROADMAP.md](docs/ROADMAP.md)** - Milestones and release planning
- **[COMMANDS_REFERENCE.md](docs/COMMANDS_REFERENCE.md)** - Full command catalog
- **[LSP_IMPLEMENTATION_GUIDE.md](docs/LSP_IMPLEMENTATION_GUIDE.md)** - Server architecture
- **[DEBT_TRACKING.md](docs/DEBT_TRACKING.md)** - Technical debt and flaky test tracking
- **[features.toml](features.toml)** - Canonical LSP capability definitions

## Coding Standards

- Run `cargo clippy --workspace` before committing
- Use `cargo fmt` for consistent formatting
- **No `unwrap()` or `expect()`** - workspace enforces `clippy::unwrap_used` and `clippy::expect_used` as deny
  - In production code: use `?`, `.ok_or_else()`, or pattern matching
  - In tests: use `#[allow(clippy::unwrap_used, clippy::expect_used)]` on test modules, or convert tests to return `Result`
- Prefer `.first()` over `.get(0)`
- Use `.push(char)` instead of `.push_str("x")` for single chars
- Use `or_default()` instead of `or_insert_with(Vec::new)`
- Avoid unnecessary `.clone()` on Copy types

## Contributing

1. Run `nix develop -c just ci-gate` before pushing
2. Check issues for "good first issue" labels
3. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines

| Area | Path |
|------|------|
| Parser improvements | `/crates/perl-parser/src/` |
| LSP features | `/crates/perl-parser/src/lsp/` |
| CLI enhancements | `/crates/perl-lsp/src/` |
| DAP features | `/crates/perl-dap/src/` |
| Tests | `/crates/*/tests/` |
