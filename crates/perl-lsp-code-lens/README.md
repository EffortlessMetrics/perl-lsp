# perl-lsp-code-lens

Inline code-lens extraction for Perl files in `perl-lsp`.

## Problem it solves

LSP clients can show clickable actions above code, but they need a provider that
knows where those actions belong. This crate finds Perl-specific code-lens
targets such as test subroutines, subtests, shebang scripts, packages, and
reference-countable declarations.

## Public API

- `CodeLensProvider` extracts lenses from a parsed Perl AST.
- `resolve_code_lens` fills in reference-count titles after lookup.
- `get_shebang_lens` creates a run-script lens for executable Perl files.
- `is_test_file` detects `.t` files for test-specific actions.

## Example

```rust,ignore
use perl_lsp_code_lens::CodeLensProvider;

let lenses = CodeLensProvider::new(source.to_string())
    .with_file_path("t/basic.t".to_string())
    .extract(&ast);
```

## Workspace role

`perl-lsp` uses this crate to power `textDocument/codeLens` and related resolve
flows without keeping test-lens logic in the main server crate.

## License

MIT OR Apache-2.0
