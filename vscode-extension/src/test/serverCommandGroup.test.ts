import * as vscode from 'vscode';
import { registerServerCommandGroup, type ServerCommandContext } from '../serverCommandGroup';
import { HealthCheckStatus, type HealthCheckResult } from '../onboarding';

const outputChannel = {
  appendLine: jest.fn(),
  show: jest.fn(),
} as unknown as vscode.LogOutputChannel;

function makeDependencies(results: HealthCheckResult[] = []): ServerCommandContext & {
  reinstallServerBinary: jest.Mock;
  restartServer: jest.Mock;
  runHealthCheck: jest.Mock;
} {
  return {
    outputChannel,
    currentServerPath: jest.fn(() => '/configured/perllsp'),
    reinstallServerBinary: jest.fn(async () => ({
      ok: true,
      serverPath: '/installed/perllsp',
      target: 'x86_64-unknown-linux-gnu',
      source: 'existing' as const,
    })),
    restartServer: jest.fn(async () => undefined),
    runHealthCheck: jest.fn(async () => results),
  };
}

beforeEach(() => {
  jest.clearAllMocks();
});

describe('registerServerCommandGroup', () => {
  test('registers server commands and delegates without owning lifecycle state', async () => {
    const dependencies = makeDependencies();
    const disposables = registerServerCommandGroup(dependencies);

    expect(disposables).toHaveLength(4);
    await vscode.commands.executeCommand('perl-lsp.showOutput');
    await vscode.commands.executeCommand('perl-lsp.reinstall');
    await vscode.commands.executeCommand('perl-lsp.restart');

    expect(outputChannel.show).toHaveBeenCalledTimes(1);
    expect(dependencies.reinstallServerBinary).toHaveBeenCalledTimes(1);
    expect(dependencies.restartServer).toHaveBeenCalledTimes(1);
  });

  test('uses the projected path by default and returns structured health results', async () => {
    const dependencies = makeDependencies([
      {
        label: 'Perl interpreter',
        ok: true,
        status: HealthCheckStatus.Ok,
        detail: 'Perl 5.40',
      },
      {
        label: 'LSP binary',
        ok: false,
        status: HealthCheckStatus.Warning,
        detail: 'using a configured fallback',
      },
    ]);
    registerServerCommandGroup(dependencies);

    const result = await vscode.commands.executeCommand('perl-lsp.runHealthCheck');

    expect(dependencies.currentServerPath).toHaveBeenCalledTimes(1);
    expect(dependencies.runHealthCheck).toHaveBeenCalledWith('/configured/perllsp');
    expect(result).toEqual({
      ok: true,
      checks: [
        { label: 'Perl interpreter', status: 'ok', detail: 'Perl 5.40' },
        { label: 'LSP binary', status: 'warning', detail: 'using a configured fallback' },
      ],
    });
    expect(outputChannel.appendLine).toHaveBeenCalledWith('[health-check] Results:');
  });

  test('preserves an explicit null path and reports health errors', async () => {
    const dependencies = makeDependencies([
      {
        label: 'LSP binary',
        ok: false,
        status: HealthCheckStatus.Error,
        detail: 'missing',
      },
    ]);
    registerServerCommandGroup(dependencies);

    const result = await vscode.commands.executeCommand('perl-lsp.runHealthCheck', null);

    expect(dependencies.currentServerPath).not.toHaveBeenCalled();
    expect(dependencies.runHealthCheck).toHaveBeenCalledWith(null);
    expect(result).toEqual({
      ok: false,
      checks: [{ label: 'LSP binary', status: 'error', detail: 'missing' }],
    });
    expect(vscode.window.showErrorMessage).toHaveBeenCalledWith(
      'Health check failed: LSP binary',
      'Show Output',
    );
  });
});
