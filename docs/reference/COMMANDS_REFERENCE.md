# Commands Reference

This reference lists the canonical commands for installing, building, testing, and validating the `perl-lsp` workspace.

## Install

### LSP server
```bash
# Install from crates.io
cargo install perl-lsp

# Install from local source checkout
cargo install --path crates/perl-lsp

# Best-effort binary installer (non-canonical)
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash
```

### DAP server
```bash
# Install from crates.io
cargo install perl-dap

# Install from local source checkout
cargo install --path crates/perl-dap
```

## Run

```bash
# Start the LSP server (editor integration)
perl-lsp --stdio

# Start the debug adapter
perl-dap
```

## Build

```bash
# Build the LSP binary
cargo build -p perl-lsp --release

# Build the parser library
cargo build -p perl-parser --release

# Build everything in the workspace
cargo build
```

## Test

```bash
# Fast workspace library test pass
cargo test --workspace --lib

# Targeted crate tests
cargo test -p perl-parser
cargo test -p perl-lsp

# Thread-constrained LSP test run (useful for CI/containers)
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2

# Resource-efficient semantic definition lane
just ci-lsp-def
```

## Lint and format

```bash
# Format all crates
cargo fmt --all

# Lint entire workspace
cargo clippy --workspace

# Faster lint pass for libraries only
cargo clippy --workspace --lib
```

## Local CI gate

```bash
# Canonical pre-push local gate
nix develop -c just ci-gate

# Optional: install git hooks to run gate automatically
bash scripts/install-githooks.sh
```

## Highlight testing

```bash
cd xtask && cargo run --no-default-features -- highlight
```
