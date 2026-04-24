# perl-pragma Roadmap

> **Note:** This is the component-specific roadmap for `perl-pragma`. For the project-wide roadmap, see [`docs/project/ROADMAP.md`](../../docs/project/ROADMAP.md).

## Purpose
Perl pragma extraction and lexical-state analysis primitives.

## Current Status (workspace version)
- **Status:** Initial Public Alpha
- **Integration:** Part of the `perl-lsp` workspace.

### Shipped in current surface
- Lexical tracking for `strict`, `warnings`, `utf8`, `encoding`, `locale`,
  `feature`, `builtin`, and version pragmas.
- Category-aware warning suppression (`no warnings 'category'`) with
  query-time checks.
- Feature bundle support (including `:5.xx` bundles and version-implied
  features) plus signatures-related strictness behavior.
- Scope restoration across block-like forms, including lexical `eval { ... }`,
  package block form (`package Name { ... }`), and phase blocks.
- Dedicated crate test suite under `crates/perl-pragma/tests` covering
  comprehensive unit and behavior-spec scenarios.

## Hardening still needed
- Expand conformance coverage for pragma argument edge cases found in real-world
  CPAN corpora.
- Add more adversarial tests around nested conditional pragma forms
  (`use if` / `no if`) and mixed pragma interactions in deep scope trees.
- Continue API contract review before the next stability ratchet.

## v0.15.0 Stability Contract
- Lock down public API for semantic versioning.
- Guarantee stability across supported platforms.

## Internal Dependencies
- Aligns with project-wide capability goals defined in `features.toml`.

<!-- Last Updated: 2026-04-24 -->
