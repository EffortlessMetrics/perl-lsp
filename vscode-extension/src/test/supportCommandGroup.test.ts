import * as vscode from 'vscode';
import { registerSupportCommandGroup, type SupportCommandContext } from '../supportCommandGroup';

function makeDependencies(): SupportCommandContext & { reportIssue: jest.Mock } {
  return {
    reportIssue: jest.fn(async () => undefined),
  };
}

let registeredDisposables: vscode.Disposable[] = [];

afterEach(() => {
  for (const disposable of registeredDisposables) {
    disposable.dispose();
  }
  registeredDisposables = [];
});

describe('registerSupportCommandGroup', () => {
  test('registers and delegates the issue-reporting command', async () => {
    const dependencies = makeDependencies();
    registeredDisposables = registerSupportCommandGroup(dependencies);

    expect(registeredDisposables).toHaveLength(1);
    await vscode.commands.executeCommand('perl-lsp.reportIssue');

    expect(dependencies.reportIssue).toHaveBeenCalledTimes(1);
  });

  test('does not invoke the support callback during registration', () => {
    const dependencies = makeDependencies();
    registeredDisposables = registerSupportCommandGroup(dependencies);

    expect(dependencies.reportIssue).not.toHaveBeenCalled();
  });
});
