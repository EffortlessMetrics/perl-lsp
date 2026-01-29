# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-dap-eval` is a **Tier 7 DAP feature module** providing safe expression evaluation validation for debugging.

**Purpose**: Safe expression evaluation validation for Perl DAP — validates expressions before evaluation to prevent dangerous operations.

**Version**: 0.1.0

## Commands

```bash
cargo build -p perl-dap-eval             # Build this crate
cargo test -p perl-dap-eval              # Run tests
cargo clippy -p perl-dap-eval            # Lint
cargo doc -p perl-dap-eval --open        # View documentation
```

## Architecture

### Dependencies

- `regex` - Pattern matching for validation
- `once_cell` - Lazy static patterns
- `thiserror` - Error definitions

### Security Focus

This crate is **security-critical** — it prevents arbitrary code execution during debugging.

### Validation Rules

| Rule | Examples Blocked |
|------|------------------|
| No system calls | `system()`, `` `cmd` ``, `exec()` |
| No file writes | `open(F, ">...")`, `unlink()` |
| No eval | `eval "..."`, `do $file` |
| No require/use | `require "..."`, `use Module` |
| No network | `socket()`, `connect()` |
| Size limits | Expressions over N chars |

### Safe Operations

| Operation | Example |
|-----------|---------|
| Variable access | `$x`, `@arr`, `%hash` |
| Simple arithmetic | `$x + 1`, `$a * $b` |
| String ops | `$s . "suffix"`, `length($s)` |
| Array/hash access | `$arr[0]`, `$hash{key}` |
| Method calls | `$obj->method()` (validated) |

## Usage

```rust
use perl_dap_eval::{ExpressionValidator, ValidationResult};

let validator = ExpressionValidator::new();

// Check if expression is safe
match validator.validate("$x + 1") {
    ValidationResult::Safe => {
        // OK to evaluate
    },
    ValidationResult::Unsafe(reason) => {
        println!("Blocked: {}", reason);
    },
    ValidationResult::NeedsReview => {
        // Might be safe, needs confirmation
    }
}
```

### Configuration

```rust
let config = ValidationConfig {
    max_length: 1000,
    allow_method_calls: true,
    blocked_functions: vec!["system", "exec", "eval"],
};

let validator = ExpressionValidator::with_config(config);
```

## Important Notes

- Security is the primary concern
- Conservative approach — block if uncertain
- Allowlist approach for known-safe patterns
- All rejections are logged for audit
