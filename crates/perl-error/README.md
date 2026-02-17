# perl-error

Error types, classification, and recovery helpers for Perl parsing.

## Scope

- Defines parser-facing error types (`ParseError`) and contextual metadata.
- Provides parse budget tracking to bound parser work.
- Includes recovery and classifier modules for resilient parse workflows.

## Public Surface

- `ParseError`, `ParseOutput`, `ErrorContext`.
- `ParseBudget`, `BudgetTracker`.
- Modules: `classifier`, `recovery`.

## Workspace Role

Core internal crate used by parser, tokenizer, and diagnostics layers.

## License

MIT OR Apache-2.0.
