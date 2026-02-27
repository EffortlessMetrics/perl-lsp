# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-dap-eval` is a **Tier 1 leaf crate** (no internal workspace dependencies) providing safe expression evaluation validation for Perl DAP debugging sessions.

**Purpose**: Validates Perl expressions before evaluation during debugging, blocking dangerous operations that could mutate state, execute code, or perform I/O.

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

- `regex` - Pattern matching for dangerous operation detection
- `once_cell` - Lazy initialization of compiled regex patterns
- `thiserror` - Derive macro for `ValidationError`

### Modules

| Module | Purpose |
|--------|---------|
| `lib.rs` | Public API re-exports, crate-level documentation |
| `validator.rs` | `SafeEvaluator` struct, `ValidationError` enum, validation logic, context-aware helper functions |
| `patterns.rs` | `DANGEROUS_OPERATIONS` constant list, `ASSIGNMENT_OPERATORS`, compiled `DANGEROUS_OPS_RE` and `REGEX_MUTATION_RE` lazy regexes |

### Key Types

| Type | Purpose |
|------|---------|
| `SafeEvaluator` | Main validator; call `validate(&self, expr)` to check an expression |
| `ValidationError` | Enum: `DangerousOperation`, `AssignmentOperator`, `IncrementDecrement`, `Backticks`, `RegexMutation`, `ContainsNewlines` |
| `ValidationResult` | Type alias for `Result<(), ValidationError>` |
| `DANGEROUS_OPERATIONS` | `&[&str]` constant listing ~80 blocked Perl built-in names |

### Validation Pipeline (in order)

1. Reject newlines (command injection vector)
2. Reject backticks (shell execution)
3. Reject assignment operators (`=`, `+=`, `.=`, etc.)
4. Reject increment/decrement (`++`, `--`)
5. Regex-based dangerous operation detection with context-aware filtering
6. Regex mutation detection (`s///`, `tr///`, `y///`)

### Context-Aware False Positive Avoidance

The validator allows matches that are:
- Inside single-quoted strings
- Sigil-prefixed identifiers (`$print`, `@say`, `%exit`)
- Simple braced scalar variables (`${print}`)
- Package-qualified but not `CORE::` (`Foo::print` is OK, `CORE::print` is blocked)
- Escape sequences (`\s` is not `s///`)

## Usage

```rust
use perl_dap_eval::{SafeEvaluator, ValidationResult};

let evaluator = SafeEvaluator::new();

// Safe expressions
assert!(evaluator.validate("$x + $y").is_ok());
assert!(evaluator.validate("$hash{key}").is_ok());
assert!(evaluator.validate("$print").is_ok());      // sigil-prefixed variable

// Dangerous expressions
assert!(evaluator.validate("system('ls')").is_err());
assert!(evaluator.validate("$x = 1").is_err());      // assignment
assert!(evaluator.validate("s/foo/bar/").is_err());   // regex mutation
```

## Important Notes

- This crate has **no internal workspace dependencies** (only external: regex, once_cell, thiserror)
- Security is the primary concern -- conservative approach blocks if uncertain
- If the regex fails to compile, the validator **allows** the expression (fail-open for regex, not fail-closed)
- Used by `perl-dap` for safe evaluation mode in debug sessions
- `DANGEROUS_OPERATIONS` is re-exported for downstream extension/testing
