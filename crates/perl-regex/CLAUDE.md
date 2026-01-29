# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-regex` is a **Tier 1 leaf crate** providing regex operator validation and parsing for Perl.

**Purpose**: Regex operator validation and parsing — handles Perl regex syntax, modifiers, and special constructs.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-regex            # Build this crate
cargo test -p perl-regex             # Run tests
cargo clippy -p perl-regex           # Lint
cargo doc -p perl-regex --open       # View documentation
```

## Architecture

### Dependencies

- `thiserror` - Error definitions

### Regex Modifiers

| Modifier | Purpose |
|----------|---------|
| `i` | Case-insensitive |
| `m` | Multi-line mode |
| `s` | Single-line mode (`.` matches newline) |
| `x` | Extended mode (whitespace ignored) |
| `g` | Global matching |
| `e` | Evaluate replacement as code |
| `o` | Compile once |
| `p` | Preserve match variables |

### Key Types

| Type | Purpose |
|------|---------|
| `RegexFlags` | Parsed modifier flags |
| `RegexError` | Validation errors |

### Validation

The crate validates:
- Modifier combinations (some are mutually exclusive)
- Balanced delimiters in regex patterns
- Escape sequences
- Character class syntax

## Usage

```rust
use perl_regex::{validate_regex, RegexFlags};

// Validate regex pattern
let result = validate_regex("/pattern/imsx");

// Parse modifiers
let flags = RegexFlags::parse("imsx")?;
assert!(flags.case_insensitive);
assert!(flags.multiline);
```

## Important Notes

- This crate validates syntax, not regex semantics
- Actual regex execution is handled by Perl at runtime
- Focus is on providing IDE feedback for invalid patterns
