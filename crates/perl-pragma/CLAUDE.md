# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

`perl-pragma` is a **Tier 1 leaf crate** that tracks lexical pragma state across Perl source files.

**Purpose**: Walk an AST and build a range-indexed map of pragma effects (strict/warnings, utf8,
encoding, locale, feature bundles, version semantics, and builtin imports), enabling scope-aware
queries at any byte offset.

**Version**: workspace (currently 0.12.3)

## Commands

```bash
cargo build -p perl-pragma           # Build this crate
cargo test -p perl-pragma            # Run tests
cargo check --all-targets -p perl-pragma  # Check lib + tests + benches/examples targets
cargo clippy -p perl-pragma          # Lint
cargo doc -p perl-pragma --open      # View documentation
```

## Architecture

### Dependencies

- `perl-ast` -- AST node types (`Node`, `NodeKind`)

### Key Types & Functions

| Item | Description |
|------|-------------|
| `PerlVersion` | Parsed major/minor version model for `use vX.Y` semantics |
| `PragmaState` | Effective lexical state: strict flags, warnings state/categories, utf8/encoding/locale, features, builtin imports |
| `PragmaTracker` | Stateless builder/query API: `build()` and `state_for_offset()` |
| `parse_perl_version` | Parses lexical version declarations |
| `version_implies_strict` / `version_implies_warnings` | Encodes version-driven pragma implications |
| `features_enabled_by_version` | Maps version bundles to feature sets |

### How It Works

1. `PragmaTracker::build(ast)` recursively walks an AST `Node`.
2. `NodeKind::Use` / `NodeKind::No` update running `PragmaState` for tracked pragma modules.
3. Scoped nodes save/restore lexical state (blocks, eval blocks, block packages, phase blocks, and other scoped bodies).
4. The result is a sorted `Vec<(Range<usize>, PragmaState)>` keyed by source ranges.
5. `state_for_offset()` performs a binary search (`partition_point`) to return the effective state at any byte offset.

### Downstream Consumers

- `perl-parser-core` -- uses pragma state during parsing
- `perl-lsp-diagnostics` -- pragma-aware diagnostic reporting

## Usage

```rust
use perl_pragma::{PragmaTracker};

let pragma_map = PragmaTracker::build(&ast);
let state = PragmaTracker::state_for_offset(&pragma_map, byte_offset);

if state.has_feature("signatures") {
    // signatures feature is lexically active
}

if state.has_builtin_import("true") {
    // `use builtin 'true'` (or qw(...)) is active in scope
}
```

## Test Surface

This crate has dedicated tests under `crates/perl-pragma/tests/`:

- `behavior_spec_tests.rs` -- consumer-facing BDD behavior scenarios
- `comprehensive_unit_tests.rs` -- broader API and edge-case coverage

## Important Notes

- Pragmas are lexical; scoped bodies restore caller state on exit.
- `use VERSION` can imply strict/warnings and feature bundles.
- `no warnings 'category'` only disables selected categories; bare `no warnings` disables all warnings.
- `use/no feature` updates lexical features; signatures strictness is tracked separately and applied at query time.
- Unrecognized `use`/`no` modules are ignored unless they parse as a version pragma.
