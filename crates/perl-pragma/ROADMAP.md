# perl-pragma Roadmap

> **Note:** This is the component-specific roadmap for `perl-pragma`. For the project-wide roadmap, see [`docs/project/ROADMAP.md`](../../docs/project/ROADMAP.md).

## Purpose
Perl pragma extraction and lexical state analysis primitives.

## Current Status (workspace version)
- **Status:** Initial Public Alpha
- **Integration:** Part of the `perl-lsp` workspace.

## Shipped Surface (implemented)
- Tracks strict/warnings plus warning-category disable lists.
- Tracks `utf8`, `encoding`, and `locale` pragma state.
- Tracks version pragmas (`use vX.Y`, `use 5.xxx`) and implied strict/warnings semantics.
- Tracks feature bundles and explicit feature toggles (`use feature`, `no feature`, `:VERSION`, `:all`).
- Tracks lexical `use builtin` imports.
- Restores lexical pragma state across scoped bodies (`{}` blocks, eval blocks, package block form, phase blocks, and other block-like AST containers).
- Includes dedicated crate tests under `tests/` that cover behavior and broader unit scenarios.

## Hardening Backlog
- Increase edge-case coverage for parser argument normalization in conditional pragmas (`use if`/`use unless` and `no if`/`no unless`).
- Add explicit regression tests for mixed pragma interactions in deeply nested scoped constructs.
- Continue tightening API/docs around version-implied feature transitions as newer Perl feature bundles are added upstream.
- Collect downstream feedback to confirm `PragmaState` field stability before a semver stability commitment.

## v0.15.0 Stability Contract
- Lock down public API for semantic versioning.
- Guarantee stability across supported platforms.

## Internal Dependencies
- Aligns with project-wide capability goals defined in `features.toml`.

<!-- Last Updated: 2026-04-24 -->
