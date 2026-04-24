# perl-pragma Roadmap

> **Note:** This is the component-specific roadmap for `perl-pragma`. For the project-wide roadmap, see [`docs/project/ROADMAP.md`](../../docs/project/ROADMAP.md).

## Purpose
Perl pragma extraction and lexical-state analysis primitives.

## Current Status (workspace version)
- **Status:** Initial Public Alpha
- **Integration:** Part of the `perl-lsp` workspace.

### Shipped
- Range-indexed pragma state map (`PragmaTracker::build` + `state_for_offset`).
- Lexical tracking for `strict`, `warnings`, `utf8`, `encoding`, and `locale`.
- Version-aware semantics (`use vX.Y`) including implied strict/warnings and
  feature bundles.
- `feature` enable/disable handling (including bundle forms such as `:5.36` and
  `:all`).
- Lexical `builtin` import tracking.
- Scoped restoration across block/eval/package/phase-style lexical containers.
- Dedicated crate test surface in `crates/perl-pragma/tests/`.

### Still Needs Hardening
- Broaden edge-case coverage for unusual pragma argument forms and mixed quoting.
- Add more regression tests around nested conditional pragmas (`use/no if|unless`).
- Continue downstream validation against real-world corpus files with dense pragma
  stacking.
- Tighten API docs with consumer-oriented examples for feature/builtin/version
  interactions.

## v0.15.0 Stability Contract
- Lock down public API for semantic versioning.
- Guarantee stability across supported platforms.

## Internal Dependencies
- Aligns with project-wide capability goals defined in `features.toml`.

<!-- Last Updated: 2026-04-24 -->
