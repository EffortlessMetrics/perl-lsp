# perl-pragma

Pragma state tracking for Perl source analysis.

## Overview

`perl-pragma` walks a `perl-ast` AST and builds a range-indexed pragma map so
callers can query effective lexical pragma state at any byte offset.

The tracker currently models:

- strict/warnings toggles and warning-category suppression
- `utf8`, `encoding`, and `locale` pragmas
- Perl version pragmas (`use v5.xx`) including implied strict/warnings behavior
- `feature` pragma controls, including feature bundles (`:5.xx`, `:all`) and
  feature-level effects such as `signatures`
- `builtin` lexical imports (`use builtin ...`)

Scoping follows Perl lexical behavior for ordinary blocks and scoped constructs,
including nested blocks, phase blocks, `eval { ... }`, and braced package
blocks.

## Public API

- **`PragmaState`** -- snapshot of effective pragma state, including strict
  flags, warnings flags/categories, utf8/encoding/locale state, active
  features, and imported builtins. Helpers include `all_strict()`,
  `is_warning_active()`, `has_feature()`, and `has_builtin_import()`.
- **`PragmaTracker`** -- builds `Vec<(Range<usize>, PragmaState)>` via `build()`
  and resolves state at an offset via `state_for_offset()`.
- **Version helpers** -- `PerlVersion`, `parse_perl_version()`,
  `version_implies_strict()`, `version_implies_warnings()`, and
  `features_enabled_by_version()`.

## Workspace Role

Tier 1 leaf crate. Depends only on `perl-ast`. Consumed by
`perl-parser-core` and `perl-lsp-diagnostics` for scope-aware pragma analysis.

## License

MIT OR Apache-2.0
