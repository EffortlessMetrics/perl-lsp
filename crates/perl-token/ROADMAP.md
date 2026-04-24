# perl-token Roadmap

> **Note:** This is the component-specific roadmap for `perl-token`. For the project-wide roadmap, see [`docs/project/ROADMAP.md`](../../docs/project/ROADMAP.md).

## Purpose
Token definitions for Perl parser

## Current Status (workspace version)
- **Status:** Initial Public Alpha
- **Integration:** Part of the `perl-lsp` workspace.

## Future Milestones

### Hardening
- Address early adopter feedback.
- Refine API contracts and error handling.
- Improve test coverage and documentation.

### v0.15.0 Stability Contract
- Lock down public API for semantic versioning.
- Guarantee stability across supported platforms.

### Leaf-Crate Stability Guardrails
- Keep runtime dependencies at zero (std-only) unless explicitly approved.
- Keep `Token` / `TokenKind` API source-compatible by default; treat API snapshot diffs as intentional-change events.
- Keep metadata, docs, and conformance tests synchronized for every TokenKind variant.
- TokenKind variant count: 132

## Internal Dependencies
- Aligns with project-wide capability goals defined in `features.toml`.

<!-- Last Updated: 2026-02-28 -->
