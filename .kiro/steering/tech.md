# Tech Stack and Build System

## Language and Toolchain
- **Language**: Rust (Edition 2024)
- **MSRV**: 1.92 (pinned in `rust-toolchain.toml`)
- **Toolchain components**: rustfmt, clippy (minimal profile)

## Build System
- **Cargo** workspace with ~35 member crates under `crates/`
- **just** task runner for all build/test/lint/CI commands (install: `cargo install just`)
- **Nix** flake for reproducible dev environment (optional but recommended)
- **cargo xtask** for custom build tasks (formatting, CI hygiene, etc.)

## Key Dependencies
- `lsp-types` 0.97 — LSP protocol types
- `tokio` 1.x — async runtime (multi-thread, net, io-util, sync, time)
- `serde` / `serde_json` — serialization
- `ropey` — rope data structure for text manipulation
- `tree-sitter` 0.26 — tree-sitter bindings (for tree-sitter-perl crates)
- `clap` 4.x — CLI argument parsing
- `tracing` — structured logging (use instead of println/eprintln)
- `proptest` — property-based testing
- `criterion` — benchmarking
- `parking_lot` — synchronization primitives
- `regex` — regular expressions (must be `static LazyLock<Regex>`, never per-call)
- `thiserror` — error derive macros
- `anyhow` — error handling

## Common Commands

```bash
# Fast PR validation (~1-2 min) — run before every push
just pr-fast

# Full pre-merge gate (~3-5 min) — required before merge
nix develop -c just ci-gate
# or without Nix:
just ci-gate

# Build the LSP server binary
cargo build -p perl-lsp-rs --release

# Test a specific crate
cargo test -p <crate>

# Test full workspace (lib tests only)
cargo test --workspace --lib

# Check all targets compile (catches integration test bit-rot)
cargo check --all-targets -p <crate>

# Format code (Windows-safe, per-crate)
cargo xtask fmt

# Lint
cargo clippy -p <crate>
cargo clippy --workspace          # full workspace

# Workspace health check / diagnostics
just doctor

# Show all available just commands
just --list

# Quick reference of common workflows
just quick-ref
```

## Formatting Config (rustfmt.toml)
- `max_width = 100`
- `use_small_heuristics = "Max"`

## Clippy Config (clippy.toml)
- `msrv = "1.92"`
- `too-many-arguments-threshold = 8`
- `cognitive-complexity-threshold = 50`

## Workspace Lints (enforced via Cargo.toml)
These are **denied** at the workspace level:
- `clippy::unwrap_used`
- `clippy::expect_used`
- `clippy::panic`
- `clippy::todo`
- `clippy::unimplemented`
- `clippy::dbg_macro`

## Code Quality Rules
- No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `dbg!()` in any code
- No `println!()` / `eprintln!()` in library code — use `tracing` instead
- Tests must return `Result<()>` or use `perl_tdd_support::must` / `must_some`
- Regex must be `static LazyLock<Regex>` — never `Regex::new()` per invocation
- Public API on facade crates: add `#[non_exhaustive]` to enums and structs
- Do not hold a lock across `.await`
- Every `#[allow(...)]` must have a justification comment
- Prefer `.first()` over `.get(0)`, `.push(ch)` over `.push_str("x")` for single chars, `.or_default()` over `.or_insert_with(Vec::new)`
