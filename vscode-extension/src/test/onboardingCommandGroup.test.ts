import * as vscode from 'vscode';
import {
  registerOnboardingCommandGroup,
  type OnboardingCommandContext,
} from '../onboardingCommandGroup';

function makeDependencies(): OnboardingCommandContext & {
  showWhatsNew: jest.Mock;
  openConfigurationGuide: jest.Mock;
  checkForUpdate: jest.Mock;
} {
  return {
    showWhatsNew: jest.fn(async () => undefined),
    openConfigurationGuide: jest.fn(),
    checkForUpdate: jest.fn(async () => undefined),
  };
}

beforeEach(() => {
  jest.clearAllMocks();
});

describe('registerOnboardingCommandGroup', () => {
  test('registers onboarding and update commands and delegates execution', async () => {
    const dependencies = makeDependencies();

    const disposables = registerOnboardingCommandGroup(dependencies);

    expect(disposables).toHaveLength(3);
    await vscode.commands.executeCommand('perl-lsp.showWhatsNew');
    await vscode.commands.executeCommand('perl-lsp.openConfigurationGuide');
    await vscode.commands.executeCommand('perl-lsp.checkForUpdate');

    expect(dependencies.showWhatsNew).toHaveBeenCalledTimes(1);
    expect(dependencies.openConfigurationGuide).toHaveBeenCalledTimes(1);
    expect(dependencies.checkForUpdate).toHaveBeenCalledTimes(1);
  });

  test('does not invoke callbacks during registration', () => {
    const dependencies = makeDependencies();

    registerOnboardingCommandGroup(dependencies);

    expect(dependencies.showWhatsNew).not.toHaveBeenCalled();
    expect(dependencies.openConfigurationGuide).not.toHaveBeenCalled();
    expect(dependencies.checkForUpdate).not.toHaveBeenCalled();
  });
});
