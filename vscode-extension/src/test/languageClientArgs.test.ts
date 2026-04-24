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

import { getLanguageServerLaunchArgs } from '../extension';

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
