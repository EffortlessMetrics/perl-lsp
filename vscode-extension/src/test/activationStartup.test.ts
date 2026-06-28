import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as vscode from 'vscode';

const mockLanguageClientStart = jest.fn(() => new Promise<void>(() => undefined));
const mockLanguageClientStop = jest.fn(async () => undefined);
const mockLanguageClientSetTrace = jest.fn(async () => undefined);
const mockLanguageClientOnDidChangeState = jest.fn(() => ({ dispose: jest.fn() }));

jest.mock('vscode-languageclient/node', () => ({
  LanguageClient: jest.fn().mockImplementation(() => ({
    initializeResult: { capabilities: {} },
    onDidChangeState: mockLanguageClientOnDidChangeState,
    setTrace: mockLanguageClientSetTrace,
    start: mockLanguageClientStart,
    stop: mockLanguageClientStop,
  })),
  Trace: { Off: 'off', Messages: 'messages', Verbose: 'verbose' },
  TransportKind: { stdio: 0 },
}));

import { activate, deactivate } from '../extension';

function delay(ms: number): Promise<'timeout'> {
  return new Promise(resolve => {
    setTimeout(() => resolve('timeout'), ms);
  });
}

async function waitUntil(condition: () => boolean, timeoutMs: number): Promise<void> {
  const startedAt = Date.now();
  while (!condition()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('condition not met before timeout');
    }
    await new Promise(resolve => {
      setTimeout(resolve, 10);
    });
  }
}

function makeContext(extensionPath: string): vscode.ExtensionContext {
  const state = {
    get: jest.fn(() => undefined),
    update: jest.fn(async () => undefined),
  };

  return {
    extension: {
      packageJSON: {
        publisher: 'EffortlessMetrics',
        name: 'perl-lsp-rs',
        version: '0.16.0',
      },
    },
    extensionMode: vscode.ExtensionMode.Production,
    extensionPath,
    globalState: state,
    subscriptions: [],
    workspaceState: state,
  } as unknown as vscode.ExtensionContext;
}

describe('extension activation startup scheduling (#3159)', () => {
  const originalSkipStartup = process.env.PERL_LSP_EXTENSION_TEST_SKIP_STARTUP;

  afterEach(async () => {
    if (originalSkipStartup === undefined) {
      delete process.env.PERL_LSP_EXTENSION_TEST_SKIP_STARTUP;
    } else {
      process.env.PERL_LSP_EXTENSION_TEST_SKIP_STARTUP = originalSkipStartup;
    }

    await deactivate();
    jest.clearAllMocks();
  });

  test('activation resolves even when language client start is still pending', async () => {
    process.env.PERL_LSP_EXTENSION_TEST_SKIP_STARTUP = '0';
    const extensionRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-activation-'));
    const serverPath = path.join(extensionRoot, process.platform === 'win32' ? 'perl-lsp.exe' : 'perl-lsp');
    fs.writeFileSync(serverPath, '');

    (vscode.workspace.getConfiguration as jest.Mock).mockImplementation(() => ({
      get: jest.fn((key: string, defaultValue?: unknown) => {
        if (key === 'serverPath') {
          return serverPath;
        }
        if (key === 'autoDownload') {
          return false;
        }
        return defaultValue;
      }),
      has: jest.fn(() => false),
      inspect: jest.fn(),
      update: jest.fn(async () => undefined),
    }));

    const activation = activate(makeContext(extensionRoot));

    await expect(Promise.race([activation.then(() => 'activated' as const), delay(250)]))
      .resolves
      .toBe('activated');
    await waitUntil(() => mockLanguageClientStart.mock.calls.length > 0, 500);

    expect(mockLanguageClientStart).toHaveBeenCalledTimes(1);
  });
});
