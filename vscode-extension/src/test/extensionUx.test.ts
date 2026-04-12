/**
 * Focused UX contract tests for extension startup warnings.
 */

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as vscode from 'vscode';
jest.mock('vscode-languageclient/node', () => ({
  LanguageClient: class {},
  Trace: {
    Off: 'off',
    Messages: 'messages',
    Verbose: 'verbose',
  },
  TransportKind: {
    stdio: 0,
  },
}));
import {
  validateIncludePaths,
  runPerlCriticOnActiveFile,
  setPerlCriticSeverity,
  syncPerlCriticConfiguration,
  warnAboutPerlExtensionConflicts,
} from '../extension';

function makeContext(version = '0.12.3'): any {
  return {
    extension: {
      packageJSON: {
        publisher: 'EffortlessMetrics',
        name: 'perl-lsp-rs',
        version,
      },
    },
    globalState: {
      get: jest.fn(() => undefined),
      update: jest.fn(async () => undefined),
    },
  };
}

describe('extension UX warnings', () => {
  afterEach(() => {
    jest.clearAllMocks();
    (vscode.workspace as any).workspaceFolders = undefined;
    (vscode.extensions as any).all = [];
    (vscode.workspace.getConfiguration as jest.Mock).mockImplementation(
      (section?: string) => ({
        get: jest.fn((key: string, defaultValue?: any) => defaultValue),
        has: jest.fn(() => false),
        inspect: jest.fn(),
        update: jest.fn(),
      })
    );
    (vscode.window.showWarningMessage as jest.Mock).mockImplementation(async () => undefined);
  });

  test('warns once for missing include paths and offers settings', async () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-ux-'));
    fs.mkdirSync(path.join(workspaceDir, 'lib'), { recursive: true });

    const context = makeContext();
    let warnedSignature: string | undefined;
    let includePaths = ['lib', 'src/libx'];
    const globalState = {
      get: jest.fn(() => warnedSignature),
      update: jest.fn(async (_key: string, value: string | undefined) => {
        warnedSignature = value;
      }),
    };
    context.globalState = globalState;

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    showWarningMessage.mockResolvedValue(undefined);

    const getConfiguration = vscode.workspace.getConfiguration as jest.Mock;
    getConfiguration.mockImplementation(() => ({
      get: jest.fn(() => includePaths),
    }));

    (vscode.workspace as any).workspaceFolders = [
      {
        name: 'workspace',
        uri: {
          fsPath: workspaceDir,
          toString: () => `file://${workspaceDir}`,
        },
      },
    ];

    await validateIncludePaths(context);

    expect(showWarningMessage).toHaveBeenCalledWith(
      expect.stringContaining('src/libx'),
      'Open Settings'
    );
    expect(globalState.update).toHaveBeenCalledWith(
      expect.stringContaining('perl-lsp.includePathsWarning.'),
      'src/libx'
    );

    showWarningMessage.mockClear();
    await validateIncludePaths(context);
    expect(showWarningMessage).not.toHaveBeenCalled();

    includePaths = ['lib', 'vendorx'];
    await validateIncludePaths(context);
    expect(showWarningMessage).toHaveBeenCalledWith(
      expect.stringContaining('vendorx'),
      'Open Settings'
    );
  });

  test('warns once per major version when conflicting Perl extensions are installed', async () => {
    const context = makeContext('0.12.3');
    let warnedMajor: string | undefined;
    context.globalState = {
      get: jest.fn(() => warnedMajor),
      update: jest.fn(async (_key: string, value: string) => {
        warnedMajor = value;
      }),
    };

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    showWarningMessage.mockResolvedValue(undefined);

    (vscode.extensions as any).all = [
      {
        id: 'EffortlessMetrics.perl-lsp-rs',
        packageJSON: {
          publisher: 'EffortlessMetrics',
          name: 'perl-lsp-rs',
          version: '0.12.3',
        },
      },
      {
        id: 'example.perl-navigator',
        packageJSON: {
          displayName: 'Perl Navigator',
          version: '1.0.0',
          contributes: {
            languages: [{ id: 'perl' }],
          },
        },
      },
    ];

    await warnAboutPerlExtensionConflicts(context);
    expect(showWarningMessage).toHaveBeenCalledWith(
      expect.stringContaining('Perl Navigator'),
      'Open Coexistence Guide'
    );

    showWarningMessage.mockClear();
    await warnAboutPerlExtensionConflicts(context);
    expect(showWarningMessage).not.toHaveBeenCalled();
  });

  test('syncs perlcritic settings to the server', async () => {
    const sendNotification = jest.fn();
    const getConfiguration = vscode.workspace.getConfiguration as jest.Mock;
    getConfiguration.mockImplementation(() => ({
      get: jest.fn((key: string, defaultValue?: any) => {
        switch (key) {
          case 'perlcritic.enabled':
            return true;
          case 'perlcritic.severity':
            return 5;
          case 'perlcritic.profile':
            return '/tmp/.perlcriticrc';
          case 'perlcritic.theme':
            return 'classic';
          default:
            return defaultValue;
        }
      }),
      has: jest.fn(() => false),
      inspect: jest.fn((key: string) => {
        switch (key) {
          case 'perlcritic.enabled':
            return { workspaceValue: true };
          case 'perlcritic.severity':
            return { workspaceValue: 5 };
          case 'perlcritic.profile':
            return { workspaceValue: '/tmp/.perlcriticrc' };
          default:
            return undefined;
        }
      }),
      update: jest.fn(),
    }));

    await syncPerlCriticConfiguration({ sendNotification } as any, vscode.Uri.file('/tmp/example.pl'));

    expect(sendNotification).toHaveBeenCalledWith(
      'workspace/didChangeConfiguration',
      expect.objectContaining({
        settings: expect.objectContaining({
          perl: expect.objectContaining({
            perlcritic: expect.objectContaining({
              enabled: true,
              severity: 5,
              profile: '/tmp/.perlcriticrc',
            }),
          }),
        }),
      })
    );
  });

  test('does not sync perlcritic defaults when nothing is explicitly configured', async () => {
    const sendNotification = jest.fn();
    (vscode.workspace.getConfiguration as jest.Mock).mockImplementation(() => ({
      get: jest.fn((key: string, defaultValue?: any) => defaultValue),
      has: jest.fn(() => false),
      inspect: jest.fn(() => undefined),
      update: jest.fn(),
    }));

    await syncPerlCriticConfiguration({ sendNotification } as any, vscode.Uri.file('/tmp/example.pl'));
    expect(sendNotification).not.toHaveBeenCalled();
  });

  test('runs perlcritic on the active Perl file', async () => {
    const sendRequest = jest.fn(async () => ({
      status: 'success',
      violationCount: 2,
      analyzerUsed: 'external',
      violations: [{}, {}],
    }));
    const activeTextEditor = {
      document: {
        languageId: 'perl',
        isDirty: false,
        uri: vscode.Uri.file('/workspace/lib/Foo.pm'),
        save: jest.fn(async () => undefined),
      },
    };
    (vscode.window as any).activeTextEditor = activeTextEditor;

    await runPerlCriticOnActiveFile({ sendRequest, sendNotification: jest.fn() } as any);

    expect(sendRequest).toHaveBeenCalledWith(
      'workspace/executeCommand',
      expect.objectContaining({
        command: 'perl.runCritic',
        arguments: ['file:///workspace/lib/Foo.pm'],
      })
    );
    expect(vscode.window.showWarningMessage).toHaveBeenCalledWith(
      expect.stringContaining('PerlCritic found 2 issues in Foo.pm.'),
      'Show Output'
    );
  });

  test('sets perlcritic severity and syncs it to the server', async () => {
    const sendNotification = jest.fn();
    const sendRequest = jest.fn();
    const showQuickPick = vscode.window.showQuickPick as jest.Mock;
    showQuickPick.mockResolvedValue({ label: '4', description: 'Strict' });

    const update = jest.fn(async () => undefined);
    (vscode.workspace.getConfiguration as jest.Mock).mockImplementation(() => ({
      get: jest.fn((key: string, defaultValue?: any) => defaultValue),
      has: jest.fn(() => false),
      inspect: jest.fn(),
      update,
    }));

    await setPerlCriticSeverity({ sendNotification, sendRequest } as any);

    expect(update).toHaveBeenCalledWith('perlcritic.severity', 4, vscode.ConfigurationTarget.Global);
    expect(sendNotification).toHaveBeenCalledWith(
      'workspace/didChangeConfiguration',
      expect.objectContaining({
        settings: expect.objectContaining({
          perl: expect.objectContaining({
            perlcritic: expect.objectContaining({
              severity: 4,
            }),
          }),
        }),
      })
    );
    expect(vscode.window.showInformationMessage).toHaveBeenCalledWith('PerlCritic severity set to 4.');
  });
});
