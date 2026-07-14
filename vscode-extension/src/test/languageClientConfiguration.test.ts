import * as vscode from 'vscode';
import {
  buildDisabledFeaturesFromConfig,
  buildLanguageClientConfigurationPayload,
  buildPerlCriticConfiguration,
  buildWorkspaceConfigurationPayload,
  classifyConfigurationChange,
  classifyConfigurationSetting,
  DEFAULT_INCLUDE_PATHS,
  hasExplicitPerlCriticOverrides,
  syncLanguageClientConfiguration,
  syncPerlCriticConfiguration,
} from '../languageClientConfiguration';

function makeConfig(values: Record<string, unknown>, explicit = Object.keys(values)) {
  return {
    get: jest.fn((key: string, defaultValue?: unknown) => values[key] ?? defaultValue),
    inspect: jest.fn((key: string) =>
      explicit.includes(key) ? { workspaceValue: values[key] } : undefined,
    ),
  } as unknown as vscode.WorkspaceConfiguration;
}

describe('language client configuration', () => {
  test('does not turn built-in defaults into an explicit workspace override', () => {
    const payload = buildWorkspaceConfigurationPayload(makeConfig({}, []));

    expect(payload).toBeUndefined();
    expect(DEFAULT_INCLUDE_PATHS).toEqual(['lib', 'local/lib/perl5']);
  });

  test('builds canonical workspace include-path payload from current settings', () => {
    const payload = buildWorkspaceConfigurationPayload(
      makeConfig({ includePaths: ['vendor/lib', 'local/lib/perl5'] }),
    );

    expect(payload).toEqual({
      workspace: { includePaths: ['vendor/lib', 'local/lib/perl5'] },
    });
  });

  test('combines workspace and critic settings under the canonical perl payload', () => {
    const config = makeConfig({
      includePaths: ['vendor/lib'],
      'critic.enabled': true,
      'critic.severity': 4,
      'perlcritic.severity': 2,
    });
    (vscode.workspace.getConfiguration as jest.Mock).mockReturnValue(config);

    expect(buildLanguageClientConfigurationPayload()).toEqual({
      settings: {
        perl: {
          workspace: { includePaths: ['vendor/lib'] },
          critic: { enabled: true, severity: 4 },
          perlcritic: { severity: 2 },
        },
      },
    });
  });

  test('initial synchronization sends one canonical configuration notification', async () => {
    const config = makeConfig({ includePaths: ['workspace/lib'] });
    (vscode.workspace.getConfiguration as jest.Mock).mockReturnValue(config);
    const sendNotification = jest.fn(async () => undefined);

    await syncLanguageClientConfiguration({ sendNotification });

    expect(sendNotification).toHaveBeenCalledTimes(1);
    expect(sendNotification).toHaveBeenCalledWith(
      'workspace/didChangeConfiguration',
      expect.objectContaining({
        settings: { perl: { workspace: { includePaths: ['workspace/lib'] } } },
      }),
    );
  });

  test('uses the document scope for folder-specific initial synchronization', () => {
    const documentUri = vscode.Uri.file('/workspace/folder/src/main.pl');
    const config = makeConfig({ includePaths: ['folder/lib'] });
    (vscode.workspace.getConfiguration as jest.Mock).mockReturnValue(config);

    buildLanguageClientConfigurationPayload(documentUri);

    expect(vscode.workspace.getConfiguration).toHaveBeenCalledWith('perl-lsp', documentUri);
  });

  test('preserves project configuration when no editor setting is explicit', () => {
    const config = makeConfig({}, []);
    (vscode.workspace.getConfiguration as jest.Mock).mockReturnValue(config);

    expect(buildLanguageClientConfigurationPayload()).toEqual({
      settings: { perl: {} },
    });
  });

  test('preserves deprecated critic aliases without making them preferred', () => {
    const config = makeConfig({ 'perlcritic.severity': 2 });
    (vscode.workspace.getConfiguration as jest.Mock).mockReturnValue(config);

    expect(buildPerlCriticConfiguration()).toEqual({
      settings: { perl: { perlcritic: { severity: 2 } } },
    });
  });

  test('detects explicit critic overrides and resets them when unset', () => {
    const values = { 'critic.severity': 4 };
    const explicit = ['critic.severity'];
    const config = makeConfig(values, explicit);
    (vscode.workspace.getConfiguration as jest.Mock).mockReturnValue(config);

    expect(hasExplicitPerlCriticOverrides()).toBe(true);
    expect(buildPerlCriticConfiguration()).toEqual({
      settings: { perl: { critic: { severity: 4 } } },
    });

    explicit.length = 0;

    expect(hasExplicitPerlCriticOverrides()).toBe(false);
    expect(buildPerlCriticConfiguration()).toBeUndefined();
    expect(buildLanguageClientConfigurationPayload()).toEqual({
      settings: { perl: {} },
    });
  });

  test('does not notify when no active client is available', async () => {
    await syncLanguageClientConfiguration(undefined);
    await syncPerlCriticConfiguration(undefined);
  });

  test('adds disabled feature aliases without mutating the configured array', () => {
    const disabledFeatures = ['lsp.hover'];
    const config = makeConfig({
      disabledFeatures,
      enableSemanticTokens: false,
      enableFormatting: false,
    });

    expect(buildDisabledFeaturesFromConfig(config)).toEqual([
      'lsp.hover',
      'lsp.semantic_tokens',
      'lsp.formatting',
    ]);
    expect(disabledFeatures).toEqual(['lsp.hover']);
  });

  test.each([
    ['perl-lsp.trace.server', 'live'],
    ['perl-lsp.includePaths', 'live'],
    ['perl-lsp.critic.severity', 'live'],
    ['perl-lsp.enableTestIntegration', 'reconstruct'],
    ['perl-lsp.aiCompletion.enabled', 'reconstruct'],
    ['perl-lsp.featureProfile', 'restart'],
    ['perl-lsp.enableFormatting', 'restart'],
    ['perl-lsp.autoPopulateNewFiles', 'unrelated'],
  ])('classifies %s as %s', (setting, expected) => {
    expect(classifyConfigurationSetting(setting)).toBe(expected);
  });

  test('reports all actions affected by one configuration event', () => {
    const changed = new Set([
      'perl-lsp.trace.server',
      'perl-lsp.aiCompletion.enabled',
      'perl-lsp.featureProfile',
    ]);

    expect(
      classifyConfigurationChange({
        affectsConfiguration: (setting: string) => changed.has(setting),
      }),
    ).toEqual(['live', 'reconstruct', 'restart']);
  });
});
