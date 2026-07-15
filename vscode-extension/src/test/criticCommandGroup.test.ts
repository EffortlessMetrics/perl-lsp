import * as vscode from 'vscode';
import { registerCriticCommandGroup, type CriticCommandContext } from '../criticCommandGroup';

function makeDependencies(): CriticCommandContext & {
  runPerlCriticOnActiveFile: jest.Mock;
  setPerlCriticSeverity: jest.Mock;
} {
  return {
    runPerlCriticOnActiveFile: jest.fn(async () => undefined),
    setPerlCriticSeverity: jest.fn(async () => undefined),
  };
}

describe('registerCriticCommandGroup', () => {
  test('registers and delegates both Critic commands', async () => {
    const dependencies = makeDependencies();
    const disposables = registerCriticCommandGroup(dependencies);

    expect(disposables).toHaveLength(2);
    await vscode.commands.executeCommand('perl-lsp.runPerlCritic');
    await vscode.commands.executeCommand('perl-lsp.setPerlCriticSeverity');

    expect(dependencies.runPerlCriticOnActiveFile).toHaveBeenCalledTimes(1);
    expect(dependencies.setPerlCriticSeverity).toHaveBeenCalledTimes(1);
  });

  test('does not invoke either feature while merely registering commands', () => {
    const dependencies = makeDependencies();

    registerCriticCommandGroup(dependencies);

    expect(dependencies.runPerlCriticOnActiveFile).not.toHaveBeenCalled();
    expect(dependencies.setPerlCriticSeverity).not.toHaveBeenCalled();
  });
});
