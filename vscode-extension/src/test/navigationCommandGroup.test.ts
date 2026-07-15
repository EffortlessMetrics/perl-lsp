import * as vscode from 'vscode';
import {
  registerNavigationCommandGroup,
  type NavigationCommandContext,
} from '../navigationCommandGroup';

function makeDependencies(): NavigationCommandContext & {
  openDemoProject: jest.Mock;
  organizeImports: jest.Mock;
  showVersion: jest.Mock;
  showStatusMenu: jest.Mock;
} {
  return {
    openDemoProject: jest.fn(async () => undefined),
    organizeImports: jest.fn(async () => undefined),
    showVersion: jest.fn(async () => undefined),
    showStatusMenu: jest.fn(async () => undefined),
  };
}

beforeEach(() => {
  jest.clearAllMocks();
});

let registeredDisposables: vscode.Disposable[] = [];

afterEach(() => {
  for (const disposable of registeredDisposables) {
    disposable.dispose();
  }
  registeredDisposables = [];
});

describe('registerNavigationCommandGroup', () => {
  test('registers navigation commands and delegates execution', async () => {
    const dependencies = makeDependencies();

    registeredDisposables = registerNavigationCommandGroup(dependencies);

    expect(registeredDisposables).toHaveLength(4);
    await vscode.commands.executeCommand('perl-lsp.openDemoProject');
    await vscode.commands.executeCommand('perl-lsp.organizeImports');
    await vscode.commands.executeCommand('perl-lsp.showVersion');
    await vscode.commands.executeCommand('perl-lsp.showStatusMenu');

    expect(dependencies.openDemoProject).toHaveBeenCalledTimes(1);
    expect(dependencies.organizeImports).toHaveBeenCalledTimes(1);
    expect(dependencies.showVersion).toHaveBeenCalledTimes(1);
    expect(dependencies.showStatusMenu).toHaveBeenCalledTimes(1);
  });

  test('does not invoke callbacks during registration', () => {
    const dependencies = makeDependencies();

    registeredDisposables = registerNavigationCommandGroup(dependencies);

    expect(dependencies.openDemoProject).not.toHaveBeenCalled();
    expect(dependencies.organizeImports).not.toHaveBeenCalled();
    expect(dependencies.showVersion).not.toHaveBeenCalled();
    expect(dependencies.showStatusMenu).not.toHaveBeenCalled();
  });
});
