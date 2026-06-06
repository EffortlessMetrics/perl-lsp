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
  copyProviderDecisionReceiptCommand,
  diagnoseConfiguredServerPath,
  explainDiagnosticCommand,
  explainMissingModuleLookupCommand,
  explainProviderDecisionCommand,
  previewPackageRenameCommand,
  previewSafeDeleteCommand,
  showWorkspaceTrustReportCommand,
  validateIncludePaths,
  runPerlCriticOnActiveFile,
  setPerlCriticSeverity,
  syncPerlCriticConfiguration,
  warnAboutPerlExtensionConflicts,
  workspaceTrustClientRuntimeState,
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

describe('diagnoseConfiguredServerPath (perl-lsp.serverPath validation)', () => {
  function makeChannel(): any {
    return { appendLine: jest.fn() };
  }

  test('flags a configured serverPath that does not exist and logs a diagnostic', () => {
    const channel = makeChannel();
    const result = diagnoseConfiguredServerPath('/nonexistent/perllsp', false, channel);
    expect(result).toBe('/nonexistent/perllsp');
    expect(channel.appendLine).toHaveBeenCalledTimes(1);
    expect(channel.appendLine.mock.calls[0][0]).toContain('/nonexistent/perllsp');
    expect(channel.appendLine.mock.calls[0][0]).toContain('does not exist');
  });

  test('returns null and stays silent when the configured serverPath exists', () => {
    const channel = makeChannel();
    const result = diagnoseConfiguredServerPath('/usr/local/bin/perllsp', true, channel);
    expect(result).toBeNull();
    expect(channel.appendLine).not.toHaveBeenCalled();
  });

  test('returns null and stays silent when no serverPath is configured', () => {
    const channel = makeChannel();
    expect(diagnoseConfiguredServerPath(undefined, false, channel)).toBeNull();
    expect(diagnoseConfiguredServerPath('', false, channel)).toBeNull();
    expect(channel.appendLine).not.toHaveBeenCalled();
  });
});

