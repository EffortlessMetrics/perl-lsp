import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';

export const DEFAULT_INCLUDE_PATHS = ['lib', 'local/lib/perl5'] as const;

export type ConfigurationClass = 'live' | 'reconstruct' | 'restart' | 'unrelated';

export type ConfigurationChangeEventLike = Pick<
  vscode.ConfigurationChangeEvent,
  'affectsConfiguration'
>;

type ConfigurationReader = {
  get<T>(section: string, defaultValue: T): T;
  inspect?: (section: string) => unknown;
};

type NativeCriticSyncSettings = {
  enabled?: boolean;
  engine?: string;
  profile?: string;
  severity?: number;
  include?: string[];
  exclude?: string[];
};

type LegacyPerlCriticSyncSettings = {
  enabled?: boolean;
  severity?: number;
  profile?: string;
  theme?: string;
};

const NATIVE_CRITIC_KEYS = [
  'critic.enabled',
  'critic.engine',
  'critic.profile',
  'critic.severity',
  'critic.include',
  'critic.exclude',
] as const;

const LEGACY_CRITIC_KEYS = [
  'perlcritic.enabled',
  'perlcritic.severity',
  'perlcritic.profile',
  'perlcritic.theme',
] as const;

export const CRITIC_SETTINGS = [
  ...NATIVE_CRITIC_KEYS.map((key) => `perl-lsp.${key}`),
  ...LEGACY_CRITIC_KEYS.map((key) => `perl-lsp.${key}`),
] as const;

const LIVE_SETTINGS = [
  'perl-lsp.includePaths',
  'perl-lsp.trace.server',
  ...CRITIC_SETTINGS,
] as const;

const RECONSTRUCT_SETTINGS = [
  'perl-lsp.enableTestIntegration',
  'perl-lsp.aiCompletion.enabled',
  'perl-lsp.aiCompletion.streaming.enabled',
] as const;

const RESTART_SETTINGS = [
  'perl-lsp.serverPath',
  'perl-lsp.autoDownload',
  'perl-lsp.channel',
  'perl-lsp.versionTag',
  'perl-lsp.downloadBaseUrl',
  'perl-lsp.featureProfile',
  'perl-lsp.disabledFeatures',
  'perl-lsp.enableSemanticTokens',
  'perl-lsp.enableFormatting',
] as const;

function hasExplicitOverride(config: ConfigurationReader, key: string): boolean {
  const value = config.inspect?.(key) as
    | {
        globalValue?: unknown;
        workspaceValue?: unknown;
        workspaceFolderValue?: unknown;
        globalLanguageValue?: unknown;
        workspaceLanguageValue?: unknown;
        workspaceFolderLanguageValue?: unknown;
      }
    | undefined;

  return Boolean(
    value &&
    (value.globalValue !== undefined ||
      value.workspaceValue !== undefined ||
      value.workspaceFolderValue !== undefined ||
      value.globalLanguageValue !== undefined ||
      value.workspaceLanguageValue !== undefined ||
      value.workspaceFolderLanguageValue !== undefined),
  );
}

function getPerlCriticConfiguration(documentUri?: vscode.Uri): vscode.WorkspaceConfiguration {
  const scope: { uri?: vscode.Uri; languageId: string } = documentUri
    ? { uri: documentUri, languageId: 'perl' }
    : { languageId: 'perl' };
  return vscode.workspace.getConfiguration('perl-lsp', scope);
}

type ExplicitSettingSpec<T> = {
  key: string;
  property: keyof T;
  defaultValue: unknown;
};

function pickExplicit<T extends object>(
  config: ConfigurationReader,
  specs: readonly ExplicitSettingSpec<T>[],
): Partial<T> {
  const settings: Partial<T> = {};

  for (const spec of specs) {
    if (hasExplicitOverride(config, spec.key)) {
      settings[spec.property] = config.get(spec.key, spec.defaultValue) as T[typeof spec.property];
    }
  }

  return settings;
}

const NATIVE_CRITIC_SPECS: readonly ExplicitSettingSpec<NativeCriticSyncSettings>[] = [
  { key: 'critic.enabled', property: 'enabled', defaultValue: true },
  { key: 'critic.engine', property: 'engine', defaultValue: 'native' },
  { key: 'critic.profile', property: 'profile', defaultValue: 'recommended' },
  { key: 'critic.severity', property: 'severity', defaultValue: 3 },
  { key: 'critic.include', property: 'include', defaultValue: [] },
  { key: 'critic.exclude', property: 'exclude', defaultValue: [] },
];

const LEGACY_CRITIC_SPECS: readonly ExplicitSettingSpec<LegacyPerlCriticSyncSettings>[] = [
  { key: 'perlcritic.enabled', property: 'enabled', defaultValue: false },
  { key: 'perlcritic.severity', property: 'severity', defaultValue: 3 },
  { key: 'perlcritic.profile', property: 'profile', defaultValue: '' },
  { key: 'perlcritic.theme', property: 'theme', defaultValue: '' },
];

