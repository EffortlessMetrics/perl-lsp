# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-regex` is a **Tier 1 leaf crate** that validates Perl regular expressions for security and performance risks.

**Purpose**: Detect dangerous or expensive regex constructs (nested quantifiers, embedded code, excessive nesting/branching) and report offset-aware diagnostics for IDE integration.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-regex            # Build this crate
cargo test -p perl-regex             # Run tests
cargo clippy -p perl-regex           # Lint
cargo doc -p perl-regex --open       # View documentation
```

## Architecture

### Dependencies

- `thiserror` - Derive macro for `RegexError`

### Key Types

| Type | Purpose |
|------|---------|
| `RegexValidator` | Main validator struct; holds configurable limits (`max_nesting`, `max_unicode_properties`) |
| `RegexError` | Error enum with `Syntax { message, offset }` variant |
| `GroupType` | Internal enum tracking group kinds during parsing (Normal, Lookbehind, BranchReset) |

### RegexValidator Methods

| Method | Purpose |
|--------|---------|
| `new()` | Constructor with default limits (nesting: 10, unicode properties: 50) |
| `validate(pattern, start_pos)` | Full validation pass returning `Result<(), RegexError>` |
| `detects_code_execution(pattern)` | Returns `true` if pattern contains `(?{...})` or `(??{...})` |
| `detect_nested_quantifiers(pattern)` | Returns `true` if pattern has nested quantifiers like `(a+)+` |

### What It Checks

- Nested quantifiers that cause catastrophic backtracking (e.g. `(a+)+`, `(a*)*`)
- Lookbehind nesting depth (max 10)
- Branch reset group nesting depth (max 10)
- Branch count within branch reset groups (max 50)
- Unicode property count via `\p{...}` / `\P{...}` (max 50)
- Embedded code execution via `(?{...})` and `(??{...})`

## Usage

```rust
use perl_regex::{RegexValidator, RegexError};

let validator = RegexValidator::new();

// Full validation
let result = validator.validate("(a+)+", 0);
assert!(result.is_err()); // nested quantifiers detected

// Check for embedded code
assert!(validator.detects_code_execution("(?{ print 'hi' })"));
assert!(!validator.detects_code_execution("(?:foo)"));
```

## Important Notes

- This crate validates regex safety/complexity, not full Perl regex syntax
- Actual regex execution is handled by Perl at runtime
- Nested quantifier detection is heuristic-based, not a full regex parse
- `RegexValidator` implements `Default` (delegates to `new()`)
- No internal workspace dependencies -- only external dep is `thiserror`
