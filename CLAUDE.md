# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

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

The workspace contains 40+ crates organized in tiers. Key crates:

| Crate | Path | Purpose |
|-------|------|---------|
| **perl-parser** | `/crates/perl-parser/` | Main parser library (v3 recursive descent) |
| **perl-lsp** | `/crates/perl-lsp/` | Standalone LSP server binary |
| **perl-dap** | `/crates/perl-dap/` | Debug Adapter Protocol (bridge mode) |
| **perl-lexer** | `/crates/perl-lexer/` | Context-aware tokenizer |
| **perl-parser-core** | `/crates/perl-parser-core/` | Core parsing infrastructure |
| **perl-workspace-index** | `/crates/perl-workspace-index/` | Workspace symbol indexing |
| **perl-semantic-analyzer** | `/crates/perl-semantic-analyzer/` | Semantic analysis |
| **perl-corpus** | `/crates/perl-corpus/` | Test corpus |
| **perl-parser-pest** | `/crates/perl-parser-pest/` | Legacy Pest parser |

Supporting crates: `perl-lsp-*` (providers), `perl-dap-*` (debug components), `perl-token`, `perl-ast`, `perl-quote`, `perl-regex`, `perl-heredoc`, `perl-error`

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
cargo test test_name                  # Run single test by name
cargo test -p perl-parser -- test_name --exact  # Run exact test in crate

# LSP tests with threading constraints
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2

# Semantic definition tests (resource-efficient mode)
just ci-lsp-def
```

### Benchmarks and Fuzzing

```bash
just benchmarks                       # Run all benchmarks
cargo bench -p perl-parser            # Parser benchmarks
just fuzz-bounded                     # Bounded fuzz run (60s per target)
just mutation-subset                  # Mutation testing subset
```

### Dead Code Detection

```bash
just dead-code                        # Full dead code report
just dead-code-report                 # Generate JSON report
just dead-code-strict                 # Run in strict mode (fail on any dead code)
cargo machete                         # Check unused dependencies (fast)
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

### Supply Chain Security

```bash
just sbom                             # Generate SBOM (both formats)
just sbom-spdx                        # Generate SBOM in SPDX format
just sbom-cyclonedx                   # Generate SBOM in CycloneDX format
just sbom-verify                      # Verify SBOM generation
just security-audit                   # Run security audit (cargo-audit)

# Verify release artifact provenance
gh attestation verify <artifact> --owner EffortlessMetrics
```

### Code Coverage

```bash
just coverage                         # Generate HTML coverage report locally
just coverage-summary                 # Show coverage summary in terminal
just coverage-lcov                    # Generate lcov.info for CI
```

### SemVer Checking

