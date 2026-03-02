# perl-lsp-completion-model

Shared completion payload model and deterministic ordering utilities for Perl LSP completion providers.

## Responsibilities

- Define completion item contracts (`CompletionItem`, `CompletionItemKind`)
- Provide deterministic `deduplicate_and_sort` ordering policy

This crate keeps completion data contracts and ranking policy decoupled from provider orchestration logic.
