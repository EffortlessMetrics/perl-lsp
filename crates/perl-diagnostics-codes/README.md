# perl-diagnostics-codes

Stable diagnostic codes and severity levels for the Perl LSP ecosystem.

## Features

- **Type-safe diagnostic codes**: Enum-based codes with stable string representations
- **Severity levels**: Error, Warning, Information, Hint
- **Diagnostic tags**: Unnecessary, Deprecated
- **Categories**: Parser, StrictWarnings, PackageModule, Subroutine, BestPractices, PerlCritic
- **Message inference**: Infer diagnostic codes from error messages

## Code Ranges

| Range       | Category                  |
|-------------|---------------------------|
| PL001-PL099 | Parser diagnostics        |
| PL100-PL199 | Strict/warnings           |
| PL200-PL299 | Package/module            |
| PL300-PL399 | Subroutine                |
| PL400-PL499 | Best practices            |
| PC001-PC005 | Perl::Critic violations   |

## Usage

```rust
use perl_diagnostics_codes::{DiagnosticCode, DiagnosticSeverity};

// Get code information
let code = DiagnosticCode::ParseError;
assert_eq!(code.as_str(), "PL001");
assert_eq!(code.severity(), DiagnosticSeverity::Error);

// Infer code from message
let code = DiagnosticCode::from_message("Missing 'use strict' pragma");
assert_eq!(code, Some(DiagnosticCode::MissingStrict));

// Parse code string
let code = DiagnosticCode::parse_code("PL102");
assert_eq!(code, Some(DiagnosticCode::UnusedVariable));
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
