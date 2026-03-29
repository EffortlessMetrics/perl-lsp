# perl-lsp-diagnostics

Diagnostics and linting provider for Perl source.

## When to use this crate

Use `perl-lsp-diagnostics` when you want editor-facing diagnostics for Perl
source without embedding the full `perl-lsp` runtime.

It combines parse errors, semantic analysis, lint checks, and dead-code signals
into one provider surface suitable for `textDocument/publishDiagnostics` or
pull-diagnostics style flows.

## Public API

- `DiagnosticsProvider`: core provider that builds diagnostics from AST and parse errors.
- `Diagnostic`, `DiagnosticSeverity`, `DiagnosticTag`, `RelatedInformation`: diagnostic payload types.
- `common_mistakes`, `deprecated`, `strict_warnings`, `unused_imports`: lint families re-exported for direct use.
- `detect_dead_code`: workspace-wide dead code detection when not targeting WASM.

## Example

```rust,ignore
use perl_lsp_diagnostics::DiagnosticsProvider;

let provider = DiagnosticsProvider::new(&ast, source.to_string());
let diagnostics = provider.get_diagnostics(&workspace_index);
assert!(!diagnostics.is_empty());
```

## Workspace role

Internal feature crate consumed by `perl-lsp` to publish diagnostics to
editors. It is mostly a workspace building block rather than a standalone
end-user crate.

## License

MIT OR Apache-2.0
