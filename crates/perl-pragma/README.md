# perl-pragma

Pragma state tracking for Perl source analysis.

## Overview

`perl-pragma` walks a `perl-ast` AST to track lexical `use`/`no` effects and
builds a range-indexed pragma map so callers can query effective state at any
byte offset in a file.

The tracked pragma surface includes:

- `strict` and `warnings` (including per-category warning disables)
- `utf8`
- `encoding`
- `locale` (including optional locale scope arguments)
- `feature` toggles and feature bundles (`:5.xx`, `:all`, individual features)
- lexical `builtin` imports
- version pragmas (`use vX.Y` / `use 5.xxx`) that imply strictness, warnings,
  and feature bundles
- conditional forms (`use if`, `use unless`, `no if`, `no unless`) when they
  target pragma-like modules

Lexical scoping is modeled across ordinary blocks and scoped constructs such as
`eval { ... }`, package block forms (`package Foo { ... }`), and phase blocks
(`BEGIN`, `END`, `CHECK`, `INIT`, `UNITCHECK`) so inner pragma changes are
restored when scope exits.

## Public API

- **`PragmaState`** -- captures strict/warnings flags plus UTF-8, encoding,
  locale, feature set, and builtin imports for the active lexical scope.
  Includes helpers like `all_strict()`, `is_warning_active()`,
  `has_feature()`, and `has_builtin_import()`.
- **`PragmaTracker`** -- walks an AST via `build()` to produce a sorted
  `Vec<(Range<usize>, PragmaState)>`, and offers `state_for_offset()` to query
  effective state at arbitrary byte offsets.
- **Version/feature helpers** -- `PerlVersion`, `parse_perl_version()`,
  `version_implies_strict()`, `version_implies_warnings()`, and
  `features_enabled_by_version()`.

## Workspace Role

Tier 1 leaf crate. Depends only on `perl-ast`. Consumed by parser and language
feature crates that need scope-aware pragma state when analyzing Perl source.

## License

MIT OR Apache-2.0
