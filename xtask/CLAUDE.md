# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`xtask` is a **Tier 5 task runner crate** providing development automation for the Perl LSP workspace.

**Purpose**: Development task runner — benchmarks, corpus management, CI gates, code generation, and release utilities.

**Version**: 0.8.3

## Commands

```bash
cargo xtask --help            # List available tasks
cargo xtask ci                # Run lean CI suite
cargo xtask check-only        # Format and clippy only (no tests)
cargo xtask bench             # Run benchmarks
cargo xtask gates             # Run CI gates with receipt generation
cargo xtask test              # Run tests with various configurations
cargo xtask corpus-audit      # Run corpus audit for coverage analysis
```

## Architecture

### Dependencies

**Internal**:
- `perl-parser` - Parser access for benchmarking
- `tree-sitter-perl` (optional) - Legacy parser tasks

**External**:
- `clap` - CLI argument parsing
- `color-eyre` - Error handling
- `walkdir` - File traversal
- `cargo_metadata` - Workspace introspection
- `serde`, `serde_json`, `serde_yaml_ng` - Configuration parsing
- `indicatif`, `console` - Progress display
- `chrono` - Timestamps
- `notify` - File watching
- `bindgen` - C binding generation

### Main Tasks

| Task | Purpose |
|------|---------|
| `ci` | Run lean CI suite (format, clippy, tests) |
| `check-only` | Format and clippy checks only |
| `build` | Build with various configurations |
| `test` | Run tests with suite/feature selection |
| `bench` | Run benchmarks |
| `compare` | C vs Rust benchmark comparison |
| `gates` | Run CI gates with receipt generation |
| `corpus-audit` | Corpus coverage analysis |
| `parse-rust` | Parse with pure Rust parser |
| `bump-version` | Bump version numbers across project |
| `publish-crates` | Publish crates to crates.io |
| `publish-vscode` | Publish VSCode extension |
| `features` | Manage feature catalog and LSP compliance |

### Features

| Feature | Purpose |
|---------|---------|
| `legacy` | Enable legacy tree-sitter-perl tasks |
| `parser-tasks` | Enable parser-related tasks (corpus, highlight, bindings) |

### Module Structure

| Module | Purpose |
|--------|---------|
| `tasks/` | Task implementations |
| `tasks/gates.rs` | CI gate runner with receipt generation |
| `tasks/ci.rs` | CI suite runner |
| `tasks/bench.rs` | Benchmark runner |
| `tasks/compare.rs` | Parser comparison |
| `tasks/corpus_audit.rs` | Corpus coverage analysis |
| `tasks/features.rs` | Feature catalog management |
| `tasks/publish.rs` | Release publishing |
| `types/` | Shared type definitions |
| `utils/` | Utility functions |

## Usage Examples

### Run CI Gates

```bash
# Run merge-gate tier (default)
cargo xtask gates

# Run PR-fast tier
cargo xtask gates --tier pr-fast

# Generate receipt JSON
cargo xtask gates --receipt

# Compare against baseline
cargo xtask gates --diff target/receipts/baseline.json
```

### Run Benchmarks

```bash
# Run all benchmarks
cargo xtask bench

# Run specific benchmark
cargo xtask bench --name parse_complex

# Save results
cargo xtask bench --save --output results.json
```

### Feature Management

```bash
# Sync documentation from features.toml
cargo xtask features sync-docs

# Verify features match capabilities
cargo xtask features verify

# Generate compliance report
cargo xtask features report
```

### Bumping the Workspace Version for a Release

The workspace version is referenced in ~190 places: the root `Cargo.toml`,
every workspace member's `Cargo.toml`, the VSCode extension manifest and
lockfile, `features.toml`, and the doc surface (`README.md`, `CLAUDE.md`,
`docs/project/ROADMAP.md`). To keep them in sync there is **one** command
that updates every site at once:

```bash
# Bump every workspace-version reference to X.Y.Z (non-interactive,
# idempotent — running it twice produces no diff).
cargo xtask bump-version 0.13.0

# Verify nothing has drifted out of sync. This is run automatically
# from `just ci-gate` via `just version-check`.
cargo xtask check-version-sync
```

The two commands share their site list (defined in
`crates/perl-ci-hygiene/src/version_sync.rs`) so the CI gate is guaranteed
to catch any drift the bump command would have repaired. Adding a new
version reference to the project? Add a collector for it in
`version_sync.rs` and both commands pick it up automatically.

Notes for releases:

- The bump command is **non-interactive**: no confirmation prompt, safe to
  call from scripts.
- The command does **not** rewrite changelog entries, release notes, blog
  posts, or PR references — historical mentions of past versions are
  immutable history, not version-bump targets.
- Crates listed in `[workspace.exclude]` (`tree-sitter-perl`, `fuzz`,
  `archive`) are tracked on their own version cadence and are skipped.
- After bumping, run `cargo build` once so `Cargo.lock` updates, then
  commit `Cargo.toml`, `Cargo.lock`, and every doc/manifest the bump
  command touched.

## Important Notes

- Workspace utility crate, not shipped in releases
- Use `cargo xtask <task>` to run tasks (not `cargo run -p xtask`)
- Gate receipts written to `target/receipts/` by default
- See `.ci/gate-policy.yaml` for gate definitions
