# CLAUDE.md (perl-lsp-perltidy)

## Role

Temporary source-compatibility facade for the formatter crate split.

## Owns

- Historical public re-export paths only.
- Compatibility tests proving old imports continue to compile and behave.

## Must not own

- Native formatter implementation.
- External process implementation.
- New production call sites.

## Focused validation

`cargo test -p perl-lsp-perltidy --all-targets`
