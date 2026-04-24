# perl-pragma

Pragma state tracking for Perl source analysis.

## Overview

`perl-pragma` walks a `perl-ast` AST to track lexical `use`/`no` effects and
builds a range-indexed pragma map so callers can query the effective state at
any byte offset in a file.

The crate tracks more than the classic strict/warnings pair. Current surface
includes:

- `strict` / `no strict` (full or category-specific)
- `warnings` / `no warnings` (including category-level disables)
- `utf8`
- `encoding`
- `locale`
- Perl version declarations (`use vX.Y`) and their implied strict/warnings rules
- `feature` declarations, including version bundles like `:5.36`
- `builtin` lexical imports
- Conditional forms via `use if` / `use unless` / `no if` / `no unless`

## Public API

- **`PerlVersion`** -- parsed major/minor version model for `use v...` handling.
- **`PragmaState`** -- effective lexical state including strict/warnings,
  unicode/locale/encoding state, enabled feature set, and imported builtins.
- **`PragmaTracker`** -- walks an AST via `build()` to produce a sorted
  `Vec<(Range<usize>, PragmaState)>`, and offers `state_for_offset()` to query
  effective state.
- **Version helpers** -- `parse_perl_version`, `version_implies_strict`,
  `version_implies_warnings`, and `features_enabled_by_version`.

## Scoping Model

Pragmas are tracked lexically and restored when leaving scoped constructs. This
includes regular blocks plus scoped containers such as:

- braced blocks (`{ ... }`)
- `eval { ... }`
- block `package Foo { ... }`
- phase blocks (`BEGIN`, `END`, `CHECK`, `INIT`, `UNITCHECK`)

`state_for_offset()` always returns the effective state at the queried byte
position after lexical restoration rules are applied.

## Workspace Role

Tier 1 leaf crate. Depends only on `perl-ast`. Consumed by
`perl-parser-core` and `perl-lsp-diagnostics` to provide scope-aware pragma
analysis for parsing and diagnostic flows.

## License

MIT OR Apache-2.0
