import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as vscode from 'vscode';
import {
  suggestAiCompletionIfSupported,
  suggestDiscoveredIncludePaths,
  validateIncludePaths,
  warnAboutPerlExtensionConflicts,
} from '../extensionWorkspaceGuidance';

const workspaceMock = vscode.workspace as unknown as { workspaceFolders: unknown };
const extensionsMock = vscode.extensions as unknown as { all: unknown[] };

function makeState(): { get: jest.Mock; update: jest.Mock } {
  const values = new Map<string, unknown>();
  return {
    get: jest.fn((key: string, defaultValue?: unknown) => values.get(key) ?? defaultValue),
    update: jest.fn(async (key: string, value: unknown) => {
      if (value === undefined) {
        values.delete(key);
      } else {
        values.set(key, value);
      }
    }),
  };
}

function mountWorkspace(workspaceDir: string, includePaths: string[]): void {
  workspaceMock.workspaceFolders = [
    {
      name: 'workspace',
      uri: { fsPath: workspaceDir, toString: () => `file://${workspaceDir}` },
    },
  ];
  (vscode.workspace.getConfiguration as jest.Mock).mockImplementation(() => ({
    get: jest.fn((key: string, defaultValue?: unknown) =>
      key === 'includePaths' ? includePaths : defaultValue,
    ),
    inspect: jest.fn(() => ({ defaultValue: ['lib', 'local/lib/perl5'] })),
    update: jest.fn(async () => undefined),
  }));
}

afterEach(() => {
  jest.clearAllMocks();
  workspaceMock.workspaceFolders = undefined;
  extensionsMock.all = [];
  (vscode.workspace.getConfiguration as jest.Mock).mockImplementation(() => ({
    get: jest.fn((_key: string, defaultValue?: unknown) => defaultValue),
    inspect: jest.fn(),
    update: jest.fn(),
  }));
});

test('does not prompt for absent built-in include paths', async () => {
  const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-guidance-default-'));
  mountWorkspace(workspaceDir, ['lib']);

  await validateIncludePaths({ globalState: makeState() } as unknown as vscode.ExtensionContext);

  expect(vscode.window.showWarningMessage).not.toHaveBeenCalled();
});

test('creates safe missing include directories inside the workspace', async () => {
  const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-guidance-create-'));
  mountWorkspace(workspaceDir, ['generated/perl']);
  (vscode.window.showWarningMessage as jest.Mock).mockResolvedValue('Create Missing Directories');

  await validateIncludePaths({ globalState: makeState() } as unknown as vscode.ExtensionContext);

  expect(fs.existsSync(path.join(workspaceDir, 'generated', 'perl'))).toBe(true);
  expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
    'Created 1 include directory: generated/perl.',
  );
});

test('does not offer or perform directory creation through a workspace symlink', async () => {
  const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-guidance-link-'));
  const outsideDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-guidance-outside-'));
  const link = path.join(workspaceDir, 'linked');
  try {
    fs.symlinkSync(outsideDir, link, 'dir');
  } catch {
    return;
  }
  mountWorkspace(workspaceDir, ['linked/escape']);
  (vscode.window.showWarningMessage as jest.Mock).mockResolvedValue('Create Missing Directories');

  await validateIncludePaths({ globalState: makeState() } as unknown as vscode.ExtensionContext);

  expect(vscode.window.showWarningMessage).toHaveBeenCalledWith(
    expect.any(String),
    'Open Settings',
  );
  expect(fs.existsSync(path.join(outsideDir, 'escape'))).toBe(false);
});

test('reports directory creation failures without aborting guidance', async () => {
  const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-guidance-mkdir-'));
  fs.writeFileSync(path.join(workspaceDir, 'blocked'), 'not a directory');
  mountWorkspace(workspaceDir, ['blocked/child']);
  const state = makeState();
  (vscode.window.showWarningMessage as jest.Mock).mockResolvedValue('Create Missing Directories');

  await validateIncludePaths({ globalState: state } as unknown as vscode.ExtensionContext);
  await validateIncludePaths({ globalState: state } as unknown as vscode.ExtensionContext);

  expect(vscode.window.showWarningMessage).toHaveBeenCalledWith(
    expect.stringContaining('Perl LSP: failed to create directory "blocked/child":'),
  );
  expect(
    (vscode.window.showWarningMessage as jest.Mock).mock.calls.filter(([message]) =>
      String(message).includes('configured include path "blocked/child"'),
    ),
  ).toHaveLength(2);
});

