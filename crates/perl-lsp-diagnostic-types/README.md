# perl-lsp-diagnostic-types

Shared diagnostic domain types for the Perl LSP ecosystem.

## Responsibility

This crate has a single responsibility:

- define common diagnostic value types (`Diagnostic`, `DiagnosticSeverity`, `DiagnosticTag`, `RelatedInformation`)
- provide stable deduplication behavior for diagnostic vectors

## Usage

```rust
use perl_lsp_diagnostic_types::{deduplicate_diagnostics, Diagnostic, DiagnosticSeverity};

let mut diagnostics = vec![Diagnostic {
    range: (0, 1),
    severity: DiagnosticSeverity::Warning,
    code: Some("example".to_string()),
    message: "Example warning".to_string(),
    related_information: Vec::new(),
    tags: Vec::new(),
}];

deduplicate_diagnostics(&mut diagnostics);
```
