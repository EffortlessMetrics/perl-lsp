import * as vscode from 'vscode';
import {
  registerRefactoringCommandGroup,
  type RefactoringCommandContext,
} from '../refactoringCommandGroup';

function makeDependencies(): RefactoringCommandContext & {
  extractVariable: jest.Mock;
  extractMethod: jest.Mock;
  showRefactoringOptions: jest.Mock;
} {
  return {
    extractVariable: jest.fn(async () => undefined),
    extractMethod: jest.fn(async () => undefined),
    showRefactoringOptions: jest.fn(async () => undefined),
  };
}

let registeredDisposables: vscode.Disposable[] = [];

afterEach(() => {
  for (const disposable of registeredDisposables) {
    disposable.dispose();
  }
  registeredDisposables = [];
});

describe('registerRefactoringCommandGroup', () => {
  test('registers and delegates every refactoring command', async () => {
    const dependencies = makeDependencies();
    registeredDisposables = registerRefactoringCommandGroup(dependencies);

    expect(registeredDisposables).toHaveLength(3);
    await vscode.commands.executeCommand('perl-lsp.extractVariable');
    await vscode.commands.executeCommand('perl-lsp.extractMethod');
    await vscode.commands.executeCommand('perl-lsp.showRefactoringOptions');

    expect(dependencies.extractVariable).toHaveBeenCalledTimes(1);
    expect(dependencies.extractMethod).toHaveBeenCalledTimes(1);
    expect(dependencies.showRefactoringOptions).toHaveBeenCalledTimes(1);
  });

  test('does not invoke feature callbacks during registration', () => {
    const dependencies = makeDependencies();
    registeredDisposables = registerRefactoringCommandGroup(dependencies);

    expect(dependencies.extractVariable).not.toHaveBeenCalled();
    expect(dependencies.extractMethod).not.toHaveBeenCalled();
    expect(dependencies.showRefactoringOptions).not.toHaveBeenCalled();
  });
});
