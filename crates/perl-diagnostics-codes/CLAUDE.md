# CLAUDE.md — perl-diagnostics-codes

## Crate Overview

- **Tier**: 1 (leaf crate, no internal workspace dependencies)
- **Version**: 0.9.1
- **Purpose**: Defines stable diagnostic codes, severity levels, tags, and categories for the Perl LSP ecosystem. All codes have fixed string representations that persist across versions.

## Commands

```bash
cargo build -p perl-diagnostics-codes        # Build
cargo test -p perl-diagnostics-codes         # Run tests
cargo clippy -p perl-diagnostics-codes       # Lint
cargo doc -p perl-diagnostics-codes --open   # View docs
```

## Architecture

### Dependencies

- `serde` (optional, behind `serde` feature) — derive `Serialize`/`Deserialize` on all public types

### Key Types

| Type | Purpose |
|------|---------|
| `DiagnosticCode` | Enum of all diagnostic codes (e.g., `ParseError`, `MissingStrict`, `CriticSeverity1`) |
| `DiagnosticSeverity` | `Error`, `Warning`, `Information`, `Hint` — maps to LSP values 1-4 via `to_lsp_value()` |
| `DiagnosticTag` | `Unnecessary`, `Deprecated` — maps to LSP tag values via `to_lsp_value()` |
| `DiagnosticCategory` | `Parser`, `StrictWarnings`, `PackageModule`, `Subroutine`, `BestPractices`, `PerlCritic` |

### DiagnosticCode Methods

| Method | Returns |
|--------|---------|
| `as_str()` | Stable string code (e.g., `"PL001"`, `"PC003"`) |
| `severity()` | Default `DiagnosticSeverity` for this code |
| `category()` | `DiagnosticCategory` classification |
| `tags()` | `&[DiagnosticTag]` (e.g., `UnusedVariable` returns `[Unnecessary]`) |
| `documentation_url()` | `Option<&str>` link to docs (None for Perl::Critic codes) |
| `parse_code(code)` | Parse `"PL001"` string into `Option<DiagnosticCode>` |
| `from_message(msg)` | Infer code from error message text |

### Code Ranges

| Range | Category | Examples |
|-------|----------|----------|
| PL001-PL099 | Parser | `ParseError`, `SyntaxError`, `UnexpectedEof` |
| PL100-PL199 | Strict/warnings | `MissingStrict`, `MissingWarnings`, `UnusedVariable`, `UndefinedVariable` |
| PL200-PL299 | Package/module | `MissingPackageDeclaration`, `DuplicatePackage` |
| PL300-PL399 | Subroutine | `DuplicateSubroutine`, `MissingReturn` |
| PL400-PL499 | Best practices | `BarewordFilehandle`, `TwoArgOpen`, `ImplicitReturn` |
| PC001-PC005 | Perl::Critic | `CriticSeverity1` through `CriticSeverity5` |

## Usage

```rust
use perl_diagnostics_codes::{DiagnosticCode, DiagnosticSeverity, DiagnosticCategory};

let code = DiagnosticCode::ParseError;
assert_eq!(code.as_str(), "PL001");
assert_eq!(code.severity(), DiagnosticSeverity::Error);
assert_eq!(code.category(), DiagnosticCategory::Parser);

// Parse from string
let code = DiagnosticCode::parse_code("PL102");
assert_eq!(code, Some(DiagnosticCode::UnusedVariable));

// Infer from message
let code = DiagnosticCode::from_message("Missing 'use strict' pragma");
assert_eq!(code, Some(DiagnosticCode::MissingStrict));
```

## Important Notes

- Diagnostic codes are **stable across versions** — never renumber existing codes
- New codes are appended at the end of their respective range
- The crate has **zero required dependencies** (`serde` is optional)
- All public enums implement `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, and `Display` (where applicable)
- Used by `perl-lsp-diagnostics` for diagnostic reporting in the LSP server
- Source is a single file: `src/lib.rs`
