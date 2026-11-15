# CLAUDE.md (perl-tidy)

## Role

Canonical Rust-native Perl source formatter.

## Owns

- Parse-gated native formatting and range formatting.
- Native formatter configuration, layout IR, diagnostics, and edits.
- Preservation/refusal rules for Perl surfaces that cannot yet be rewritten safely.

## Must not own

- LSP request/response types or live-document lifecycle.
- Subprocess execution or discovery of `perl` / `perltidy`.
- Filesystem writes or metadata-preserving replacement.

## Focused validation

`cargo test -p perl-tidy --all-targets`
