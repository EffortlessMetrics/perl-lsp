# perl-lsp-rename

Perl rename provider for `textDocument/prepareRename` and `textDocument/rename`.
It validates a symbol first, then produces the text edits needed to rename it
consistently across definitions and references.

## Use this crate when

Use `perl-lsp-rename` if you need rename logic with Perl-specific rules. It is
the right layer when you need validation, scope-aware symbol resolution, and
edit generation. If you only need the shared provider surface, use
`perl-lsp-providers`.

## Key exports

- `RenameProvider` - main entry point for `prepare_rename()` and `rename()`
- `RenameOptions` - controls validation and whether comments/strings are
  included
- `RenameResult` - edits plus validation state
- `TextEdit` - single replacement at a location

## Example

```rust,ignore
use perl_lsp_rename::{RenameOptions, RenameProvider};

let provider = RenameProvider::new(&ast, source.to_string());
let _prepared = provider.prepare_rename(position);
let result = provider.rename(position, "new_name", &RenameOptions::default());
```

## Stack role

This is the rename engine consumed by `perl-lsp`. It relies on parser, scope,
and symbol data, then returns the precise edits that the editor applies.