```bash
just semver-check                     # Check all published packages
just semver-check-package <name>      # Check specific package
just semver-diff <name>               # Show API diff for package
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

### CI Gate Tiers

| Tier | Command | Time | When to Use |
|------|---------|------|-------------|
| **A (PR-fast)** | `just pr-fast` | ~1-2 min | Quick iteration during development |
| **B (Merge gate)** | `just ci-gate` | ~3-5 min | Before pushing (required) |
| **C (Nightly)** | `just ci-full` | ~15-30 min | Mutation testing, fuzzing, benchmarks |

## Parser Versions

- **v3 (Native)**: Current - recursive descent parser
- **v2 (Pest)**: Legacy - kept out of default gate
- **v1 (C-based)**: Benchmarking only

## Workspace Exclusions

These directories are excluded from the default workspace (require special builds):
- `tree-sitter-perl-c/` - Requires libclang
- `fuzz/` - Specialized fuzz testing build
- `archive/` - Legacy components

## Key Paths

| What | Where |
|------|-------|
| Parser source | `crates/perl-parser/src/` |
| LSP providers | `crates/perl-lsp-*/src/` |
| LSP server binary | `crates/perl-lsp/src/` |
| DAP server | `crates/perl-dap/src/` |
| Tests | `crates/*/tests/` |
| Test corpus | `test_corpus/`, `tree-sitter-perl/test/corpus/` |
| Fuzz targets | `fuzz/fuzz_targets/` |
| VSCode extension | `vscode-extension/` |
| Documentation | `docs/` |
| Features catalog | `features.toml` |
| CI gate policy | `.ci/gate-policy.yaml` |
| Technical debt ledger | `.ci/debt-ledger.yaml` |
| Dependabot config | `.github/dependabot.yml` |
| Supply chain security | `deny.toml`, `docs/SUPPLY_CHAIN_SECURITY.md` |
| Build tooling | `xtask/` |

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

### Crate Dependency Tiers

The workspace uses a tiered dependency structure (see `Cargo.toml`):
- **Tier 1**: Leaf crates with no internal dependencies (`perl-token`, `perl-quote`, `perl-ast`, etc.)
- **Tier 2**: Single-level dependencies (`perl-parser-core`, `perl-lsp-transport`, etc.)
- **Tier 3**: Two-level dependencies (`perl-workspace-index`, `perl-refactoring`)
- **Tier 4**: Three-level dependencies (`perl-semantic-analyzer`, `perl-lsp-providers`)
- **Tier 5**: Application crates (`perl-parser`, `perl-lsp`, `perl-dap`)

## Documentation

- **[CURRENT_STATUS.md](docs/CURRENT_STATUS.md)** - Computed metrics and project health
- **[ROADMAP.md](docs/ROADMAP.md)** - Milestones and release planning
- **[COMMANDS_REFERENCE.md](docs/COMMANDS_REFERENCE.md)** - Full command catalog
- **[LSP_IMPLEMENTATION_GUIDE.md](docs/LSP_IMPLEMENTATION_GUIDE.md)** - Server architecture
- **[DEBT_TRACKING.md](docs/DEBT_TRACKING.md)** - Technical debt and flaky test tracking
- **[DEPENDENCY_MANAGEMENT.md](docs/DEPENDENCY_MANAGEMENT.md)** - Automated dependency updates with Dependabot
- **[DEPENDENCY_QUICK_REFERENCE.md](docs/DEPENDENCY_QUICK_REFERENCE.md)** - Quick commands for dependency management
- **[features.toml](features.toml)** - Canonical LSP capability definitions

## Truth Sources

Metrics in this project are **computed, not hand-edited**:
- `CURRENT_STATUS.md` - Auto-generated via `scripts/update-current-status.py`
- `features.toml` - Canonical LSP capability definitions
- Test output and CI receipts are the evidence for all claims

## Coding Standards

- Run `cargo clippy --workspace` before committing
- Use `cargo fmt` for consistent formatting
- **No fatal constructs in production code** - the following are banned:
  - `unwrap()`, `expect()` - use `?`, `.ok_or_else()`, or pattern matching
  - `panic!()`, `todo!()`, `unimplemented!()` - return `Result`/`Option`
  - `std::process::abort()` - never use, not even in binaries
  - `std::process::exit()` - allowed **only** in `bin/` directories and `lifecycle.rs`
  - `dbg!()` - use `tracing::debug!` instead
  - **Exception**: One centralized `#[allow(clippy::expect_used)]` for `lsp_types::Uri` fallback (see `crates/perl-lsp/src/util/uri.rs`)
  - In tests: use `Result<()>` return types, or `perl_tdd_support::must`/`must_some` helpers
- **Regex init**: Use `Option<Regex>` with `.ok()` for graceful degradation
- **Non-empty collections**: Use fixed-size arrays (`[T; N]`) for compile-time guarantees
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
| LSP features | `/crates/perl-lsp-*/src/` |
| CLI enhancements | `/crates/perl-lsp/src/` |
| DAP features | `/crates/perl-dap/src/` |
| Tests | `/crates/*/tests/` |
