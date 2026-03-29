# perl-lsp-code-actions

Perl code action engine for quick fixes and AST-driven refactorings. It turns
diagnostics and structured syntax into actionable edits such as import fixes,
declaration insertion, extract-variable refactors, and loop conversions.

## Use this crate when

Use `perl-lsp-code-actions` if you need the code-action logic itself. Use
`perl-lsp-providers` when you want the umbrella re-export surface. This crate is
the provider layer, not the editor transport layer.

## Key exports

- `CodeActionsProvider` - diagnostic-driven fixes for common Perl problems
- `EnhancedCodeActionsProvider` - AST-driven refactorings and structural edits
- `CodeAction`, `CodeActionKind`, `CodeActionEdit` - shared action types

## What it covers

- missing variable declarations and pragma fixes
- parse-error and bareword recovery actions
- import management and other refactor helpers
- extract-variable, extract-subroutine, and loop-modernization actions

## Stack role

This crate sits behind the editor-facing LSP code-action handlers in `perl-lsp`.
It consumes diagnostics and AST state, then returns the edits the client can
offer to the user.
