# perl-module-token-parser

Single-line Perl module token parser used by cursor-aware reference and import workflows.

## Scope

- Parse a Perl module-style token at a byte offset (supporting `::` and `'` separators)
- Return stable byte spans suitable for downstream cursor/range operations
- Enforce boundary-safe token boundaries (no partial identifiers)

## API

- `parse_module_token(text, start)`
- `ModuleTokenSpan`