describe('extension UX warnings', () => {
  afterEach(() => {
    jest.clearAllMocks();
    (vscode.window as any).activeTextEditor = undefined;
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
    // Both paths are explicit (not built-in defaults), so both are creatable.
    getConfiguration.mockImplementation(() => ({
      get: jest.fn(() => ['t/lib', 'vendor/perl']),
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

    expect(fs.existsSync(path.join(workspaceDir, 't/lib'))).toBe(true);
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

  test('does not offer directory creation for absolute include paths', async () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-ux-abs-'));
    const absoluteMissing = path.join(workspaceDir, 'missing-absolute-lib');
    const context = makeContext();
    context.globalState = {
      get: jest.fn(() => undefined),
      update: jest.fn(async () => undefined),
    };

    (vscode.workspace.getConfiguration as jest.Mock).mockImplementation(() => ({
      get: jest.fn(() => [absoluteMissing]),
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

    expect(showWarningMessage).toHaveBeenCalledWith(
      expect.stringContaining('absolute path'),
      'Open Settings'
    );
    expect(showWarningMessage.mock.calls[0]).toHaveLength(2);
  });

  test('does not offer directory creation for include paths outside the workspace', async () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-ux-traversal-'));
    const context = makeContext();
    context.globalState = {
      get: jest.fn(() => undefined),
      update: jest.fn(async () => undefined),
    };

    (vscode.workspace.getConfiguration as jest.Mock).mockImplementation(() => ({
      get: jest.fn(() => ['../outside-lib']),
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

    expect(showWarningMessage).toHaveBeenCalledWith(
      expect.stringContaining('../outside-lib'),
      'Open Settings'
    );
    expect(showWarningMessage.mock.calls[0]).toHaveLength(2);
    expect(fs.existsSync(path.resolve(workspaceDir, '../outside-lib'))).toBe(false);
  });

  // --- Origin-aware include-path validation -------------------------------
  //
  // Built-in default include paths (e.g. "lib", "local/lib/perl5") are
  // optional search hints: missing ones must NOT produce a user-facing
  // warning. Only explicitly-configured paths are expectations worth warning
  // about. See the include-path warning policy fix.

  // A configuration mock whose effective value is `paths` and whose built-in
  // default (via inspect) is the package.json default ["lib", "local/lib/perl5"].
  function mockIncludePathConfig(paths: string[]): void {
    (vscode.workspace.getConfiguration as jest.Mock).mockImplementation(() => ({
      get: jest.fn(() => paths),
      inspect: jest.fn(() => ({
        key: 'perl-lsp.includePaths',
        defaultValue: ['lib', 'local/lib/perl5'],
      })),
    }));
  }

  function setSingleWorkspace(workspaceDir: string): void {
    (vscode.workspace as any).workspaceFolders = [
      {
        name: 'workspace',
        uri: {
          fsPath: workspaceDir,
          toString: () => `file://${workspaceDir}`,
        },
      },
    ];
  }

  test('default_missing_lib_is_not_reported', async () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-default-lib-'));
    const context = makeContext();
    mockIncludePathConfig(['lib']);
    setSingleWorkspace(workspaceDir);

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    showWarningMessage.mockResolvedValue(undefined);

    await validateIncludePaths(context);

    expect(showWarningMessage).not.toHaveBeenCalled();
  });

  test('default_missing_local_lib_perl5_is_not_reported', async () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-default-local-'));
    const context = makeContext();
    mockIncludePathConfig(['local/lib/perl5']);
    setSingleWorkspace(workspaceDir);

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    showWarningMessage.mockResolvedValue(undefined);

    await validateIncludePaths(context);

    expect(showWarningMessage).not.toHaveBeenCalled();
  });

  test('explicit_missing_include_path_is_reported', async () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-explicit-'));
    const context = makeContext();
    mockIncludePathConfig(['vendor/lib']);
    setSingleWorkspace(workspaceDir);

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    showWarningMessage.mockResolvedValue(undefined);

    await validateIncludePaths(context);

    expect(showWarningMessage).toHaveBeenCalledWith(
      expect.stringContaining('vendor/lib'),
      'Open Settings',
      'Create Missing Directories'
    );
  });

  test('mixed_default_and_explicit_reports_only_explicit', async () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-mixed-'));
    const context = makeContext();
    // Neither path exists on disk; "lib" is a default hint, "vendor/lib" is explicit.
    mockIncludePathConfig(['lib', 'vendor/lib']);
    setSingleWorkspace(workspaceDir);

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    showWarningMessage.mockResolvedValue(undefined);

    await validateIncludePaths(context);

    expect(showWarningMessage).toHaveBeenCalledTimes(1);
    const [message] = showWarningMessage.mock.calls[0];
    expect(message).toContain('vendor/lib');
    expect(message).not.toContain('"lib"');
    // The suppressed default must not inflate the missing-path count.
    expect(message).not.toContain('include paths are missing');
  });

  test('create_missing_directories_creates_only_explicit_paths', async () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-create-explicit-only-'));
    const context = makeContext();
    mockIncludePathConfig(['lib', 'vendor/lib', 'local/lib/perl5']);
    setSingleWorkspace(workspaceDir);

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    showWarningMessage.mockResolvedValue('Create Missing Directories');

    await validateIncludePaths(context);

    expect(showWarningMessage).toHaveBeenCalledWith(
      expect.stringContaining('vendor/lib'),
      'Open Settings',
      'Create Missing Directories'
    );
    expect(fs.existsSync(path.join(workspaceDir, 'vendor/lib'))).toBe(true);
    expect(fs.existsSync(path.join(workspaceDir, 'lib'))).toBe(false);
    expect(fs.existsSync(path.join(workspaceDir, 'local/lib/perl5'))).toBe(false);
    expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
      expect.stringContaining('Created 1 include directory: vendor/lib.')
    );
  });

  test('existing_default_path_is_still_used', async () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-existing-default-'));
    fs.mkdirSync(path.join(workspaceDir, 'lib'), { recursive: true });
    const context = makeContext();
    mockIncludePathConfig(['lib']);
    setSingleWorkspace(workspaceDir);

    const showWarningMessage = vscode.window.showWarningMessage as jest.Mock;
    showWarningMessage.mockResolvedValue(undefined);

    await validateIncludePaths(context);

    // Present default path: no warning, and the directory is left untouched
    // so the server keeps resolving modules through it.
    expect(showWarningMessage).not.toHaveBeenCalled();
    expect(fs.existsSync(path.join(workspaceDir, 'lib'))).toBe(true);
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

  test('explains a provider decision through the LSP execute command', async () => {
    const sendRequest = jest.fn(async () => ({
      provider: 'goto_definition',
      decision: 'fallback',
      user_message: 'Goto definition used fallback.',
    }));
    (vscode.window as any).activeTextEditor = {
      document: {
        languageId: 'perl',
        uri: vscode.Uri.file('/workspace/lib/Foo.pm'),
      },
      selection: {
        active: { line: 12, character: 4 },
      },
    };

    await explainProviderDecisionCommand({ sendRequest } as any, 'goto_definition');

    expect(sendRequest).toHaveBeenCalledWith(
      'workspace/executeCommand',
      {
        command: 'perl.explainProviderDecision',
        arguments: [
          {
            provider: 'goto_definition',
            request_position: {
              uri_scheme: 'file',
              line: 12,
              character: 4,
            },
          },
        ],
      }
    );
    expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
      'Goto definition used fallback.',
      'Show Output'
    );
  });

  test('explains a diagnostic through the provider decision command', async () => {
    const requestReceipt = {
      provider: 'diagnostics',
      decision: 'acted',
      diagnostic_explanation: {
        schema_version: 'diagnostic_explanation.v1',
        diagnostic_explanations: [
          {
            code: 'PL701',
            trust_boundary: 'module_resolution',
          },
        ],
      },
    };
    const sendRequest = jest.fn(async () => ({
      provider: 'diagnostics',
      decision: 'acted',
      user_message: 'Diagnostic explanation is available.',
    }));

    await explainDiagnosticCommand(
      { sendRequest } as any,
      {
        provider: 'diagnostics',
        request_receipt: requestReceipt,
      }
    );

    expect(sendRequest).toHaveBeenCalledWith(
      'workspace/executeCommand',
      {
        command: 'perl.explainProviderDecision',
        arguments: [
          {
            provider: 'diagnostics',
            request_receipt: requestReceipt,
          },
        ],
      }
    );
    expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
      'Diagnostic explanation is available.',
      'Show Output'
    );
  });

  test('previews safe-delete through the no-edit LSP command', async () => {
    const sendRequest = jest.fn(async () => ({
      provider: 'safe_delete',
      decision: 'blocked',
      user_message: 'Safe delete refused. No edits were applied.',
    }));
    (vscode.window as any).activeTextEditor = {
      document: {
        languageId: 'perl',
        uri: vscode.Uri.file('/workspace/lib/Foo.pm'),
      },
      selection: {
        active: { line: 8, character: 2 },
      },
    };

    await previewSafeDeleteCommand({ sendRequest } as any);

    expect(sendRequest).toHaveBeenCalledWith(
      'workspace/executeCommand',
      {
        command: 'perl.previewSafeDelete',
        arguments: [
          {
            textDocument: { uri: 'file:///workspace/lib/Foo.pm' },
            position: { line: 8, character: 2 },
          },
        ],
      }
    );
    expect(vscode.window.showWarningMessage).toHaveBeenCalledWith(
      'Safe delete refused. No edits were applied.',
      'Show Output'
    );
  });

  test('previews package rename through the no-edit LSP command', async () => {
    const sendRequest = jest.fn(async () => ({
      provider: 'rename',
      decision: 'allowed',
      user_message: 'Package rename preview is available. No edits were applied.',
    }));
    (vscode.window.showInputBox as jest.Mock).mockResolvedValue('renamed_shared');
    (vscode.window as any).activeTextEditor = {
      document: {
        languageId: 'perl',
        uri: vscode.Uri.file('/workspace/lib/Foo.pm'),
        getText: jest.fn(() => 'shared'),
        getWordRangeAtPosition: jest.fn(() => ({ start: { line: 12, character: 4 }, end: { line: 12, character: 10 } })),
      },
      selection: {
        active: { line: 12, character: 4 },
        isEmpty: true,
      },
    };

    await previewPackageRenameCommand({ sendRequest } as any);

    expect(vscode.window.showInputBox).toHaveBeenCalledWith(
      expect.objectContaining({
        value: 'shared',
        placeHolder: 'renamed_symbol',
      })
    );
    expect(sendRequest).toHaveBeenCalledWith(
      'workspace/executeCommand',
      {
        command: 'perl.previewPackageRename',
        arguments: [
          {
            textDocument: { uri: 'file:///workspace/lib/Foo.pm' },
            position: { line: 12, character: 4 },
            newName: 'renamed_shared',
          },
        ],
      }
    );
    expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
      'Package rename preview is available. No edits were applied.',
      'Show Output'
    );
  });

  test('copies the provider decision bug-report payload', async () => {
    const sendRequest = jest.fn(async () => ({
      provider: 'safe_delete',
      decision: 'blocked',
      copyable_payload: {
        schema_version: 'provider_decision_bug_report.v1',
        provider: 'safe_delete',
        decision: 'blocked',
      },
    }));

    await copyProviderDecisionReceiptCommand({ sendRequest } as any, 'safe_delete');

    expect(sendRequest).toHaveBeenCalledWith(
      'workspace/executeCommand',
      expect.objectContaining({
        command: 'perl.explainProviderDecision',
        arguments: [
          expect.objectContaining({
            provider: 'safe_delete',
          }),
        ],
      })
    );
    const clipboardText = (vscode.env.clipboard.writeText as jest.Mock).mock.calls[0][0] as string;
    expect(JSON.parse(clipboardText)).toEqual({
      schema_version: 'provider_decision_bug_report.v1',
      provider: 'safe_delete',
      decision: 'blocked',
    });
    expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
      'Provider decision receipt copied.'
    );
  });

  test('summarizes launch configuration module paths without copying raw paths', () => {
    const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-launch-config-'));
    fs.mkdirSync(path.join(workspaceDir, '.vscode'), { recursive: true });
    fs.writeFileSync(path.join(workspaceDir, '.vscode', 'launch.json'), '{}');

    (vscode.workspace as any).workspaceFolders = [
      {
        name: 'workspace',
        uri: vscode.Uri.file(workspaceDir),
      },
    ];

    (vscode.workspace.getConfiguration as jest.Mock).mockImplementation((section?: string) => {
      if (section === 'launch') {
        return {
          get: jest.fn((key: string, defaultValue?: unknown) => {
            if (key !== 'configurations') {
              return defaultValue;
            }
            return [
              {
                type: 'perl',
                request: 'launch',
                name: 'Perl launch',
                program: '${workspaceFolder}/script/app.pl',
                cwd: 'script',
                perlPath: '/opt/perl/bin/perl',
                includePaths: ['${workspaceFolder}/lib', 'local/lib/perl5', 42],
              },
              {
                type: 'perl',
                request: 'attach',
                name: 'Perl attach',
                includePaths: ['t/lib'],
              },
              {
                type: 'node',
                request: 'launch',
                name: 'Ignored non-Perl launch',
                includePaths: ['node_modules'],
              },
            ];
          }),
        };
      }
      return {
        get: jest.fn((_key: string, defaultValue?: unknown) => defaultValue),
        has: jest.fn(() => false),
        inspect: jest.fn(),
        update: jest.fn(),
      };
    });

    const state = workspaceTrustClientRuntimeState();
    const dap = state.dap as Record<string, unknown>;
    const launchConfiguration = dap.launch_configuration as Record<string, unknown>;
    const includePathCounts = launchConfiguration.include_path_kind_counts as Record<string, number>;

    expect(dap.launch_json_workspace_count).toBe(1);
    expect(launchConfiguration.status).toBe('client_launch_config_reported');
    expect(launchConfiguration.configuration_count).toBe(3);
    expect(launchConfiguration.perl_configuration_count).toBe(2);
    expect(launchConfiguration.launch_request_count).toBe(1);
    expect(launchConfiguration.attach_request_count).toBe(1);
    expect(launchConfiguration.include_paths_configured_count).toBe(2);
    expect(launchConfiguration.include_path_entry_count).toBe(3);
    expect(launchConfiguration.non_string_include_path_count).toBe(1);
    expect(includePathCounts.workspace_variable).toBe(1);
    expect(includePathCounts.relative).toBe(2);
    expect(JSON.stringify(state)).not.toContain('/opt/perl/bin/perl');
    expect(JSON.stringify(state)).not.toContain('local/lib/perl5');
  });

  test('shows the workspace trust report in the trust output channel', async () => {
    const outputChannel = {
      appendLine: jest.fn(),
      show: jest.fn(),
      dispose: jest.fn(),
    };
    (vscode.window.createOutputChannel as jest.Mock).mockReturnValueOnce(outputChannel);
    const sendRequest = jest.fn(async () => ({
      schema_version: 'workspace_trust_report.v1',
      workspace: {
        root_path: '/workspace',
        workspace_folder_count: 1,
        open_document_count: 2,
      },
      module_resolution: {
        global_workspace_config: {
          include_paths: ['lib'],
          effective_include_paths: ['lib', 'local/lib/perl5'],
          system_inc_status: 'configured_not_probed_by_report',
          use_perl5lib: false,
          perl5lib_entry_count: 0,
          perl_path: '/usr/bin/perl',
        },
      },
      setup_hints: {
        status: 'advisory',
        hint_count: 1,
        hints: [
          {
            severity: 'info',
            message: 'PERL5LIB is not inherited by workspace module resolution.',
            action: 'Configure `perl.workspace.includePaths` for paths the editor should search.',
          },
        ],
        perl_binary: {
          resolution_status: 'configured_not_probed_by_report',
          version_status: 'not_probed_by_report',
        },
        perldoc: {
          status: 'oracle_contract_reported_not_run',
        },
        dap: {
          status: 'not_probed_by_lsp_workspace_report',
        },
        claim_boundary: 'Setup hints are derived from current configuration only.',
      },
      client_runtime_state: {
        source: 'vscode-extension',
        perldoc: {
          status: 'client_surface_registered',
        },
        dap: {
          status: 'client_state_reported',
          managed_adapter_exists: true,
          active_perl_debug_session: false,
          launch_json_workspace_count: 1,
          launch_configuration: {
            status: 'client_launch_config_reported',
            configuration_count: 2,
            perl_configuration_count: 1,
            include_paths_configured_count: 1,
            include_path_entry_count: 2,
            perl_path_configured_count: 1,
            claim_boundary: 'Launch configuration state reports counts and path classes only.',
          },
        },
      },
      index: {
        state: 'ready',
        availability: 'full',
        indexed_file_count: 42,
        indexed_symbol_count: 137,
      },
      providers: {
        support_tiers: {
          completion: 'partial-live-with-fallback',
          workspace_trust_report: 'partial-live-with-fallback',
        },
        decision_trace_count: 3,
      },
      dynamic_boundaries: {
        policy: 'Dynamic facts remain labeled.',
      },
      claim_boundary: 'Aggregates current runtime state only.',
    }));

    await showWorkspaceTrustReportCommand({ sendRequest } as any, () => ({
      schema_version: 'workspace_trust_client_runtime.v1',
      source: 'vscode-extension',
      perldoc: {
        status: 'client_surface_registered',
      },
      dap: {
        status: 'client_state_reported',
        managed_adapter_exists: true,
        active_perl_debug_session: false,
        launch_json_workspace_count: 1,
        launch_configuration: {
          status: 'client_launch_config_reported',
          configuration_count: 2,
          perl_configuration_count: 1,
          include_paths_configured_count: 1,
          include_path_entry_count: 2,
          perl_path_configured_count: 1,
          claim_boundary: 'Launch configuration state reports counts and path classes only.',
        },
      },
    }));

    expect(sendRequest).toHaveBeenCalledWith(
      'workspace/executeCommand',
      {
        command: 'perl.workspaceTrustReport',
        arguments: [{
          client_runtime_state: {
            schema_version: 'workspace_trust_client_runtime.v1',
            source: 'vscode-extension',
            perldoc: {
              status: 'client_surface_registered',
            },
            dap: {
              status: 'client_state_reported',
              managed_adapter_exists: true,
              active_perl_debug_session: false,
              launch_json_workspace_count: 1,
              launch_configuration: {
                status: 'client_launch_config_reported',
                configuration_count: 2,
                perl_configuration_count: 1,
                include_paths_configured_count: 1,
                include_path_entry_count: 2,
                perl_path_configured_count: 1,
                claim_boundary: 'Launch configuration state reports counts and path classes only.',
              },
            },
          },
        }],
      }
    );
    const rendered = outputChannel.appendLine.mock.calls
      .map((call: unknown[]) => call[0])
      .join('\n');
    expect(rendered).toContain('Perl LSP Trust Report');
    expect(rendered).toContain('Setup hints');
    expect(rendered).toContain('Perl binary: configured_not_probed_by_report');
    expect(rendered).toContain('perldoc: oracle_contract_reported_not_run');
    expect(rendered).toContain('DAP Perl: not_probed_by_lsp_workspace_report');
    expect(rendered).toContain('Client runtime state');
    expect(rendered).toContain('perldoc surface: client_surface_registered');
    expect(rendered).toContain('DAP adapter: client_state_reported');
    expect(rendered).toContain('DAP managed adapter exists: true');
    expect(rendered).toContain('DAP launch configs: 2');
    expect(rendered).toContain('DAP Perl configs: 1');
    expect(rendered).toContain('DAP includePaths entries: 2');
    expect(rendered).toContain('launch config boundary: Launch configuration state reports counts and path classes only.');
    expect(rendered).toContain('PERL5LIB is not inherited by workspace module resolution.');
    expect(rendered).toContain('Setup hints are derived from current configuration only.');
    expect(rendered).toContain('completion: partial-live-with-fallback');
    expect(rendered).toContain('Aggregates current runtime state only.');
    expect(outputChannel.show).toHaveBeenCalled();
  });

  test('explains a missing-module lookup through the LSP execute command', async () => {
    const outputChannel = {
      appendLine: jest.fn(),
      show: jest.fn(),
      dispose: jest.fn(),
    };
    (vscode.window.createOutputChannel as jest.Mock).mockReturnValueOnce(outputChannel);
    const sendRequest = jest.fn(async () => ({
      schema_version: 'missing_module_lookup_explanation.v1',
      requested_module: 'Missing::Payload',
      expected_relative_path: 'Missing/Payload.pm',
      module_resolution: {
        result: {
          status: 'not_found',
          why: 'No searched @INC candidate matched.',
        },
        effective_include_paths: [
          {
            path: 'lib',
            source: 'workspace includePaths',
            kind: 'workspace_relative',
            candidate_paths: [
              {
                path: '/workspace/lib/Missing/Payload.pm',
                exists: false,
              },
            ],
          },
        ],
        perl5lib_policy: 'enabled_but_environment_empty',
        use_system_inc: false,
      },
      user_message: 'Module Missing::Payload was not found in the current effective @INC state.',
      claim_boundary: 'explains one missing-module lookup only',
    }));
    (vscode.window as any).activeTextEditor = {
      document: {
        languageId: 'perl',
        uri: vscode.Uri.file('/workspace/script.pl'),
        getText: jest.fn(() => ''),
        lineAt: jest.fn(() => ({ text: 'use Missing::Payload;' })),
      },
      selection: {
        active: { line: 0, character: 8 },
      },
    };

    await explainMissingModuleLookupCommand({ sendRequest } as any);

    expect(sendRequest).toHaveBeenCalledWith(
      'workspace/executeCommand',
      {
        command: 'perl.explainMissingModuleLookup',
        arguments: [
          {
            module: 'Missing::Payload',
            textDocument: { uri: 'file:///workspace/script.pl' },
            position: { line: 0, character: 8 },
          },
        ],
      }
    );
    expect(vscode.window.showWarningMessage).toHaveBeenCalledWith(
      'Module Missing::Payload was not found in the current effective @INC state.',
      'Show Output'
    );
    const rendered = outputChannel.appendLine.mock.calls
      .map((call: unknown[]) => String(call[0]))
      .join('\n');
    expect(rendered).toContain('Perl LSP Missing Module Lookup');
    expect(rendered).toContain('Missing::Payload');
    expect(rendered).toContain('workspace includePaths');
    expect(rendered).toContain('Raw lookup JSON');
  });
});
