# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

`perl-pragma` is a **Tier 1 leaf crate** that tracks lexical pragma state across Perl source files.

**Purpose**: Walk an AST to build a range-indexed map of effective pragma state (strict/warnings, utf8/encoding/locale, feature flags, builtin imports, and version-implied semantics), enabling scope-aware queries at any byte offset.

**Version**: workspace (currently 0.12.4)

## Commands

```bash
cargo build -p perl-pragma                    # Build this crate
cargo test -p perl-pragma                     # Run crate tests (tests/ included)
cargo check --all-targets -p perl-pragma      # Check all targets
cargo clippy -p perl-pragma                   # Lint
cargo doc -p perl-pragma --open               # View documentation
```

## Architecture

### Dependencies

- `perl-ast` -- AST node types (`Node`, `NodeKind`)

### Key Types and Functions

| Item | Description |
|------|-------------|
| `PerlVersion` | Parsed major/minor Perl version for lexical version pragmas |
| `PragmaState` | Effective lexical state: strict, warnings, utf8, encoding, locale, features, builtin imports |
| `PragmaTracker` | Stateless builder/query helper with `build()` and `state_for_offset()` |
| `parse_perl_version` | Parses `v5.36`, `5.036`, and similar forms |
| `features_enabled_by_version` | Computes feature bundles implied by `use VERSION` |

### How It Works

1. `PragmaTracker::build(ast)` recursively walks an AST `Node`.
2. `NodeKind::Use` / `NodeKind::No` update tracked pragma state for strict/warnings, utf8/encoding/locale, feature toggles, builtin imports, and version pragmas.
3. Scoped containers (for example `Block`, `Eval`, `PhaseBlock`, and `Package { block: Some(..) }`) save/restore lexical state.
4. The result is a sorted `Vec<(Range<usize>, PragmaState)>`.
5. `state_for_offset()` performs a binary search (`partition_point`) to return the effective state at any byte offset.

### Downstream Consumers

- `perl-parser-core` -- uses pragma state during parsing
- `perl-lsp-diagnostics` -- pragma-aware diagnostic reporting

## Test Surface

The crate has direct tests in `crates/perl-pragma/tests/`, including:

- `behavior_spec_tests.rs` -- BDD-style behavior scenarios
- `comprehensive_unit_tests.rs` -- broad API/unit coverage

Run with:

```bash
cargo test -p perl-pragma
```

## Usage

```rust
use perl_pragma::{PragmaTracker, features_enabled_by_version};

let pragma_map = PragmaTracker::build(&ast);
let state = PragmaTracker::state_for_offset(&pragma_map, byte_offset);

if state.utf8 && state.has_feature("unicode_strings") {
    // unicode-aware behavior is active in this lexical scope
}

let v540_features = features_enabled_by_version(perl_pragma::PerlVersion::new(5, 40));
assert!(v540_features.contains(&"builtin"));
```

## Important Notes

- Pragmas are lexically scoped; state is restored after scoped bodies.
- `use feature 'signatures'` implies effective strictness via `signatures_strict`.
- `no feature 'signatures'` unwinds the feature-implied strictness without removing explicit `use strict` effects.
- `no warnings 'category'` preserves global warnings while disabling specific categories.
- Unrecognized pragma modules in `use`/`no` are ignored by this crate.
