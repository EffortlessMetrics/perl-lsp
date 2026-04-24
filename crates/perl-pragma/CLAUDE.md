# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

`perl-pragma` is a **Tier 1 leaf crate** that tracks effective pragma state
across Perl source files.

**Purpose**: Walk a `perl-ast` tree and produce a range-indexed map of lexical
pragma effects, including strict/warnings controls, utf8/encoding/locale,
feature bundles, builtin imports, and version-implied semantics.

**Version**: Workspace-managed (`version.workspace = true`).

## Commands

```bash
cargo build -p perl-pragma
cargo test -p perl-pragma
cargo check --all-targets -p perl-pragma
cargo clippy -p perl-pragma
cargo doc -p perl-pragma --open
```

## Architecture

### Dependencies

- `perl-ast` -- AST node types (`Node`, `NodeKind`)

### Key Types and Functions

| Item | Description |
|------|-------------|
| `PragmaState` | Effective lexical state: strict flags, warnings + disabled categories, `utf8`, `encoding`, locale state, active features, builtin imports |
| `PragmaTracker` | Stateless entry points: `build()` and `state_for_offset()` |
| `PerlVersion` | Parsed Perl version pair used by version pragmas |
| `parse_perl_version()` | Parses `v5.xx` / `5.xxx` declarations |
| `features_enabled_by_version()` | Returns feature bundle implied by a version pragma |

### Lexical Scoping Model

`PragmaTracker::build_ranges` applies pragma transitions and restores caller
state at lexical boundaries. Scoped handling includes:

- `Block`
- `PhaseBlock` (`BEGIN`, `END`, `INIT`, `CHECK`, `UNITCHECK`)
- `Eval` blocks (`eval { ... }`)
- braced package blocks (`package Foo { ... }`)
- other scoped bodies (`sub`, `method`, `class`, loop bodies, `try` branches)

`state_for_offset()` returns the effective state at an offset by selecting the
latest preceding range and applying any derived strictness (for example,
`feature 'signatures'` implications).

## Tests

This crate has first-class integration-style tests under `tests/`:

- `tests/comprehensive_unit_tests.rs` -- broad API and edge-case coverage
- `tests/behavior_spec_tests.rs` -- BDD-style scenario coverage for lexical
  behavior and pragma interactions

When changing behavior, update/add tests in this crate rather than relying only
on downstream integration coverage.

## Downstream Consumers

- `perl-parser-core` -- uses pragma state during parsing
- `perl-lsp-diagnostics` -- pragma-aware diagnostic reporting
