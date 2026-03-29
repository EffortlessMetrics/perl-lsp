# perl-lsp-code-lens

Perl code lens provider for inline actions above code. It is responsible for
test-run lenses, reference-count lenses, and script-level actions such as
running a shebang file.

## Use this crate when

Use `perl-lsp-code-lens` if you need the code-lens logic itself. It is the
layer between parsed Perl code and the editor commands that sit above it.

## Key exports

- `CodeLensProvider` - extracts lenses from an AST and optional file path
- `CodeLens` / `Command` - serialized lens payloads
- `resolve_code_lens` - turns reference-count data into a runnable command
- `get_shebang_lens` - adds a top-of-file "Run Script" action
- `is_test_file` - `.t` file detection for test-specific lenses

## Example

```rust,ignore
use perl_lsp_code_lens::CodeLensProvider;

let provider = CodeLensProvider::new(source.to_string())
    .with_file_path("t/basic.t".to_string());
let lenses = provider.extract(&ast);
```

## Stack role

`perl-lsp` uses this crate to surface inline run and reference actions in the
editor. It sits on top of parser output and file context.
