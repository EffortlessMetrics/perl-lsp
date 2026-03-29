# perl-lsp-diagnostics

Perl diagnostics engine for editor-facing parse errors, lint checks, semantic
warnings, and dead-code reporting. This crate turns parser and analysis results
into `publishDiagnostics`-style output.

## Use this crate when

Use `perl-lsp-diagnostics` if you need the diagnostic logic itself. If you only
need the shared diagnostic types, depend on the re-exported types rather than
rebuilding the provider stack. If you want the umbrella API, use
`perl-lsp-providers`.

## Key exports

- `DiagnosticsProvider` - assembles diagnostics from AST, source, and optional
  workspace context
- `Diagnostic`, `DiagnosticSeverity`, `DiagnosticTag`, `RelatedInformation` -
  shared diagnostic types
- `common_mistakes`, `deprecated`, `missing_module`, `package_subroutine`,
  `security`, `strict_warnings`, `unused_imports`, `version_compat` - lint
  families re-exported for targeted checks
- `detect_dead_code` - workspace-wide dead-code detection outside WASM builds

## Example

```rust,ignore
use perl_lsp_diagnostics::DiagnosticsProvider;

let provider = DiagnosticsProvider::new();
let diagnostics = provider.generate_diagnostics(&ast, source, Some(&workspace_index))?;
```

## Stack role

This crate sits between parsing and the editor runtime. It consumes ASTs,
scope analysis, and workspace data, then emits the diagnostics that `perl-lsp`
publishes to clients.
