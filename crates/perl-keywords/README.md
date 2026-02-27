# perl-keywords

Canonical Perl keyword inventories and allocation-free lookup helpers.

## Scope

- Canonical sorted keyword list (`KEYWORDS`)
- Sorted callsite-specific keyword buckets used by LSP, DAP, and lexer paths
- Binary-search helpers for keyword classification

## API

- `KEYWORDS`
- `is_keyword(token)`
- `LSP_COMPLETION_KEYWORDS`
- `DAP_COMPLETION_KEYWORDS`
- `LSP_RUNTIME_COMPLETION_KEYWORDS`
- `RENAME_KEYWORDS`
- `PARSER_LSP_KEYWORDS`
- `LEXER_KEYWORDS`