test('dismissal is sticky for an unchanged discovered module layout', async () => {
  const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-guidance-cache-'));
  fs.mkdirSync(path.join(workspaceDir, 'src'), { recursive: true });
  fs.writeFileSync(path.join(workspaceDir, 'src', 'Module.pm'), 'package Module; 1;\n');
  const globalState = makeState();
  mountWorkspace(workspaceDir, ['lib']);
  (vscode.window.showInformationMessage as jest.Mock).mockResolvedValue('Dismiss');

  await suggestDiscoveredIncludePaths({ globalState } as unknown as vscode.ExtensionContext);
  await suggestDiscoveredIncludePaths({ globalState } as unknown as vscode.ExtensionContext);

  expect(vscode.window.showInformationMessage).toHaveBeenCalledTimes(1);
});

test('retries a discovered-path suggestion after an update failure', async () => {
  const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-guidance-retry-'));
  fs.mkdirSync(path.join(workspaceDir, 'src'), { recursive: true });
  fs.writeFileSync(path.join(workspaceDir, 'src', 'Module.pm'), 'package Module; 1;\n');
  const globalState = makeState();
  const update = jest.fn(async () => {
    throw new Error('workspace is read-only');
  });
  workspaceMock.workspaceFolders = [
    {
      name: 'workspace',
      uri: { fsPath: workspaceDir, toString: () => `file://${workspaceDir}` },
    },
  ];
  (vscode.workspace.getConfiguration as jest.Mock).mockImplementation(() => ({
    get: jest.fn((_key: string, defaultValue?: unknown) => defaultValue),
    update,
  }));
  (vscode.window.showInformationMessage as jest.Mock).mockResolvedValue('Add to Include Paths');

  await suggestDiscoveredIncludePaths({ globalState } as unknown as vscode.ExtensionContext);
  await suggestDiscoveredIncludePaths({ globalState } as unknown as vscode.ExtensionContext);

  expect(update).toHaveBeenCalledTimes(2);
  expect(vscode.window.showInformationMessage).toHaveBeenCalledTimes(2);
  expect(globalState.update).not.toHaveBeenCalledWith(
    expect.stringContaining('perl-lsp.includePathsSuggestion.'),
    expect.any(String),
  );
});

test('adds discovered module directories and caches the resulting layout', async () => {
  const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-guidance-add-'));
  fs.mkdirSync(path.join(workspaceDir, 'src'), { recursive: true });
  fs.mkdirSync(path.join(workspaceDir, 'vendor'), { recursive: true });
  fs.writeFileSync(path.join(workspaceDir, 'src', 'Module.pm'), 'package Module; 1;\n');
  fs.writeFileSync(path.join(workspaceDir, 'vendor', 'Other.pm'), 'package Other; 1;\n');
  const globalState = makeState();
  const update = jest.fn(async () => undefined);
  workspaceMock.workspaceFolders = [
    {
      name: 'workspace',
      uri: { fsPath: workspaceDir, toString: () => `file://${workspaceDir}` },
    },
  ];
  (vscode.workspace.getConfiguration as jest.Mock).mockImplementation(() => ({
    get: jest.fn((_key: string, defaultValue?: unknown) => defaultValue),
    inspect: jest.fn(),
    update,
  }));
  (vscode.window.showInformationMessage as jest.Mock).mockResolvedValue('Add to Include Paths');

  await suggestDiscoveredIncludePaths({ globalState } as unknown as vscode.ExtensionContext);

  expect(update).toHaveBeenCalledWith(
    'includePaths',
    expect.arrayContaining(['src', 'vendor']),
    vscode.ConfigurationTarget.Workspace,
  );
  expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
    'Added src, vendor to perl-lsp.includePaths.',
  );
  expect(globalState.update).toHaveBeenCalledWith(
    expect.stringContaining('perl-lsp.includePathsSuggestion.'),
    expect.any(String),
  );
});

