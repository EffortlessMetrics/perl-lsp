# perl-lsp-providers

Umbrella re-export crate for the Perl LSP provider stack. Use it when you want
one dependency surface for completion, diagnostics, navigation, rename, code
actions, formatting, and IDE compatibility shims.

## Use this crate when

Use `perl-lsp-providers` if your application wants the whole provider surface
in one place. If you only need one feature family, depend on the dedicated
crate instead so the boundary stays explicit.

## Key exports

- `completion`, `diagnostics`, `navigation`, `rename`, `code_actions` - primary
  provider families
- `formatting`, `semantic_tokens`, `inlay_hints`, `folding` - additional LSP
  feature surfaces
- `tooling` - perltidy / perlcritic integrations
- `ide` - LSP and DAP compatibility shims
- `Node`, `NodeKind`, `SourceLocation`, `Parser`, `ast`, `position` - parser
  core re-exports for convenience

## Example

```rust,ignore
use perl_lsp_providers::{completion::CompletionProvider, diagnostics::DiagnosticsProvider};

let completion = CompletionProvider::new(&ast);
let diagnostics = DiagnosticsProvider::new();
```

## Stack role

This crate is the convenience integration layer for the workspace. It keeps the
individual feature crates available under one root without hiding their real
boundaries.
