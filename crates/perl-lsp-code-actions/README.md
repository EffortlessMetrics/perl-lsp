# perl-lsp-code-actions

LSP code actions provider for the Perl LSP ecosystem. Generates quick fixes
driven by diagnostic codes and refactoring actions driven by AST analysis.

## Public API

- `CodeActionsProvider` -- diagnostic-driven quick fixes (declare variable,
  add pragmas, fix parse errors, fix barewords, handle unused variables).
- `EnhancedCodeActionsProvider` -- AST-driven refactorings (extract variable,
  extract subroutine, loop conversion, import management, postfix conversion,
  error-checking insertion).
- `CodeAction`, `CodeActionKind`, `CodeActionEdit` -- result types.

## Workspace Role

Internal feature crate in the `tree-sitter-perl-rs` workspace, consumed by
`perl-lsp` for `textDocument/codeAction` request dispatch. Not intended for
standalone use outside the workspace.

## License

MIT OR Apache-2.0
