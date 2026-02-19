# perl-lsp-code-actions

Code action providers for Perl LSP quick fixes and refactorings.

## Scope

- Produces LSP code actions from AST, diagnostics, and workspace context.
- Supports quick fixes and higher-level refactors (extract/import/modernization flows).
- Integrates with rename and diagnostics crates for cross-feature actions.

## Public Surface

- `CodeActionsProvider`, `CodeAction`, `CodeActionKind`.
- `EnhancedCodeActionsProvider` for advanced refactors.
- `CodeActionEdit` payload type.

## Workspace Role

Internal feature crate consumed by `perl-lsp` request dispatch.

## License

MIT OR Apache-2.0.
