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
    let warned = false;
    const globalState = {
      get: jest.fn(() => warned),
      update: jest.fn(async (_key: string, value: boolean) => {
        warned = value;
      }),
    };
    context.globalState = globalState;

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    showWarningMessage.mockResolvedValue(undefined);

    const getConfiguration = vscode.workspace.getConfiguration as jest.Mock;
    getConfiguration.mockImplementation(() => ({
      get: jest.fn(() => ['lib', 'src/libx']),
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
      true
    );

    showWarningMessage.mockClear();
    await validateIncludePaths(context);
    expect(showWarningMessage).not.toHaveBeenCalled();
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
});
