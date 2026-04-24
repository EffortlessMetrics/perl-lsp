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
- Keep `perl-token` as a std-only leaf crate (no runtime deps without explicit allowlisting).
- Require TokenKind metadata + docs + conformance test updates for new variants.
- TokenKind variants: 132

## Internal Dependencies
- Aligns with project-wide capability goals defined in `features.toml`.

<!-- Last Updated: 2026-02-28 -->
