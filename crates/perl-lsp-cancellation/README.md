# perl-lsp-cancellation

Standalone microcrate for LSP cancellation infrastructure.

## Scope

- Cancellation tokens with atomic state checks.
- Thread-safe cancellation registry with optional token cache.
- Provider cleanup context and RAII request cleanup guard.
- Performance metrics for registration/cancellation/completion.

The crate intentionally owns only cancellation concerns and is consumed by `perl-lsp`
through a narrow re-exporting module.
