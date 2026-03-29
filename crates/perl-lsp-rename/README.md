# perl-lsp-rename

Rename provider for Perl refactoring.

## When to use this crate

Use `perl-lsp-rename` when you want rename and prepare-rename behavior for Perl
symbols without depending on the full server runtime.

It is the right crate for:

- validating whether a symbol can be renamed
- generating coordinated rename edits
- preserving sigils and Perl naming rules during refactors

## Public API

- `RenameProvider`: main entry point for `prepare_rename()` and `rename()`.
- `RenameOptions`: controls validation and comment/string renaming.
- `RenameResult`: contains edits, validity state, and error information.
- `TextEdit`: a single location plus replacement text.

## Example

```rust,ignore
use perl_lsp_rename::RenameProvider;

let provider = RenameProvider::new(&ast, source.to_string());
let prep = provider.prepare_rename(position)?;
let result = provider.rename(position, "new_name", &options);
assert!(prep.is_some());
assert!(result.valid);
```

## Workspace role

Internal feature crate consumed by `perl-lsp` for rename request handling.
It is mostly a workspace building block rather than a standalone end-user crate.

## License

MIT OR Apache-2.0
