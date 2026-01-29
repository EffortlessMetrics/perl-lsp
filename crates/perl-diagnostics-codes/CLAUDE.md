# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-diagnostics-codes` is a **Tier 1 leaf crate** providing stable diagnostic codes and severity levels for Perl LSP.

**Purpose**: Stable diagnostic codes and severity levels for Perl LSP — ensures consistent error/warning identification across versions.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-diagnostics-codes        # Build this crate
cargo test -p perl-diagnostics-codes         # Run tests
cargo clippy -p perl-diagnostics-codes       # Lint
cargo doc -p perl-diagnostics-codes --open   # View documentation
```

## Architecture

### Dependencies

- `serde` (optional) - Serialization

### Key Types

| Type | Purpose |
|------|---------|
| `DiagnosticCode` | Unique diagnostic identifier |
| `Severity` | Error, Warning, Info, Hint |
| `Category` | Diagnostic category |

### Diagnostic Code Format

```
PL-XXXX
│  └─── 4-digit number
└────── "PL" prefix for Perl
```

### Code Ranges

| Range | Category |
|-------|----------|
| PL-0001 to PL-0999 | Syntax errors |
| PL-1000 to PL-1999 | Semantic errors |
| PL-2000 to PL-2999 | Warnings |
| PL-3000 to PL-3999 | Style hints |
| PL-4000 to PL-4999 | Deprecations |

### Common Codes

```rust
// Syntax errors
PL_0001  // Unexpected token
PL_0002  // Unterminated string
PL_0003  // Unbalanced delimiter

// Semantic errors
PL_1001  // Undefined variable
PL_1002  // Undefined subroutine
PL_1003  // Type mismatch

// Warnings
PL_2001  // Unused variable
PL_2002  // Redefining subroutine
PL_2003  // Missing strict/warnings
```

## Usage

```rust
use perl_diagnostics_codes::{DiagnosticCode, Severity};

let code = DiagnosticCode::UNDEFINED_VARIABLE;
assert_eq!(code.as_str(), "PL-1001");
assert_eq!(code.severity(), Severity::Error);
assert_eq!(code.message(), "Undefined variable");
```

### Integration with LSP

```rust
use lsp_types::Diagnostic;

let diagnostic = Diagnostic {
    code: Some(NumberOrString::String(code.as_str().to_string())),
    severity: Some(code.severity().into()),
    message: format!("{}: {}", code.as_str(), code.message()),
    ..
};
```

## Important Notes

- Codes are stable across versions (don't renumber)
- New codes are added at the end of each range
- Deprecated codes are kept but marked deprecated
- Used by `perl-lsp-diagnostics` for reporting
