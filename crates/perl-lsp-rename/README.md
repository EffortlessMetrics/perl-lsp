# perl-lsp-rename

Rename refactoring provider for Perl LSP.

## Scope

- Validates rename requests and new symbol names.
- Resolves rename targets and cross-reference edit ranges.
- Produces LSP-compatible text edits for single-file and workspace scenarios.

## Public Surface

- `RenameProvider`.
- `RenameOptions`, `RenameResult`.
- `TextEdit`.

## Workspace Role

Internal feature crate consumed by `perl-lsp` rename request handling.

## License

MIT OR Apache-2.0.
