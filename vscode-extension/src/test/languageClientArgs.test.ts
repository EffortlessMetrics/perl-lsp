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

import { formatIssueDiagnosticInfo, getLanguageServerLaunchArgs } from '../extension';

describe('language client launch args', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  test('does not add stdio because the client transport already uses stdio', () => {
    expect(getLanguageServerLaunchArgs(false)).toEqual([]);
    expect(getLanguageServerLaunchArgs(true)).toEqual(['--log']);
  });

  test('adds the configured feature profile without reintroducing stdio', () => {
    (vscode.workspace.getConfiguration as jest.Mock).mockReturnValue({
      get: jest.fn((key: string, defaultValue?: unknown) => {
        if (key === 'featureProfile') {
          return 'prod';
        }
        return defaultValue;
      }),
    });

    expect(getLanguageServerLaunchArgs(false)).toEqual(['--feature-profile=prod']);
    expect(getLanguageServerLaunchArgs(true)).toEqual(['--log', '--feature-profile=prod']);
  });
});


describe('issue diagnostic formatting', () => {
  test('uses the provided editor name for non-VS Code hosts', () => {
    const info = formatIssueDiagnosticInfo({
      serverVersion: 'perllsp 0.12.4',
      extensionVersion: '0.12.4',
      editorVersion: '1.99.0',
      platform: 'linux',
      arch: 'x64',
      editorName: 'Kilo Code',
    });

    expect(info).toContain('Kilo Code: 1.99.0');
    expect(info).not.toContain('VS Code: 1.99.0');
  });

  test('falls back to VS Code when editor name is missing', () => {
    const info = formatIssueDiagnosticInfo({
      serverVersion: 'perllsp 0.12.4',
      extensionVersion: '0.12.4',
      editorVersion: '1.99.0',
      platform: 'linux',
      arch: 'x64',
    });

    expect(info).toContain('VS Code: 1.99.0');
  });
});
