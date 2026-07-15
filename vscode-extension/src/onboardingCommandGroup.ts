import * as vscode from 'vscode';

export interface OnboardingCommandContext {
  readonly showWhatsNew: () => Promise<void>;
  readonly openConfigurationGuide: () => void;
  readonly checkForUpdate: () => Promise<void>;
}

export function registerOnboardingCommandGroup(
  dependencies: OnboardingCommandContext,
): vscode.Disposable[] {
  const showWhatsNewCommand = vscode.commands.registerCommand('perl-lsp.showWhatsNew', async () => {
    await dependencies.showWhatsNew();
  });
  const openConfigurationGuideCommand = vscode.commands.registerCommand(
    'perl-lsp.openConfigurationGuide',
    () => {
      dependencies.openConfigurationGuide();
    },
  );
  const checkForUpdateCommand = vscode.commands.registerCommand(
    'perl-lsp.checkForUpdate',
    async () => {
      await dependencies.checkForUpdate();
    },
  );

  return [showWhatsNewCommand, openConfigurationGuideCommand, checkForUpdateCommand];
}
