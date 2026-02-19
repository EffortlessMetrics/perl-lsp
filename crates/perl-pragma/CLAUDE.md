# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-pragma` is a **Tier 1 leaf crate** that tracks pragma state across Perl source files.

**Purpose**: Walks an AST to build a range-indexed map of `use strict`, `no strict`, `use warnings`, and `no warnings` effects, enabling scope-aware pragma queries at any byte offset.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-pragma           # Build this crate
cargo test -p perl-pragma            # Run tests
cargo clippy -p perl-pragma          # Lint
cargo doc -p perl-pragma --open      # View documentation
```

## Architecture

### Dependencies

- `perl-ast` -- AST node types (`Node`, `NodeKind`)

### Key Types

| Type | Description |
|------|-------------|
| `PragmaState` | Boolean flags: `strict_vars`, `strict_subs`, `strict_refs`, `warnings` |
| `PragmaTracker` | Stateless struct with `build()` and `state_for_offset()` methods |

### How It Works

1. `PragmaTracker::build(ast)` recursively walks an AST `Node`.
2. `NodeKind::Use { module: "strict" | "warnings", .. }` and `NodeKind::No { .. }` toggle flags on a running `PragmaState`.
3. `NodeKind::Block` saves/restores state to model lexical scoping.
4. The result is a sorted `Vec<(Range<usize>, PragmaState)>`.
5. `state_for_offset()` performs a binary search (`partition_point`) to return the effective state at any byte offset.

### Downstream Consumers

- `perl-parser-core` -- uses pragma state during parsing
- `perl-lsp-diagnostics` -- pragma-aware diagnostic reporting

## Usage

```rust
use perl_pragma::{PragmaState, PragmaTracker};

let pragma_map = PragmaTracker::build(&ast);
let state = PragmaTracker::state_for_offset(&pragma_map, byte_offset);
if state.strict_vars {
    // strict vars is in effect at this offset
}
```

## Important Notes

- Pragmas are lexically scoped; `Block` nodes save/restore state
- `use strict` with no args enables all three categories; with args only the named ones
- `no strict` / `no warnings` disable the corresponding flags
- Unrecognized modules in `use`/`no` are silently ignored
- No tests directory exists yet; the crate is exercised through downstream integration tests
