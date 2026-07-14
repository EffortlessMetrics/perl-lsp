import * as vscode from 'vscode';
import {
  registerWorkspaceConfigurationEvents,
  type WorkspaceEventHandlers,
} from '../extensionWorkspaceEvents';

describe('workspace configuration event routing', () => {
  test('dispatches each configuration class once', () => {
    let listener: ((event: vscode.ConfigurationChangeEvent) => void) | undefined;
    (vscode.workspace.onDidChangeConfiguration as jest.Mock).mockImplementation((callback) => {
      listener = callback;
      return { dispose: jest.fn() };
    });

    const handlers: WorkspaceEventHandlers = {
      onLiveConfigurationChanged: jest.fn(),
      onReconstructConfigurationChanged: jest.fn(),
      onRestartRequired: jest.fn(),
    };
    const disposable = registerWorkspaceConfigurationEvents(handlers);

    listener?.({
      affectsConfiguration: (setting: string) =>
        new Set([
          'perl-lsp.includePaths',
          'perl-lsp.aiCompletion.enabled',
          'perl-lsp.featureProfile',
        ]).has(setting),
    } as vscode.ConfigurationChangeEvent);

    expect(handlers.onLiveConfigurationChanged).toHaveBeenCalledTimes(1);
    expect(handlers.onReconstructConfigurationChanged).toHaveBeenCalledTimes(1);
    expect(handlers.onRestartRequired).toHaveBeenCalledTimes(1);
    expect(disposable).toBeDefined();
  });

  test('routes unrelated changes nowhere', () => {
    let listener: ((event: vscode.ConfigurationChangeEvent) => void) | undefined;
    (vscode.workspace.onDidChangeConfiguration as jest.Mock).mockImplementation((callback) => {
      listener = callback;
      return { dispose: jest.fn() };
    });

    const handlers: WorkspaceEventHandlers = {
      onLiveConfigurationChanged: jest.fn(),
      onReconstructConfigurationChanged: jest.fn(),
      onRestartRequired: jest.fn(),
    };
    registerWorkspaceConfigurationEvents(handlers);

    listener?.({
      affectsConfiguration: (setting: string) => setting === 'perl-lsp.autoPopulateNewFiles',
    } as vscode.ConfigurationChangeEvent);

    expect(handlers.onLiveConfigurationChanged).not.toHaveBeenCalled();
    expect(handlers.onReconstructConfigurationChanged).not.toHaveBeenCalled();
    expect(handlers.onRestartRequired).not.toHaveBeenCalled();
  });

  test('reports synchronous handler failures without escaping the event listener', async () => {
    let listener: ((event: vscode.ConfigurationChangeEvent) => void) | undefined;
    (vscode.workspace.onDidChangeConfiguration as jest.Mock).mockImplementation((callback) => {
      listener = callback;
      return { dispose: jest.fn() };
    });

    const onError = jest.fn();
    registerWorkspaceConfigurationEvents({
      onLiveConfigurationChanged: () => {
        throw new Error('sync handler failed');
      },
      onReconstructConfigurationChanged: jest.fn(),
      onRestartRequired: jest.fn(),
      onError,
    });

    listener?.({
      affectsConfiguration: (setting: string) => setting === 'perl-lsp.includePaths',
    } as vscode.ConfigurationChangeEvent);

    expect(onError).toHaveBeenCalledWith(expect.any(Error));
  });

  test('uses stderr when no error handler is supplied', () => {
    let listener: ((event: vscode.ConfigurationChangeEvent) => void) | undefined;
    (vscode.workspace.onDidChangeConfiguration as jest.Mock).mockImplementation((callback) => {
      listener = callback;
      return { dispose: jest.fn() };
    });
    const stderrWrite = jest.spyOn(process.stderr, 'write').mockImplementation(() => true);

    registerWorkspaceConfigurationEvents({
      onLiveConfigurationChanged: () => {
        throw new Error('unreported handler failed');
      },
      onReconstructConfigurationChanged: jest.fn(),
      onRestartRequired: jest.fn(),
    });

    listener?.({
      affectsConfiguration: (setting: string) => setting === 'perl-lsp.includePaths',
    } as vscode.ConfigurationChangeEvent);

    expect(stderrWrite).toHaveBeenCalledWith(
      expect.stringContaining(
        '[workspace-configuration] handler failed: Error: unreported handler failed',
      ),
    );
    stderrWrite.mockRestore();
  });

  test('reports rejected async handlers', async () => {
    let listener: ((event: vscode.ConfigurationChangeEvent) => void) | undefined;
    (vscode.workspace.onDidChangeConfiguration as jest.Mock).mockImplementation((callback) => {
      listener = callback;
      return { dispose: jest.fn() };
    });

    const onError = jest.fn();
    registerWorkspaceConfigurationEvents({
      onLiveConfigurationChanged: async () => {
        throw new Error('async handler failed');
      },
      onReconstructConfigurationChanged: jest.fn(),
      onRestartRequired: jest.fn(),
      onError,
    });

    listener?.({
      affectsConfiguration: (setting: string) => setting === 'perl-lsp.includePaths',
    } as vscode.ConfigurationChangeEvent);
    await Promise.resolve();

    expect(onError).toHaveBeenCalledWith(expect.any(Error));
  });
});
