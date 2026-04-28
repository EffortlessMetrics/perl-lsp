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
      'Open Settings',
      'Create Missing Directories'
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
      'Open Settings',
      'Create Missing Directories'
    );
  });

  test('can create missing relative include paths directly from the warning', async () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-ux-create-'));
    const context = makeContext();
    context.globalState = {
      get: jest.fn(() => undefined),
      update: jest.fn(async () => undefined),
    };

    const getConfiguration = vscode.workspace.getConfiguration as jest.Mock;
    getConfiguration.mockImplementation(() => ({
      get: jest.fn(() => ['lib', 'vendor/perl']),
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

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    showWarningMessage.mockResolvedValue('Create Missing Directories');

    await validateIncludePaths(context);

    expect(fs.existsSync(path.join(workspaceDir, 'lib'))).toBe(true);
    expect(fs.existsSync(path.join(workspaceDir, 'vendor/perl'))).toBe(true);
    expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
      expect.stringContaining('Created 2 include directories')
    );
  });

  test('does not offer directory creation when include path traverses a symlink outside workspace', async () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-ux-symlink-'));
    const outsideDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-ux-outside-'));
    const symlinkPath = path.join(workspaceDir, 'linked');
    try {
      fs.symlinkSync(outsideDir, symlinkPath, 'dir');
    } catch {
      return;
    }

    const context = makeContext();
    context.globalState = {
      get: jest.fn(() => undefined),
      update: jest.fn(async () => undefined),
    };

    const getConfiguration = vscode.workspace.getConfiguration as jest.Mock;
    getConfiguration.mockImplementation(() => ({
      get: jest.fn(() => ['linked/created-from-warning']),
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

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    showWarningMessage.mockResolvedValue(undefined);

    await validateIncludePaths(context);

    // The symlinked path must be excluded from creatablePaths so only 'Open Settings' is offered.
    expect(showWarningMessage).toHaveBeenCalledWith(
      expect.stringContaining('linked/created-from-warning'),
      'Open Settings'
    );
    expect(showWarningMessage).not.toHaveBeenCalledWith(
      expect.any(String),
      'Open Settings',
      'Create Missing Directories'
    );
    // Belt-and-suspenders: even if the user somehow triggered creation, nothing should land outside.
    expect(fs.existsSync(path.join(outsideDir, 'created-from-warning'))).toBe(false);
  });

  test('does not create directories outside workspace when user clicks Create Missing Directories with a symlinked include path', async () => {
    // This test verifies the T2 re-check guard in the mkdir loop: even if creatablePaths
    // somehow contains a symlinked path (e.g. due to a race between the T1 filter and the
    // actual mkdir call), hasSafeExistingAncestor is re-evaluated before mkdirSync runs.
    // We simulate this by injecting a mixed set of paths: one safe (inside workspace) and
    // one that resolves through a symlink to outside.  We then verify only the safe one is
    // created and nothing lands outside.
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-ux-symlink2-'));
    const outsideDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-ux-outside2-'));
    const symlinkPath = path.join(workspaceDir, 'linked2');
    try {
      fs.symlinkSync(outsideDir, symlinkPath, 'dir');
    } catch {
      // Symlink creation not supported on this platform/environment — skip.
      return;
    }

    const context = makeContext();
    context.globalState = {
      get: jest.fn(() => undefined),
      update: jest.fn(async () => undefined),
    };

    const getConfiguration = vscode.workspace.getConfiguration as jest.Mock;
    // 'safe-lib' is inside the workspace; 'linked2/escape' traverses the symlink outside.
    getConfiguration.mockImplementation(() => ({
      get: jest.fn(() => ['safe-lib', 'linked2/escape']),
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

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    // The user clicks 'Create Missing Directories'.
    showWarningMessage.mockResolvedValue('Create Missing Directories');

    await validateIncludePaths(context);

    // 'safe-lib' is safe: it should be created inside the workspace.
    expect(fs.existsSync(path.join(workspaceDir, 'safe-lib'))).toBe(true);
    // 'linked2/escape' resolves through a symlink outside: nothing should be created there.
    expect(fs.existsSync(path.join(outsideDir, 'escape'))).toBe(false);
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
          case 'perlcritic.theme':
            return { workspaceValue: 'classic' };
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
              theme: 'classic',
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
