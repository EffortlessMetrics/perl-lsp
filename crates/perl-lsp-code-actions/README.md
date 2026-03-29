# perl-lsp-code-actions

LSP code actions provider for the Perl LSP ecosystem. Generates quick fixes
driven by diagnostic codes and refactoring actions driven by AST analysis.

## When to use this crate

Use `perl-lsp-code-actions` when you want Perl-aware `textDocument/codeAction`
support without pulling in the full `perl-lsp` runtime. It is most useful for
Rust-based editor tooling that already has diagnostics or AST context and needs
to build quick fixes or refactorings.

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
