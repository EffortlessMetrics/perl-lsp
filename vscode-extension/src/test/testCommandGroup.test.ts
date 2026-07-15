import * as vscode from 'vscode';
import { registerTestCommandGroup, type TestCommandContext } from '../testCommandGroup';

function makeDependencies(): TestCommandContext & {
  runTests: jest.Mock;
  runCurrentTest: jest.Mock;
  runTestAtCursor: jest.Mock;
  runAllTests: jest.Mock;
} {
  return {
    runTests: jest.fn(async () => undefined),
    runCurrentTest: jest.fn(async () => undefined),
    runTestAtCursor: jest.fn(async () => undefined),
    runAllTests: jest.fn(async () => undefined),
  };
}

describe('registerTestCommandGroup', () => {
  test('registers and delegates the complete test command surface', async () => {
    const dependencies = makeDependencies();
    const disposables = registerTestCommandGroup(dependencies);

    expect(disposables).toHaveLength(4);
    await vscode.commands.executeCommand('perl-lsp.runTests', { program: '/tmp/example.t' });
    await vscode.commands.executeCommand('perl-lsp.runCurrentTest');
    await vscode.commands.executeCommand('perl-lsp.runTestAtCursor');
    await vscode.commands.executeCommand('perl-lsp.runAllTests');

    expect(dependencies.runTests).toHaveBeenCalledWith({ program: '/tmp/example.t' });
    expect(dependencies.runCurrentTest).toHaveBeenCalledTimes(1);
    expect(dependencies.runTestAtCursor).toHaveBeenCalledTimes(1);
    expect(dependencies.runAllTests).toHaveBeenCalledTimes(1);
  });

  test('does not run test features while only composing commands', () => {
    const dependencies = makeDependencies();

    registerTestCommandGroup(dependencies);

    expect(dependencies.runTests).not.toHaveBeenCalled();
    expect(dependencies.runCurrentTest).not.toHaveBeenCalled();
    expect(dependencies.runTestAtCursor).not.toHaveBeenCalled();
    expect(dependencies.runAllTests).not.toHaveBeenCalled();
  });
});
