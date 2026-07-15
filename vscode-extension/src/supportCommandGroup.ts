import * as vscode from 'vscode';

/** Explicit callbacks for support and issue-reporting command registration. */
export interface SupportCommandContext {
  readonly reportIssue: () => Promise<void>;
}

/** Register support commands without owning diagnostic collection behavior. */
export function registerSupportCommandGroup(
  dependencies: SupportCommandContext,
): vscode.Disposable[] {
  const reportIssueCommand = vscode.commands.registerCommand('perl-lsp.reportIssue', () =>
    dependencies.reportIssue(),
  );

  return [reportIssueCommand];
}