test('does not prompt for AI completion without a real server capability', async () => {
  (vscode.workspace.getConfiguration as jest.Mock).mockReturnValue({
    get: jest.fn(() => false),
    update: jest.fn(),
  });
  const workspaceState = makeState();

  await suggestAiCompletionIfSupported({ workspaceState } as unknown as vscode.ExtensionContext, {
    initializeResult: { capabilities: { hoverProvider: true } },
  });

  expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
});

test('offers AI completion when the server advertises inline completions', async () => {
  const update = jest.fn(async () => undefined);
  (vscode.workspace.getConfiguration as jest.Mock).mockReturnValue({
    get: jest.fn(() => false),
    update,
  });
  (vscode.window.showInformationMessage as jest.Mock).mockResolvedValue('Enable');
  const workspaceState = makeState();

  await suggestAiCompletionIfSupported({ workspaceState } as unknown as vscode.ExtensionContext, {
    initializeResult: { capabilities: { inlineCompletionProvider: {} } },
  });

  expect(update).toHaveBeenCalledWith(
    'aiCompletion.enabled',
    true,
    vscode.ConfigurationTarget.Workspace,
  );
  expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
    'AI-powered inline completions enabled.',
  );
  expect(workspaceState.update).toHaveBeenCalledWith(
    'perl-lsp.aiCompletion.firstRunNotificationShown',
    true,
  );
});

test('does not report the extension itself as a Perl conflict', async () => {
  extensionsMock.all = [
    {
      id: 'effortlessmetrics.perl-lsp-rs',
      packageJSON: {
        publisher: 'EffortlessMetrics',
        name: 'perl-lsp-rs',
        version: '0.12.3',
        contributes: { languages: [{ id: 'perl' }] },
      },
    },
  ];

  await warnAboutPerlExtensionConflicts({
    extension: {
      packageJSON: { publisher: 'EffortlessMetrics', name: 'perl-lsp-rs', version: '0.12.3' },
    },
    globalState: makeState(),
  } as unknown as vscode.ExtensionContext);

  expect(vscode.window.showWarningMessage).not.toHaveBeenCalled();
});

test('reports competing Perl extensions and opens the coexistence guide', async () => {
  extensionsMock.all = [
    {
      id: 'vendor.one',
      packageJSON: { displayName: 'Perl Tools', contributes: { languages: [{ id: 'perl' }] } },
    },
    { id: 'vendor.two', packageJSON: { displayName: 'Perl Critic', keywords: ['perlcritic'] } },
    { id: 'vendor.three', packageJSON: { name: 'perl-tidy' } },
    { id: 'vendor.four', packageJSON: { description: 'Another Perl navigator' } },
  ];
  (vscode.window.showWarningMessage as jest.Mock).mockResolvedValue('Open Coexistence Guide');
  const globalState = makeState();

  await warnAboutPerlExtensionConflicts({
    extension: {
      packageJSON: { publisher: 'EffortlessMetrics', name: 'perl-lsp-rs', version: '1.2.3' },
    },
    globalState,
  } as unknown as vscode.ExtensionContext);

  expect(vscode.window.showWarningMessage).toHaveBeenCalledWith(
    expect.stringContaining('detected 4 other Perl extensions'),
    'Open Coexistence Guide',
  );
  expect(vscode.window.showWarningMessage).toHaveBeenCalledWith(
    expect.stringContaining('(+1 more)'),
    'Open Coexistence Guide',
  );
  expect(vscode.env.openExternal).toHaveBeenCalledWith(
    expect.objectContaining({ toString: expect.any(Function) }),
  );
  expect(globalState.update).toHaveBeenCalledWith('perl-lsp.conflictWarningMajorVersion', '1');
});
