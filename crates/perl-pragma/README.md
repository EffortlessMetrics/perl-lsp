# perl-pragma

Pragma state tracking for Perl source analysis.

## Overview

`perl-pragma` walks a `perl-ast` AST to track `use strict`, `no strict`,
`use warnings`, and `no warnings` statements. It builds a range-indexed
pragma map so callers can query the effective pragma state at any byte offset
in the source.

## Public API

- **`PragmaState`** -- tracks `strict_vars`, `strict_subs`, `strict_refs`,
  `warnings`, and tracked feature/builtin state. Provides helper query methods.
- **`PragmaEnvironment`** -- immutable compile-time environment with
  `query(PragmaQuery)` / `snapshot_at(offset)` APIs for position-based state
  lookup.
- **`PragmaSnapshot`** -- immutable per-position snapshot exposing strict,
  warnings, and feature checks for downstream diagnostics/semantic consumers.
- **`PragmaTracker`** -- walks an AST via `build()` to produce a sorted
  `Vec<(Range<usize>, PragmaState)>` for compatibility with existing callers.

## Workspace Role

Tier 1 leaf crate. Depends only on `perl-ast`. Consumed by
`perl-parser-core` and `perl-lsp-diagnostics` to provide scope-aware
pragma analysis for parsing and diagnostic flows.

## License

MIT OR Apache-2.0
