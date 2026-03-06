# perl-lsp-critic-parser

Standalone SRP microcrate for parsing `perlcritic --verbose` output.

## Responsibility

- Parse newline-delimited `perlcritic` records into structured fields
- Handle policy names containing `::`
- Preserve message text (including additional `:` characters)
- Tolerate platform-specific file paths (including Windows drive letters)
