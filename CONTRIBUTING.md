# Contributing to Perl LSP

Thank you for your interest in contributing to Perl LSP! Whether you're fixing a bug, improving the parser, or adding an LSP feature, this guide will help you get started.

## Getting Started

### Prerequisites

- **Rust** toolchain (pinned via `rust-toolchain.toml`, MSRV 1.92)
- **Nix** (recommended) for a reproducible dev environment

### Setup

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
nix develop          # Recommended: reproducible environment

# Or without Nix -- just ensure Rust is installed:
# curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build and Test

```bash
cargo build -p perl-lsp --release     # Build the LSP server
cargo test --workspace --lib          # Run all library tests
```

## Project Structure

The workspace contains many crates organized into families. Key crates:

| Crate | Purpose |
|-------|---------|
| `perl-parser` | Main parser (v3 recursive descent) |
| `perl-lsp` | LSP server binary |
| `perl-dap` | Debug Adapter Protocol server |
| `perl-lexer` | Context-aware tokenizer |

Crate families: `perl-module-*` (module resolution), `perl-lsp-*` (LSP providers), `perl-dap-*` (DAP), `perl-workspace-*` (workspace discovery).

For the full crate map, key paths, and architecture details, see [CLAUDE.md](CLAUDE.md).

## Finding Issues to Work On

- Look for issues labeled **`good first issue`** for beginner-friendly tasks
- **`help wanted`** marks issues where maintainer input is available
- **`parser`** issues improve Perl parsing coverage
- **`lsp`** issues add or fix editor features
- Browse [open issues](https://github.com/EffortlessMetrics/perl-lsp/issues) or check the [roadmap](docs/project/ROADMAP.md) for larger goals

## Development Workflow

### 1. Branch

```bash
git checkout -b feature/your-feature-name
```

### 2. Code

Follow the [coding standards](#coding-standards) below.

### 3. Test Locally

```bash
cargo fmt --all                       # Format
cargo clippy --workspace              # Lint
cargo test -p <your-crate>            # Test the crate you changed
```

### 4. Run the CI Gate

You **must** pass the local CI gate before pushing:

```bash
nix develop -c just ci-gate           # Required before push (~3-5 min)
```

For faster iteration during development:

```bash
just pr-fast                          # Quick check (~1-2 min)
```

Install the pre-push hook to run the gate automatically:

```bash
bash scripts/install-githooks.sh
```

### 5. Open a Pull Request

1. Push your branch and open a PR
2. Describe your changes and link related issues (e.g., "Fixes #123")
3. All PRs run format checks, clippy, and tests automatically in CI

#### CI Labels (Opt-in)

Add these labels to trigger additional CI checks:

| Label | What it runs |
|-------|--------------|
| `ci:bench` | Performance benchmarks |
| `ci:strict` | Pedantic clippy |
| `ci:mac` | macOS build |
| `ci:semver` | Breaking change detection |

For full CI details, see [CI & Automation](docs/project/CI.md).

## Coding Standards

### Formatting and Linting

- Run `cargo fmt --all` before every commit
- Fix all `cargo clippy --workspace` warnings
- Use [conventional commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, etc.

### Banned in Production Code

| Banned | Use Instead |
|--------|-------------|
| `unwrap()`, `expect()` | `?`, `.ok_or_else()`, pattern matching |
| `panic!()`, `todo!()`, `unimplemented!()` | Return `Result` or `Option` |
| `dbg!()` | `tracing::debug!` |
| `std::process::exit()` | Only in `bin/` and `lifecycle.rs` |

In tests: use `Result<()>` returns or `perl_tdd_support::must` / `must_some` helpers.

### Style Preferences

- `.first()` over `.get(0)`
- `.push(char)` over `.push_str("x")` for single characters
- `or_default()` over `or_insert_with(Vec::new)`
- Avoid `.clone()` on `Copy` types

### Documentation Anti-Drift

Metrics in this project are **computed, not hand-edited**. Never put exact numeric claims (crate counts, test counts, percentages) in prose files. Link to [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for live metrics instead.

## Testing Guidelines

- Place tests in `tests/` or inline with `#[cfg(test)]`
- Test both success and failure paths
- For parser changes, add edge case tests and run `just cpan-corpus-sweep` to check CPAN coverage

```bash
cargo test -p <crate>                          # Test a specific crate
cargo test -p perl-parser -- test_name --exact # Run an exact test
cargo nextest run                              # Fast parallel runner
```

For LSP tests, control threading to avoid flaky results:

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

See [COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md) for the full command catalog.

## SemVer and Breaking Changes

We follow [Semantic Versioning 2.0.0](https://semver.org/). Check for breaking changes before submitting PRs that modify public APIs:

```bash
just semver-check
```

If a breaking change is necessary:
1. Document it in the PR description with a migration guide
2. Label the PR with `breaking-change`
3. Coordinate with maintainers

See [STABILITY.md](docs/reference/STABILITY.md) for our API stability policy.

## Adding New Crates

1. Create the crate under `crates/` using the naming convention of its family
2. Add it to the workspace `members` in the root `Cargo.toml`
3. Follow the structure of a sibling crate in the same family
4. Run `nix develop -c just ci-gate` to verify

## Getting Help

- **Issues**: [Browse or create issues](https://github.com/EffortlessMetrics/perl-lsp/issues)
- **Discussions**: Use [GitHub Discussions](https://github.com/EffortlessMetrics/perl-lsp/discussions) for questions and ideas
- **Docs**: See `docs/` for detailed guides -- start with [COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md)

## Code of Conduct

We follow the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). Please be respectful and constructive in all interactions.

## License

This project is dual-licensed under [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE). By contributing, you agree that your contributions will be licensed under both licenses.
