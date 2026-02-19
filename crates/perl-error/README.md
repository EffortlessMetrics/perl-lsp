# perl-error

Error types, classification, and recovery strategies for Perl parsing.

## Overview

`perl-error` provides the unified error infrastructure used across the Perl parser
ecosystem. It defines parser-facing error types, parse budget tracking to bound
parser work on adversarial input, error classification for LSP diagnostics, and
recovery traits for resilient parsing.

## Public API

| Type | Purpose |
|------|---------|
| `ParseError` | Enum of all parse error variants (syntax, lexer, regex, EOF, etc.) |
| `ParseOutput` | Structured parse result combining AST, diagnostics, and budget usage |
| `ParseBudget` | Configurable limits for errors, depth, token skips, and recoveries |
| `BudgetTracker` | Runtime budget consumption tracker |
| `ErrorContext` | Error enriched with line/column, source text, and fix suggestions |
| `classifier::ErrorClassifier` | Heuristic classification of error nodes into specific kinds |
| `classifier::ParseErrorKind` | Fine-grained error categories (unclosed string, missing semicolon, etc.) |
| `recovery::ErrorRecovery` | Trait for sync-point-based error recovery in parsers |
| `recovery::RecoveryResult` | Outcome of a budget-aware recovery attempt |

## Workspace Role

Core internal crate consumed by `perl-parser`, `perl-parser-core`, `perl-lsp-diagnostics`,
and other crates that need structured error handling and recovery.

## License

MIT OR Apache-2.0
