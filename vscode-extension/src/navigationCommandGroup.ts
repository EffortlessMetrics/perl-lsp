import * as vscode from 'vscode';

export interface NavigationCommandContext {
  readonly openDemoProject: () => Promise<void>;
  readonly organizeImports: () => Promise<void>;
  readonly showVersion: () => Promise<void>;
  readonly showStatusMenu: () => Promise<void>;
}

export function registerNavigationCommandGroup(
  dependencies: NavigationCommandContext,
): vscode.Disposable[] {
  const openDemoProjectCommand = vscode.commands.registerCommand(
    'perl-lsp.openDemoProject',
    dependencies.openDemoProject,
  );
  const organizeImportsCommand = vscode.commands.registerCommand(
    'perl-lsp.organizeImports',
    dependencies.organizeImports,
  );
  const showVersionCommand = vscode.commands.registerCommand(
    'perl-lsp.showVersion',
    dependencies.showVersion,
  );
  const showStatusMenuCommand = vscode.commands.registerCommand(
    'perl-lsp.showStatusMenu',
    dependencies.showStatusMenu,
  );

  return [
    openDemoProjectCommand,
    organizeImportsCommand,
    showVersionCommand,
    showStatusMenuCommand,
  ];
}
