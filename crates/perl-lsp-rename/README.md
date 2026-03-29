# perl-lsp-rename

LSP rename provider for Perl symbol refactoring.

## When to use this crate

Use `perl-lsp-rename` when you want rename and prepare-rename behavior for Perl
symbols without taking a dependency on the full server runtime.

It is the right crate for:

- validating whether a symbol can be renamed
- generating coordinated rename edits
- preserving sigils and Perl naming rules during refactors

## Features

- **Prepare rename**: validates that a symbol at a given position is renameable
- **Rename execution**: generates text edits for all occurrences (definitions + references)
- **Name validation**: rejects empty names, keywords, invalid identifiers, and naming conflicts
- **Sigil handling**: preserves Perl sigils (`$`, `@`, `%`, `&`) during variable renames
- **Special variable protection**: prevents renaming of built-in variables and functions
- **Optional text search**: can also rename occurrences in comments and strings

## Public API

| Type | Purpose |
|------|---------|
| `RenameProvider` | Main entry point: `prepare_rename()` and `rename()` |
| `RenameOptions` | Controls validation, comment/string renaming |
| `RenameResult` | Contains edits, validity flag, and optional error |
| `TextEdit` | A single location + replacement text |

## Workspace role

Internal feature crate in the `tree-sitter-perl-rs` workspace, consumed by
`perl-lsp` to handle `textDocument/rename` and `textDocument/prepareRename` requests.
Depends on `perl-parser-core` for AST types and `perl-semantic-analyzer` for symbol tables.

## License

MIT OR Apache-2.0
