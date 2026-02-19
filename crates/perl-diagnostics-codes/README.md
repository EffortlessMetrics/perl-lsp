# perl-diagnostics-codes

Stable diagnostic codes and severity levels for the Perl LSP ecosystem.

Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## Overview

This crate defines the canonical `DiagnosticCode` enum, `DiagnosticSeverity` levels, `DiagnosticTag` values, and `DiagnosticCategory` classifications used across the Perl LSP toolchain. Codes are stable across versions and map directly to LSP protocol values.

## Code Ranges

| Range | Category |
|-------------|---------------------------|
| PL001-PL099 | Parser diagnostics |
| PL100-PL199 | Strict/warnings |
| PL200-PL299 | Package/module |
| PL300-PL399 | Subroutine |
| PL400-PL499 | Best practices |
| PC001-PC005 | Perl::Critic violations |

## Usage

```rust
use perl_diagnostics_codes::{DiagnosticCode, DiagnosticSeverity};

let code = DiagnosticCode::ParseError;
assert_eq!(code.as_str(), "PL001");
assert_eq!(code.severity(), DiagnosticSeverity::Error);
assert_eq!(code.category(), perl_diagnostics_codes::DiagnosticCategory::Parser);
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
