import * as vscode from 'vscode';
import {
  explainDiagnosticCommand,
  showWorkspaceTrustReportCommand,
  type LspExecuteCommandClient,
} from '../diagnosticCommands';

function makeOutputChannel(): vscode.OutputChannel & {
  appendLine: jest.Mock;
  show: jest.Mock;
} {
  return {
    appendLine: jest.fn(),
    show: jest.fn(),
    dispose: jest.fn(),
  } as unknown as vscode.OutputChannel & {
    appendLine: jest.Mock;
    show: jest.Mock;
  };
}

afterEach(() => {
  jest.clearAllMocks();
  (vscode.window as unknown as { activeTextEditor: unknown }).activeTextEditor = undefined;
});

describe('diagnostic command implementation boundary', () => {
  test('uses the injected unavailable-state message without importing extension lifecycle state', async () => {
    await explainDiagnosticCommand(
      undefined,
      { provider: 'diagnostics' },
      {
        serverNotRunningMessage: () => 'The language server is still starting.',
      },
    );

    expect(vscode.window.showWarningMessage).toHaveBeenCalledWith(
      'The language server is still starting.',
    );
  });

  test('uses the injected runtime-state supplier and output channel', async () => {
    const outputChannel = makeOutputChannel();
    const runtimeState = {
      schema_version: 'workspace_trust_client_runtime.v1',
      source: 'vscode-extension',
    };
    const sendRequest = jest.fn(async () => ({
      schema_version: 'workspace_trust_report.v1',
      workspace: { workspace_folder_count: 1, open_document_count: 0 },
      client_runtime_state: runtimeState,
    }));

    await showWorkspaceTrustReportCommand(
      { sendRequest } as unknown as LspExecuteCommandClient,
      () => runtimeState,
      { outputChannel },
    );

    expect(sendRequest).toHaveBeenCalledWith('workspace/executeCommand', {
      command: 'perl.workspaceTrustReport',
      arguments: [{ client_runtime_state: runtimeState }],
    });
    expect(outputChannel.appendLine).toHaveBeenCalledWith(
      expect.stringContaining('Perl LSP Trust Report'),
    );
    expect(outputChannel.appendLine).toHaveBeenCalledWith(
      expect.stringContaining('workspace_trust_report.v1'),
    );
    expect(outputChannel.show).toHaveBeenCalledTimes(1);
  });

  test('reports execute-command failures through the VS Code error surface', async () => {
    const sendRequest = jest.fn(async () => {
      throw new Error('request failed');
    });

    await explainDiagnosticCommand({ sendRequest } as unknown as LspExecuteCommandClient, {
      provider: 'diagnostics',
    });

    expect(vscode.window.showErrorMessage).toHaveBeenCalledWith(
      'Perl LSP command failed: request failed',
    );
  });
});
