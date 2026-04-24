# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-pragma` is a **Tier 1 leaf crate** that tracks pragma state across Perl source files.

**Purpose**: Walks an AST to build a range-indexed map of lexical pragma effects (`use`/`no`) so downstream consumers can query effective state at any byte offset.

**Version**: workspace (currently 0.12.3)

## Commands

```bash
cargo build -p perl-pragma                    # Build this crate
cargo test -p perl-pragma                     # Run crate tests
cargo check --all-targets -p perl-pragma      # Check all targets for this crate
cargo clippy -p perl-pragma                   # Lint
cargo doc -p perl-pragma --open               # View documentation
```

## Architecture

### Dependencies

- `perl-ast` -- AST node types (`Node`, `NodeKind`)

### Key Types

| Type | Description |
|------|-------------|
| `PragmaState` | Strict/warnings flags plus `utf8`, `encoding`, `locale`, warning-category suppression, enabled features, and builtin imports |
| `PerlVersion` | Parsed Perl version used by version-implied semantics |
| `PragmaTracker` | Builder/query surface via `build()` and `state_for_offset()` |

### Core Behavior

1. `PragmaTracker::build(ast)` recursively walks an AST `Node`.
2. `NodeKind::Use` / `NodeKind::No` apply lexical effects for `strict`, `warnings`, `utf8`, `encoding`, `locale`, `feature`, `builtin`, and version pragmas.
3. Conditional pragma wrappers (`use if`, `use unless`, `no if`, `no unless`) are interpreted when they target pragma-like modules.
4. Block-like forms (`Block`, `Eval` block form, `PhaseBlock`, package block form, and other scoped containers) save/restore `PragmaState` to model lexical scope.
5. The result is a sorted `Vec<(Range<usize>, PragmaState)>`; `state_for_offset()` uses binary search (`partition_point`) to return the effective state.

### Downstream Consumers

- `perl-parser-core` -- uses pragma state during parsing
- `perl-lsp-diagnostics` -- pragma-aware diagnostic reporting

## Usage

```rust
use perl_pragma::{PragmaState, PragmaTracker};

let pragma_map = PragmaTracker::build(&ast);
let state = PragmaTracker::state_for_offset(&pragma_map, byte_offset);

if state.utf8 || state.has_feature("unicode_strings") {
    // unicode-sensitive analysis path
}

if state.has_builtin_import("true") {
    // builtin::true is available in this lexical scope
}
```

## Tests

This crate has a dedicated test surface in `crates/perl-pragma/tests`:

- `comprehensive_unit_tests.rs` -- broad API/state transition coverage
- `behavior_spec_tests.rs` -- scenario-style lexical behavior coverage

Run with:

```bash
cargo test -p perl-pragma
```

## Important Notes

- Pragmas are lexically scoped; scope-exit restores outer state.
- `use vX.Y` and `use 5.xxx` can imply strictness, warnings, and feature bundles.
- `no warnings 'CATEGORY'` records category disables while keeping global warnings active.
- `use feature 'signatures'` toggles strictness implication via dedicated state tracking.
- Unknown modules in `use`/`no` are ignored unless recognized as version pragmas.
