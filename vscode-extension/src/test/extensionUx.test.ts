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
  explainDiagnosticCommand,
  explainMissingModuleLookupCommand,
  explainProviderDecisionCommand,
  previewSafeDeleteCommand,
  showWorkspaceTrustReportCommand,
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
          status: 'not_probed_by_report',
        },
        dap: {
          status: 'not_probed_by_lsp_workspace_report',
        },
        claim_boundary: 'Setup hints are derived from current configuration only.',
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

    await showWorkspaceTrustReportCommand({ sendRequest } as any);

    expect(sendRequest).toHaveBeenCalledWith(
      'workspace/executeCommand',
      {
        command: 'perl.workspaceTrustReport',
        arguments: [],
      }
    );
    const rendered = outputChannel.appendLine.mock.calls
      .map((call: unknown[]) => call[0])
      .join('\n');
    expect(rendered).toContain('Perl LSP Trust Report');
    expect(rendered).toContain('Setup hints');
    expect(rendered).toContain('Perl binary: configured_not_probed_by_report');
    expect(rendered).toContain('perldoc: not_probed_by_report');
    expect(rendered).toContain('DAP Perl: not_probed_by_lsp_workspace_report');
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
