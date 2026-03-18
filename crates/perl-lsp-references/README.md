# perl-lsp-references

Standalone SRP microcrate for same-file Perl LSP reference lookup.

## Responsibilities

- Resolve the symbol at a byte offset within a parsed Perl AST.
- Find same-file references for variables and subroutines.
- Keep reference matching logic separate from broader navigation features.

## API

- `find_references_single_file` — returns byte-offset ranges for same-file variable and subroutine references.
