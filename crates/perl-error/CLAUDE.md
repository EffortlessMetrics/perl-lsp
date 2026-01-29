# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-error` is a **Tier 2 error infrastructure crate** providing error types and recovery strategies for the Perl parser.

**Purpose**: Error types and recovery strategies — unified error handling for lexer, parser, and semantic analysis.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-error            # Build this crate
cargo test -p perl-error             # Run tests
cargo clippy -p perl-error           # Lint
cargo doc -p perl-error --open       # View documentation
```

## Architecture

### Dependencies

- `thiserror` - Error derive macro
- `perl-ast` - AST types
- `perl-regex` - Regex error types
- `perl-position-tracking` - Position types
- `perl-lexer` - Lexer error types

### Error Categories

| Category | Purpose |
|----------|---------|
| `LexError` | Tokenization failures |
| `ParseError` | Syntax errors |
| `SemanticError` | Type/scope errors |
| `RecoveryError` | Error recovery metadata |

### Error Structure

```rust
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
    pub message: String,
    pub recovery: Option<RecoveryStrategy>,
}
```

### Recovery Strategies

| Strategy | Description |
|----------|-------------|
| `SkipToDelimiter` | Skip until matching delimiter |
| `SkipToSemicolon` | Skip until statement end |
| `InsertMissing` | Insert expected token |
| `DeleteUnexpected` | Remove unexpected token |

## Usage

```rust
use perl_error::{ParseError, ParseErrorKind};

fn handle_error(err: ParseError) {
    match err.kind {
        ParseErrorKind::UnexpectedToken { expected, found } => {
            // Handle unexpected token
        },
        ParseErrorKind::UnterminatedString => {
            // Handle unterminated string
        },
        // ...
    }
}
```

### Error Recovery

```rust
use perl_error::RecoveryStrategy;

// Parser can continue after errors
match parser.parse_statement() {
    Ok(stmt) => statements.push(stmt),
    Err(e) => {
        errors.push(e);
        parser.recover(e.recovery);
    }
}
```

## Important Notes

- Errors include position information for diagnostics
- Recovery strategies enable continued parsing after errors
- Integrate with `perl-diagnostics-codes` for LSP error codes
