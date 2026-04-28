# perl-diagnostics

Unified diagnostic codes, types, and catalog for Perl LSP.

This crate consolidates three previously separate diagnostic crates into a
single, coherent API:

| Former crate | Now |
|---|---|
| `perl-diagnostics-codes` | `perl_diagnostics::codes` |
| `perl-lsp-diagnostic-types` | `perl_diagnostics::types` |
| `perl-lsp-diagnostic-catalog` | `perl_diagnostics::catalog` |

## Modules

- **`codes`** — canonical `DiagnosticCode`, `DiagnosticCategory`,
  `DiagnosticSeverity`, and `DiagnosticTag` enums. All other modules derive
  their severity/tag types from here; there is exactly one definition in the
  workspace.

- **`types`** — `Diagnostic` and `RelatedInformation` structs. `DiagnosticSeverity`
  and `DiagnosticTag` are re-exported from `codes` so the legacy
  `types::DiagnosticSeverity` import path still resolves to the same type.

- **`catalog`** — LSP-facing metadata helpers that map a `DiagnosticCode` to its
  human-readable message, related documentation URL, and default severity.

All public items are additionally re-exported from the crate root via the
`api` module so consumers need only `use perl_diagnostics::*`.

## Usage

```toml
[dependencies]
perl-diagnostics = { path = "../../crates/perl-diagnostics" }
# Enable serde support for JSON serialization:
# perl-diagnostics = { path = "...", features = ["serde"] }
```

```rust
use perl_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};

let diag = Diagnostic {
    code: DiagnosticCode::MissingStrict,
    severity: DiagnosticSeverity::Warning,
    message: "Missing 'use strict'".into(),
    ..Default::default()
};
```

## Type Unification

`DiagnosticSeverity` and `DiagnosticTag` are defined once in `codes` and
re-exported through `types`. This means `types::DiagnosticSeverity` and
`codes::DiagnosticSeverity` are the same type — no orphan-impl issues, no
`From` conversion needed when passing values between modules.

## Features

| Feature | Default | Effect |
|---|---|---|
| `serde` | off | Derives `Serialize`/`Deserialize` on all public types |
