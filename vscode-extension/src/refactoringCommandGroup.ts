import * as vscode from 'vscode';

/** Explicit callbacks for refactoring command registration. */
export interface RefactoringCommandContext {
  readonly extractVariable: () => Promise<void>;
  readonly extractMethod: () => Promise<void>;
  readonly showRefactoringOptions: () => Promise<void>;
}

/** Register refactoring commands without owning client or editor state. */
export function registerRefactoringCommandGroup(
  dependencies: RefactoringCommandContext,
): vscode.Disposable[] {
  const extractVariableCommand = vscode.commands.registerCommand('perl-lsp.extractVariable', () =>
    dependencies.extractVariable(),
  );
  const extractMethodCommand = vscode.commands.registerCommand('perl-lsp.extractMethod', () =>
    dependencies.extractMethod(),
  );
  const showRefactoringOptionsCommand = vscode.commands.registerCommand(
    'perl-lsp.showRefactoringOptions',
    () => dependencies.showRefactoringOptions(),
  );

  return [extractVariableCommand, extractMethodCommand, showRefactoringOptionsCommand];
}
