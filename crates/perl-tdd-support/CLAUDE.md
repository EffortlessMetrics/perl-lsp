# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-tdd-support` is a **Tier 3 testing utility crate** providing test-driven development helpers for the Perl LSP workspace.

**Purpose**: Test-driven development helpers for Perl — provides testing utilities, assertions, and fixtures used across the workspace.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-tdd-support          # Build this crate
cargo test -p perl-tdd-support           # Run tests
cargo clippy -p perl-tdd-support         # Lint
cargo doc -p perl-tdd-support --open     # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - Parsing access
- `serde`, `serde_json` - Test data serialization

### Optional Dependencies (with `lsp-compat`)

- `lsp-types` - LSP type testing
- `url` - URI handling

### Features

| Feature | Purpose |
|---------|---------|
| `lsp-compat` | LSP type compatibility (optional) |

### Key Functions

| Function | Purpose |
|----------|---------|
| `must` | Assert Result is Ok, panic with context |
| `must_some` | Assert Option is Some, panic with context |
| `parse_fixture` | Parse a test fixture file |
| `assert_parse_ok` | Assert parsing succeeds |
| `assert_parse_err` | Assert parsing fails |

## Usage

### Result/Option Assertions

```rust
use perl_tdd_support::{must, must_some};

#[test]
fn test_parsing() -> Result<()> {
    // Instead of .unwrap() which doesn't show context
    let ast = must(parser.parse())?;
    let symbol = must_some(ast.find_symbol("foo"))?;
    Ok(())
}
```

### Parse Testing

```rust
use perl_tdd_support::{assert_parse_ok, assert_parse_err};

#[test]
fn test_valid_perl() {
    assert_parse_ok("my $x = 42;");
}

#[test]
fn test_invalid_perl() {
    assert_parse_err("my $x = ;");  // Missing expression
}
```

### Fixture Loading

```rust
use perl_tdd_support::parse_fixture;

#[test]
fn test_from_fixture() {
    let (source, expected) = parse_fixture("tests/fixtures/complex.pl");
    let result = analyze(source);
    assert_eq!(result, expected);
}
```

## Test Patterns

### Avoiding `unwrap()` and `expect()`

Per coding standards, use this crate's helpers instead:

```rust
// Bad
let result = parse().unwrap();

// Good
let result = must(parse())?;

// Bad
let item = vec.first().expect("should have item");

// Good
let item = must_some(vec.first())?;
```

## Important Notes

- Used throughout workspace for consistent test patterns
- Provides better error messages than raw `unwrap()`
- Integrates with workspace coding standards
- Test-only dependency (not included in production builds)
