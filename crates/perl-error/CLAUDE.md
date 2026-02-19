# CLAUDE.md

This file provides guidance to Claude Code when working with code in this crate.

## Crate Overview

- **Name**: `perl-error`
- **Version**: 0.9.1
- **Tier**: Tier 1 leaf crate (depends on `perl-ast`, `perl-regex`, `perl-position-tracking`, `perl-lexer`)
- **Purpose**: Unified error types, budget tracking, error classification, and recovery traits for the Perl parser ecosystem.

## Commands

```bash
cargo build -p perl-error            # Build
cargo test -p perl-error             # Run tests
cargo clippy -p perl-error           # Lint
cargo doc -p perl-error --open       # View documentation
```

## Architecture

### Dependencies

| Dependency | Usage |
|-----------|-------|
| `thiserror` | `#[derive(Error)]` on `ParseError` enum |
| `perl-ast` | `Node`, `NodeKind`, `SourceLocation` for AST integration |
| `perl-regex` | `RegexError` conversion via `From` impl |
| `perl-position-tracking` | `LineIndex`, `Range` for source position mapping |
| `perl-lexer` | `TokenType` used in recovery trait signatures |

### Key Types (lib.rs)

| Type | Description |
|------|-------------|
| `ParseError` | `thiserror` enum: `UnexpectedEof`, `UnexpectedToken`, `SyntaxError`, `LexerError`, `RecursionLimit`, `InvalidNumber`, `InvalidString`, `UnclosedDelimiter`, `InvalidRegex`, `NestingTooDeep` |
| `ParseResult<T>` | `Result<T, ParseError>` alias |
| `ParseOutput` | Structured output: AST + diagnostics + budget usage + terminated-early flag |
| `ParseBudget` | Configurable limits: `max_errors`, `max_depth`, `max_tokens_skipped`, `max_recoveries` with `default()`, `strict()`, `for_ide()`, `unlimited()` presets |
| `BudgetTracker` | Tracks consumption: `errors_emitted`, `current_depth`, `tokens_skipped`, `recoveries_attempted`; methods like `begin_recovery()`, `can_skip_more()` |
| `ErrorContext` | Error enriched with `line`, `column`, `source_line`, `suggestion` |
| `get_error_contexts()` | Free function: enriches `&[ParseError]` with source context using `LineIndex` |

### Modules

| Module | Description |
|--------|-------------|
| `classifier` | `ErrorClassifier` struct with `classify()`, `get_diagnostic_message()`, `get_suggestion()`, `get_explanation()`. `ParseErrorKind` enum with 15 variants for fine-grained categorization. |
| `recovery` | `recovery::ParseError` (separate from root `ParseError`): range-based error with expected/found/hint. `SyncPoint` enum, `RecoveryResult` enum, `ErrorRecovery` trait, `ParserErrorRecovery` trait, `StatementRecovery` trait. |

### Important: Two ParseError Types

The crate has two distinct `ParseError` types:
- `crate::ParseError` (lib.rs) -- `thiserror` enum used across the parser ecosystem
- `crate::recovery::ParseError` (recovery.rs) -- struct with `Range`, `expected`, `found`, `recovery_hint` used specifically in recovery context

### Conversions

- `From<perl_regex::RegexError> for ParseError` -- converts regex syntax errors to `ParseError::SyntaxError`

## Usage

```rust
use perl_error::{ParseError, ParseResult, ParseBudget, BudgetTracker};

// Create errors
let err = ParseError::syntax("missing semicolon", 42);
let err = ParseError::unexpected("semicolon", "comma", 15);

// Budget-bounded parsing
let budget = ParseBudget::strict();
let mut tracker = BudgetTracker::new();
tracker.record_error();
if tracker.errors_exhausted(&budget) { /* stop */ }

// Structured output
use perl_error::ParseOutput;
// let output = ParseOutput::finish(ast, diagnostics, tracker, false);
```

## Important Notes

- All tests are inline (`#[cfg(test)] mod tests`) in `lib.rs` and `classifier.rs`.
- Recovery traits in `recovery.rs` are meant to be implemented by the parser crate, not here.
- `ParseBudget` defaults are tuned for IDE usage; use `strict()` for untrusted input.
- The `ErrorClassifier::classify()` method uses heuristics (quote counting, line-level delimiter balance) and may produce false positives on complex Perl source.