function getNativeCriticSyncSettings(
  config: ConfigurationReader,
  severityOverride?: number,
): NativeCriticSyncSettings {
  const settings = pickExplicit(config, NATIVE_CRITIC_SPECS);

  if (severityOverride !== undefined) {
    settings.severity = severityOverride;
  }

  return settings;
}

function getLegacyPerlCriticSyncSettings(
  config: ConfigurationReader,
): LegacyPerlCriticSyncSettings {
  return pickExplicit(config, LEGACY_CRITIC_SPECS);
}

function buildCriticSettings(
  documentUri?: vscode.Uri,
  severityOverride?: number,
): Record<string, unknown> | undefined {
  const config = getPerlCriticConfiguration(documentUri);
  const critic = getNativeCriticSyncSettings(config, severityOverride);
  const perlcritic = getLegacyPerlCriticSyncSettings(config);
  const perl: Record<string, unknown> = {};

  if (Object.keys(critic).length > 0) {
    perl.critic = critic;
  }
  if (Object.keys(perlcritic).length > 0) {
    perl.perlcritic = perlcritic;
  }

  return Object.keys(perl).length > 0 ? perl : undefined;
}

function readIncludePaths(config: ConfigurationReader): string[] {
  const configured = config.get<unknown>('includePaths', [...DEFAULT_INCLUDE_PATHS]);
  if (!Array.isArray(configured)) {
    return [...DEFAULT_INCLUDE_PATHS];
  }

  return configured.filter((value): value is string => typeof value === 'string');
}

export function buildWorkspaceConfigurationPayload(
  config: ConfigurationReader = vscode.workspace.getConfiguration('perl-lsp'),
): Record<string, unknown> | undefined {
  if (!hasExplicitOverride(config, 'includePaths')) {
    return undefined;
  }

  return {
    workspace: {
      includePaths: readIncludePaths(config),
    },
  };
}

export function buildPerlCriticConfiguration(
  documentUri?: vscode.Uri,
  severityOverride?: number,
): Record<string, unknown> | undefined {
  const critic = buildCriticSettings(documentUri, severityOverride);
  return critic ? { settings: { perl: critic } } : undefined;
}

export function buildLanguageClientConfigurationPayload(
  documentUri?: vscode.Uri,
): Record<string, unknown> {
  const config = vscode.workspace.getConfiguration('perl-lsp', documentUri);
  const perl: Record<string, unknown> = {};
  const workspace = buildWorkspaceConfigurationPayload(config);
  if (workspace) {
    Object.assign(perl, workspace);
  }
  const critic = buildCriticSettings(documentUri);
  if (critic) {
    Object.assign(perl, critic);
  }

  return { settings: { perl } };
}

export async function syncLanguageClientConfiguration(
  activeClient: Pick<LanguageClient, 'sendNotification'> | undefined,
  documentUri?: vscode.Uri,
): Promise<void> {
  if (!activeClient) {
    return;
  }

  await activeClient.sendNotification(
    'workspace/didChangeConfiguration',
    buildLanguageClientConfigurationPayload(documentUri),
  );
}

export async function syncPerlCriticConfiguration(
  activeClient: Pick<LanguageClient, 'sendNotification'> | undefined,
  documentUri?: vscode.Uri,
): Promise<void> {
  if (!activeClient) {
    return;
  }

  const payload = buildPerlCriticConfiguration(documentUri);
  if (payload) {
    await activeClient.sendNotification('workspace/didChangeConfiguration', payload);
  }
}

export function buildDisabledFeaturesFromConfig(config: ConfigurationReader): string[] {
  const base = config.get<string[]>('disabledFeatures', []).slice();
  if (!config.get<boolean>('enableSemanticTokens', true) && !base.includes('lsp.semantic_tokens')) {
    base.push('lsp.semantic_tokens');
  }
  if (!config.get<boolean>('enableFormatting', true) && !base.includes('lsp.formatting')) {
    base.push('lsp.formatting');
  }
  return base;
}

export function classifyConfigurationSetting(setting: string): ConfigurationClass {
  if ((LIVE_SETTINGS as readonly string[]).includes(setting)) {
    return 'live';
  }
  if ((RECONSTRUCT_SETTINGS as readonly string[]).includes(setting)) {
    return 'reconstruct';
  }
  if ((RESTART_SETTINGS as readonly string[]).includes(setting)) {
    return 'restart';
  }
  return 'unrelated';
}

export function classifyConfigurationChange(
  event: ConfigurationChangeEventLike,
): ConfigurationClass[] {
  const classes = new Set<ConfigurationClass>();
  for (const setting of [...LIVE_SETTINGS, ...RECONSTRUCT_SETTINGS, ...RESTART_SETTINGS]) {
    if (event.affectsConfiguration(setting)) {
      classes.add(classifyConfigurationSetting(setting));
    }
  }
  return [...classes];
}

export function hasExplicitPerlCriticOverrides(documentUri?: vscode.Uri): boolean {
  const config = getPerlCriticConfiguration(documentUri);
  return [...NATIVE_CRITIC_KEYS, ...LEGACY_CRITIC_KEYS].some((key) =>
    hasExplicitOverride(config, key),
  );
}
