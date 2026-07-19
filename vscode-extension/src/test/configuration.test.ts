/**
 * Unit tests validating the extension's static configuration files:
 *   - language-configuration.json
 *   - package.json (contributes section, activation events, settings)
 *   - snippets/*.json
 *
 * These are "contract tests" -- they verify that the configuration surfaces
 * exposed to VSCode and end-users match expectations. Breaking these
 * signals a user-visible regression.
 */

import * as fs from 'fs';
import * as path from 'path';

const EXT_ROOT = path.resolve(__dirname, '..', '..');
const REPO_ROOT = path.resolve(EXT_ROOT, '..');

interface AutoClosingPair {
  open: string;
  close?: string;
  notIn?: string[];
}

interface LanguageConfiguration {
  comments: { lineComment: string };
  brackets: [string, string][];
  autoClosingPairs: AutoClosingPair[];
  surroundingPairs: [string, string][];
  indentationRules: {
    increaseIndentPattern: string;
    decreaseIndentPattern: string;
  };
  wordPattern: string;
}

interface SettingItems {
  type: string;
  enum: Array<string | number>;
}

interface SettingDefinition {
  order: number;
  description: string;
  markdownDescription: string;
  type: string;
  default: unknown;
  enum: Array<string | number>;
  enumDescriptions: string[];
  items: SettingItems;
  scope: string;
  deprecationMessage: string;
  minimum: number;
}

interface LanguageContribution {
  id: string;
  aliases: string[];
  extensions: string[];
  configuration?: string;
  firstLine: string;
}

interface CommandContribution {
  command: string;
  title: string;
  category: string;
}

interface ConfigurationSection {
  title: string;
  properties: Record<string, SettingDefinition>;
}

interface MenuEntry {
  command: string;
  when?: string;
}

interface DebuggerContribution {
  type: string;
  configurationAttributes: { launch: { required: string[] } };
  initialConfigurations: unknown[];
}

interface BreakpointContribution {
  language: string;
}

interface KeybindingContribution {
  command: string;
  key: string;
  when: string;
}

interface GrammarContribution {
  language: string;
  scopeName: string;
  path: string;
  embeddedLanguages: Record<string, string>;
}

interface GrammarPattern {
  match: string;
  name: string;
}

interface GrammarRepositoryEntry {
  patterns: GrammarPattern[];
}

interface GrammarRepository {
  keywords: GrammarRepositoryEntry;
  swig: GrammarRepositoryEntry;
  headers: GrammarRepositoryEntry;
  steps: GrammarRepositoryEntry;
  tags: GrammarRepositoryEntry;
  tables: GrammarRepositoryEntry;
}

interface GrammarFile {
  repository: GrammarRepository;
}

interface Snippet {
  prefix: string | string[];
  body: string | string[];
  description: string;
}

interface PackageManifest {
  name: string;
  version: string;
  main: string;
  publisher: string;
  activationEvents: string[];
  engines: { vscode: string };
  dependencies: Record<string, string>;
  extensionKind: string[];
  capabilities: { untrustedWorkspaces: { supported: boolean } };
  contributes: {
    languages: LanguageContribution[];
    commands: CommandContribution[];
    configuration: ConfigurationSection[];
    menus: { commandPalette: MenuEntry[] };
    debuggers: DebuggerContribution[];
    breakpoints: BreakpointContribution[];
    keybindings: KeybindingContribution[];
    grammars: GrammarContribution[];
  };
}

type SnippetCatalog = Record<string, Snippet>;

function readJson<T>(relativePath: string): T {
  const fullPath = path.join(EXT_ROOT, relativePath);
  return JSON.parse(fs.readFileSync(fullPath, 'utf8')) as T;
}

function readRepoText(relativePath: string): string {
  return fs.readFileSync(path.join(REPO_ROOT, relativePath), 'utf8');
}

function required<T>(value: T | undefined, label: string): T {
  if (value === undefined) {
    throw new Error(`Missing ${label}`);
  }
  return value;
}

function getSetting(properties: Record<string, SettingDefinition>, key: string): SettingDefinition {
  return required(properties[key], key);
}

// ---------------------------------------------------------------------------
// language-configuration.json
// ---------------------------------------------------------------------------
describe('language-configuration.json', () => {
  let langConfig: LanguageConfiguration;

  beforeAll(() => {
    langConfig = readJson<LanguageConfiguration>('language-configuration.json');
  });

  test('has line comment set to #', () => {
    expect(langConfig.comments.lineComment).toBe('#');
  });

  test('defines bracket pairs for {}, [], ()', () => {
    const brackets: [string, string][] = langConfig.brackets;
    const pairs = brackets.map(([o, c]) => `${o}${c}`);
    expect(pairs).toContain('{}');
    expect(pairs).toContain('[]');
    expect(pairs).toContain('()');
  });

  test('has auto-closing pairs for all bracket types', () => {
    const opens = langConfig.autoClosingPairs.map((pair) => pair.open);
    expect(opens).toContain('{');
    expect(opens).toContain('[');
    expect(opens).toContain('(');
    expect(opens).toContain('"');
    expect(opens).toContain("'");
  });

  test('auto-closing single quotes are suppressed inside strings and comments', () => {
    const singleQuote = required(
      langConfig.autoClosingPairs.find((pair) => pair.open === "'"),
      'single-quote auto-closing pair',
    );
    expect(singleQuote).toBeDefined();
    expect(singleQuote.notIn).toContain('string');
    expect(singleQuote.notIn).toContain('comment');
  });

  test('has surrounding pairs for common delimiters', () => {
    const pairs = (langConfig.surroundingPairs as [string, string][]).map(([o, c]) => `${o}${c}`);
    expect(pairs).toContain('{}');
    expect(pairs).toContain('()');
    expect(pairs).toContain('""');
    expect(pairs).toContain("''");
    expect(pairs).toContain('``');
  });

  test('has indentation rules', () => {
    expect(langConfig.indentationRules).toBeDefined();
    expect(langConfig.indentationRules.increaseIndentPattern).toBeTruthy();
    expect(langConfig.indentationRules.decreaseIndentPattern).toBeTruthy();
  });

  test('increaseIndentPattern matches Perl keywords', () => {
    const pattern = new RegExp(langConfig.indentationRules.increaseIndentPattern);
    expect(pattern.test('sub foo {')).toBe(true);
    expect(pattern.test('if ($x) {')).toBe(true);
    expect(pattern.test('while (1) {')).toBe(true);
    expect(pattern.test('for my $i (@arr) {')).toBe(true);
    expect(pattern.test('foreach my $item (@list) {')).toBe(true);
  });

  test('decreaseIndentPattern matches closing structures', () => {
    const pattern = new RegExp(langConfig.indentationRules.decreaseIndentPattern);
    expect(pattern.test('}')).toBe(true);
    expect(pattern.test('    }')).toBe(true);
    expect(pattern.test('    )')).toBe(true);
  });

  test('has a wordPattern defined', () => {
    expect(langConfig.wordPattern).toBeTruthy();
    expect(() => new RegExp(langConfig.wordPattern)).not.toThrow();
  });
});

describe('gherkin-language-configuration.json', () => {
  let langConfig: LanguageConfiguration;

  beforeAll(() => {
    langConfig = readJson<LanguageConfiguration>('gherkin-language-configuration.json');
  });

  test('has line comment set to #', () => {
    expect(langConfig.comments.lineComment).toBe('#');
  });

  test('defines .feature editing bracket pairs', () => {
    const brackets: [string, string][] = langConfig.brackets;
    const pairs = brackets.map(([o, c]) => `${o}${c}`);
    expect(pairs).toContain('{}');
    expect(pairs).toContain('[]');
    expect(pairs).toContain('()');
  });

  test('has quote auto-closing pairs', () => {
    const opens = langConfig.autoClosingPairs.map((pair) => pair.open);
    expect(opens).toContain('"');
    expect(opens).toContain("'");
  });
});

// ---------------------------------------------------------------------------
// package.json contributes
// ---------------------------------------------------------------------------
describe('package.json contributes', () => {
  let pkg: PackageManifest;

  beforeAll(() => {
    pkg = readJson<PackageManifest>('package.json');
  });

  describe('language registration', () => {
    test('registers perl language', () => {
      const langs = pkg.contributes.languages;
      expect(langs).toBeDefined();
      const perl = langs.find((language) => language.id === 'perl');
      expect(perl).toBeDefined();
    });

    test('registers gherkin language for feature files', () => {
      const langs = pkg.contributes.languages;
      const gherkin = required(
        langs.find((language) => language.id === 'gherkin'),
        'gherkin language',
      );
      expect(gherkin).toBeDefined();
      expect(gherkin.aliases).toContain('Gherkin');
      expect(gherkin.aliases).toContain('Cucumber');
      expect(gherkin.extensions).toContain('.feature');
      expect(gherkin.configuration).toBe('./gherkin-language-configuration.json');
    });

    test('perl language has expected file extensions', () => {
      const perl = required(
        pkg.contributes.languages.find((language) => language.id === 'perl'),
        'perl language',
      );
      const exts: string[] = perl.extensions;
      expect(exts).toContain('.pl');
      expect(exts).toContain('.cgi');
      expect(exts).toContain('.fcgi');
      expect(exts).toContain('.pm');
      expect(exts).toContain('.xs');
      expect(exts).toContain('.xsi');
      expect(exts).toContain('.t');
      expect(exts).toContain('.pod');
      expect(exts).toContain('.psgi');
      expect(exts).toContain('.mason');
      expect(exts).toContain('.mas');
      expect(exts).toContain('.tt');
      expect(exts).toContain('.tt2');
      expect(exts).toContain('.ep');
      expect(exts).not.toContain('.m');
      expect(exts).toContain('.xs');
      expect(exts).toContain('.i');
    });

    test('perl language keeps XS interface files associated with perl', () => {
      const perl = required(
        pkg.contributes.languages.find((language) => language.id === 'perl'),
        'perl language',
      );
      const exts: string[] = perl.extensions;
      expect(exts.filter((ext: string) => ext === '.xs' || ext === '.i')).toHaveLength(2);
    });

    test('perl language has shebang first-line detection', () => {
      const perl = required(
        pkg.contributes.languages.find((language) => language.id === 'perl'),
        'perl language',
      );
      expect(perl.firstLine).toBeTruthy();
      const pattern = new RegExp(perl.firstLine);
      expect(pattern.test('#!/usr/bin/perl')).toBe(true);
      expect(pattern.test('#!/usr/bin/env perl')).toBe(true);
    });

    test('perl language aliases include "Perl" and "Perl 5"', () => {
      const perl = required(
        pkg.contributes.languages.find((language) => language.id === 'perl'),
        'perl language',
      );
      expect(perl.aliases).toContain('Perl');
      expect(perl.aliases).toContain('Perl 5');
    });
  });

  describe('activation events', () => {
    test('uses only lazy activation events required by the extension', () => {
      expect(pkg.activationEvents).toEqual([
        'onLanguage:perl',
        'onLanguage:perl5',
        'onLanguage:gherkin',
        'onWalkthrough:perl-lsp.gettingStarted',
        'onDebugResolve:perl',
        'onDebugInitialConfigurations',
      ]);
    });

    test('does not activate after VS Code startup', () => {
      expect(pkg.activationEvents).not.toContain('onStartupFinished');
    });

    test('does not declare redundant command activation events', () => {
      const commandActivationEvents = pkg.activationEvents.filter((event) =>
        event.startsWith('onCommand:'),
      );

      expect(commandActivationEvents).toEqual([]);
    });
  });

  describe('commands', () => {
    test('registers expected commands', () => {
      const commandIds = pkg.contributes.commands.map((command) => command.command);
      expect(commandIds).toContain('perl-lsp.restart');
      expect(commandIds).toContain('perl-lsp.showVersion');
      expect(commandIds).toContain('perl-lsp.showOutput');
      expect(commandIds).toContain('perl-lsp.reinstall');
      expect(commandIds).toContain('perl-lsp.organizeImports');
      expect(commandIds).toContain('perl-lsp.runTests');
      expect(commandIds).toContain('perl-lsp.showStatusMenu');
      expect(commandIds).toContain('perl-lsp.createDebugConfig');
    });

    test('registers refactoring commands', () => {
      const commandIds = pkg.contributes.commands.map((command) => command.command);
      expect(commandIds).toContain('perl-lsp.extractVariable');
      expect(commandIds).toContain('perl-lsp.extractMethod');
      expect(commandIds).toContain('perl-lsp.showRefactoringOptions');
    });

    test('refactoring commands have descriptive titles', () => {
      const cmds = pkg.contributes.commands;
      const extractVar = required(
        cmds.find((command) => command.command === 'perl-lsp.extractVariable'),
        'extractVariable command',
      );
      const extractMethod = required(
        cmds.find((command) => command.command === 'perl-lsp.extractMethod'),
        'extractMethod command',
      );
      const showRefactoring = required(
        cmds.find((command) => command.command === 'perl-lsp.showRefactoringOptions'),
        'showRefactoringOptions command',
      );
      expect(extractVar).toBeDefined();
      expect(extractVar.title).toBeTruthy();
      expect(extractMethod).toBeDefined();
      expect(extractMethod.title).toBeTruthy();
      expect(showRefactoring).toBeDefined();
      expect(showRefactoring.title).toBeTruthy();
    });

    test('all commands have a category', () => {
      for (const cmd of pkg.contributes.commands) {
        expect(cmd.category).toBeTruthy();
      }
    });

    test('all commands have a title', () => {
      for (const cmd of pkg.contributes.commands) {
        expect(cmd.title).toBeTruthy();
      }
    });
  });

  describe('configuration settings', () => {
    // The configuration is an array of grouped sections; merge all properties
    // into a single lookup map for backwards-compatible assertions.
    let properties: Record<string, SettingDefinition>;
    let configSections: ConfigurationSection[];

    beforeAll(() => {
      const configuration = pkg.contributes.configuration;
      // Support both array (grouped) and legacy single-object formats.
      configSections = Array.isArray(configuration) ? configuration : [configuration];
      properties = Object.assign({}, ...configSections.map((section) => section.properties ?? {}));
    });

    // --- Grouping structure ---

    test('configuration is an array with three named groups', () => {
      expect(Array.isArray(pkg.contributes.configuration)).toBe(true);
      const titles: string[] = configSections.map((section) => section.title);
      expect(titles.some((t) => /core/i.test(t))).toBe(true);
      expect(titles.some((t) => /editor/i.test(t))).toBe(true);
      expect(titles.some((t) => /advanced/i.test(t))).toBe(true);
    });

    test('Core group contains serverPath, autoDownload, includePaths, enableSemanticTokens', () => {
      const coreSection = required(
        configSections.find((section) => /core/i.test(section.title)),
        'core configuration section',
      );
      expect(coreSection).toBeDefined();
      const keys = Object.keys(coreSection.properties);
      expect(keys).toContain('perl-lsp.serverPath');
      expect(keys).toContain('perl-lsp.autoDownload');
      expect(keys).toContain('perl-lsp.includePaths');
      expect(keys).toContain('perl-lsp.enableSemanticTokens');
      expect(keys).not.toContain('perl-lsp.enableDiagnostics');
    });

    test('Editor group contains formatting and test integration settings (enableRefactoring removed)', () => {
      const editorSection = required(
        configSections.find((section) => /editor/i.test(section.title)),
        'editor configuration section',
      );
      expect(editorSection).toBeDefined();
      const keys = Object.keys(editorSection.properties);
      expect(keys).toContain('perl-lsp.enableFormatting');
      expect(keys).toContain('perl-lsp.formatOnSave');
      expect(keys).toContain('perl-lsp.enableTestIntegration');
      expect(keys).toContain('perl-lsp.critic.enabled');
      expect(keys).toContain('perl-lsp.critic.engine');
      expect(keys).toContain('perl-lsp.critic.profile');
      expect(keys).toContain('perl-lsp.critic.severity');
      expect(keys).toContain('perl-lsp.critic.include');
      expect(keys).toContain('perl-lsp.critic.exclude');
      expect(keys).toContain('perl-lsp.perlcritic.enabled');
      expect(keys).toContain('perl-lsp.perlcritic.severity');
      expect(keys).toContain('perl-lsp.perlcritic.profile');
      expect(keys).toContain('perl-lsp.perlcritic.theme');
      expect(keys).not.toContain('perl-lsp.enableRefactoring');
    });

    test('Advanced group contains featureProfile, trace.server, channel, downloadBaseUrl', () => {
      const advancedSection = required(
        configSections.find((section) => /advanced/i.test(section.title)),
        'advanced configuration section',
      );
      expect(advancedSection).toBeDefined();
      const keys = Object.keys(advancedSection.properties);
      expect(keys).toContain('perl-lsp.featureProfile');
      expect(keys).toContain('perl-lsp.trace.server');
      expect(keys).toContain('perl-lsp.channel');
      expect(keys).toContain('perl-lsp.downloadBaseUrl');
    });

    test('all settings have an order field', () => {
      for (const setting of Object.values(properties)) {
        expect(typeof setting.order).toBe('number');
      }
    });

    test('all settings have a description field (plain-text fallback for non-markdown contexts)', () => {
      for (const setting of Object.values(properties)) {
        expect(typeof setting.description).toBe('string');
        expect(setting.description.length).toBeGreaterThan(10);
      }
    });

    test('all settings have a type field', () => {
      for (const setting of Object.values(properties)) {
        expect(setting.type).toBeTruthy();
      }
    });

    test('all settings have a default value', () => {
      for (const setting of Object.values(properties)) {
        expect(setting).toHaveProperty('default');
      }
    });

    // --- Individual setting contracts ---

    test('defines serverPath setting', () => {
      expect(getSetting(properties, 'perl-lsp.serverPath')).toBeDefined();
      expect(getSetting(properties, 'perl-lsp.serverPath').type).toBe('string');
    });

    test('defines autoDownload setting with default true', () => {
      expect(getSetting(properties, 'perl-lsp.autoDownload')).toBeDefined();
      expect(getSetting(properties, 'perl-lsp.autoDownload').default).toBe(true);
    });

    test('defines trace.server with valid enum values', () => {
      const trace = getSetting(properties, 'perl-lsp.trace.server');
      expect(trace).toBeDefined();
      expect(trace.enum).toContain('off');
      expect(trace.enum).toContain('messages');
      expect(trace.enum).toContain('verbose');
    });

    test('defines channel setting with valid enum', () => {
      const channel = getSetting(properties, 'perl-lsp.channel');
      expect(channel).toBeDefined();
      expect(channel.enum).toContain('latest');
      expect(channel.enum).toContain('stable');
      expect(channel.enum).toContain('tag');
    });

    test('defines featureProfile setting with valid enum', () => {
      const profile = getSetting(properties, 'perl-lsp.featureProfile');
      expect(profile).toBeDefined();
      expect(profile.enum).toContain('auto');
      expect(profile.enum).toContain('ga');
      expect(profile.enum).toContain('all');
    });

    test('featureProfile has enumDescriptions for every enum value', () => {
      const profile = getSetting(properties, 'perl-lsp.featureProfile');
      expect(Array.isArray(profile.enumDescriptions)).toBe(true);
      expect(profile.enumDescriptions.length).toBe(profile.enum.length);
      for (const desc of profile.enumDescriptions) {
        expect(typeof desc).toBe('string');
        expect(desc.length).toBeGreaterThan(0);
      }
    });

    test('trace.server has enumDescriptions matching its enum values', () => {
      const trace = getSetting(properties, 'perl-lsp.trace.server');
      expect(Array.isArray(trace.enumDescriptions)).toBe(true);
      expect(trace.enumDescriptions.length).toBe(trace.enum.length);
      for (const desc of trace.enumDescriptions) {
        expect(typeof desc).toBe('string');
        expect(desc.length).toBeGreaterThan(0);
      }
    });

    test('channel has enumDescriptions matching its enum values', () => {
      const channel = getSetting(properties, 'perl-lsp.channel');
      expect(Array.isArray(channel.enumDescriptions)).toBe(true);
      expect(channel.enumDescriptions.length).toBe(channel.enum.length);
      for (const desc of channel.enumDescriptions) {
        expect(typeof desc).toBe('string');
        expect(desc.length).toBeGreaterThan(0);
      }
    });

    test('defines enableSemanticTokens with default true', () => {
      expect(getSetting(properties, 'perl-lsp.enableSemanticTokens').default).toBe(true);
    });

    test('defines formatOnSave with default false', () => {
      expect(getSetting(properties, 'perl-lsp.formatOnSave').default).toBe(false);
    });

    test('defines includePaths with sensible defaults', () => {
      const includePaths = getSetting(properties, 'perl-lsp.includePaths');
      expect(includePaths.default).toContain('lib');
      expect(includePaths.default).toContain('local/lib/perl5');
    });

    test('defines critic.enabled with default true (native on by default)', () => {
      const setting = getSetting(properties, 'perl-lsp.critic.enabled');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('boolean');
      expect(setting.default).toBe(true);
    });

    test('defines critic.engine as a native/legacy picker defaulting to native', () => {
      const setting = getSetting(properties, 'perl-lsp.critic.engine');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('string');
      expect(setting.enum).toEqual(['native', 'legacy']);
      expect(setting.default).toBe('native');
    });

    test('defines critic.profile as a recommended/strict picker defaulting to recommended', () => {
      const setting = getSetting(properties, 'perl-lsp.critic.profile');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('string');
      expect(setting.enum).toEqual(['recommended', 'strict']);
      expect(setting.default).toBe('recommended');
    });

    test('defines critic.severity as a 1-5 picker with default 3', () => {
      const setting = getSetting(properties, 'perl-lsp.critic.severity');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('number');
      expect(setting.enum).toEqual([1, 2, 3, 4, 5]);
      expect(setting.default).toBe(3);
    });

    test('defines critic.include and critic.exclude as string arrays', () => {
      for (const key of ['perl-lsp.critic.include', 'perl-lsp.critic.exclude']) {
        const setting = getSetting(properties, key);
        expect(setting).toBeDefined();
        expect(setting.type).toBe('array');
        expect(setting.items.type).toBe('string');
        expect(setting.default).toEqual([]);
      }
    });

    test('legacy perlcritic.* aliases are deprecated but still present', () => {
      for (const key of [
        'perl-lsp.perlcritic.enabled',
        'perl-lsp.perlcritic.severity',
        'perl-lsp.perlcritic.profile',
        'perl-lsp.perlcritic.theme',
      ]) {
        const setting = getSetting(properties, key);
        expect(setting).toBeDefined();
        expect(setting.deprecationMessage).toBeTruthy();
      }
    });

    test('defines perlcritic.enabled with default false', () => {
      const setting = getSetting(properties, 'perl-lsp.perlcritic.enabled');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('boolean');
      expect(setting.default).toBe(false);
    });

    test('defines perlcritic.severity as a 1-5 picker with default 3', () => {
      const setting = getSetting(properties, 'perl-lsp.perlcritic.severity');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('number');
      expect(setting.enum).toEqual([1, 2, 3, 4, 5]);
      expect(setting.enumDescriptions).toHaveLength(5);
      expect(setting.default).toBe(3);
    });

    test('defines perlcritic.profile as a string setting', () => {
      const setting = getSetting(properties, 'perl-lsp.perlcritic.profile');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('string');
      expect(setting.default).toBe('');
    });

    test('defines perlcritic.theme as a string setting', () => {
      const setting = getSetting(properties, 'perl-lsp.perlcritic.theme');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('string');
      expect(setting.default).toBe('');
    });

    test('includePaths markdownDescription mentions module-not-found guidance', () => {
      const desc: string = getSetting(properties, 'perl-lsp.includePaths').markdownDescription;
      // Must mention the "Can't locate" symptom so users know what to search for
      expect(desc).toMatch(/can't locate/i);
    });

    test('includePaths has items schema typed as string', () => {
      const includePaths = getSetting(properties, 'perl-lsp.includePaths');
      expect(includePaths.items).toBeDefined();
      expect(includePaths.items.type).toBe('string');
    });

    test('defines downloadBaseUrl for internal hosting', () => {
      const setting = getSetting(properties, 'perl-lsp.downloadBaseUrl');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('string');
      expect(setting.scope).toBe('machine');
    });

    test('defines autoPopulateNewFiles with default true', () => {
      const setting = getSetting(properties, 'perl-lsp.autoPopulateNewFiles');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('boolean');
      expect(setting.default).toBe(true);
    });

    test('defines updateCheckInterval setting used by background update checker', () => {
      const setting = getSetting(properties, 'perl-lsp.updateCheckInterval');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('number');
      expect(setting.default).toBe(24);
      // minimum of 0 means "disable"
      expect(setting.minimum).toBe(0);
    });

    test('defines autoUpdate setting used by silent updater', () => {
      const setting = getSetting(properties, 'perl-lsp.autoUpdate');
      expect(setting).toBeDefined();
      expect(setting.type).toBe('boolean');
      expect(setting.default).toBe(false);
    });

    test('machine-scoped settings use scope machine', () => {
      // Settings that store binary/system paths must be machine-scoped so
      // remote/container environments get the correct binary path.
      const machineScoped = [
        'perl-lsp.serverPath',
        'perl-lsp.downloadBaseUrl',
        'perl-lsp.channel',
        'perl-lsp.versionTag',
        'perl-lsp.autoDownload',
        'perl-lsp.updateCheckInterval',
        'perl-lsp.autoUpdate',
      ];
      for (const key of machineScoped) {
        expect(getSetting(properties, key).scope).toBe('machine');
      }
    });

    test('resource-scoped settings use scope resource', () => {
      // Per-file/workspace settings should be resource-scoped so they can be
      // overridden in workspace and folder settings.
      const resourceScoped = [
        'perl-lsp.includePaths',
        'perl-lsp.enableSemanticTokens',
        'perl-lsp.enableFormatting',
        'perl-lsp.formatOnSave',
        'perl-lsp.perltidyConfig',
        'perl-lsp.critic.enabled',
        'perl-lsp.critic.engine',
        'perl-lsp.critic.profile',
        'perl-lsp.critic.severity',
        'perl-lsp.critic.include',
        'perl-lsp.critic.exclude',
        'perl-lsp.perlcritic.enabled',
        'perl-lsp.perlcritic.severity',
        'perl-lsp.perlcritic.profile',
        'perl-lsp.perlcritic.theme',
        'perl-lsp.enableTestIntegration',
        'perl-lsp.autoPopulateNewFiles',
      ];
      for (const key of resourceScoped) {
        expect(getSetting(properties, key).scope).toBe('resource');
      }
    });

    test('disabledFeatures items have an enum for VS Code settings UI picker', () => {
      const setting = getSetting(properties, 'perl-lsp.disabledFeatures');
      expect(setting.items?.enum).toBeDefined();
      expect(Array.isArray(setting.items.enum)).toBe(true);
      expect(setting.items.enum.length).toBeGreaterThan(0);
    });
  });

  describe('openConfigurationGuide command', () => {
    test('registers perl-lsp.openConfigurationGuide command', () => {
      const commandIds = pkg.contributes.commands.map((command) => command.command);
      expect(commandIds).toContain('perl-lsp.openConfigurationGuide');
    });

    test('openConfigurationGuide has Perl category', () => {
      const cmd = required(
        pkg.contributes.commands.find(
          (command) => command.command === 'perl-lsp.openConfigurationGuide',
        ),
        'openConfigurationGuide command',
      );
      expect(cmd.category).toBe('Perl');
    });

    test('openConfigurationGuide is listed in commandPalette without language restriction', () => {
      const palette = pkg.contributes.menus.commandPalette;
      const entry = required(
        palette.find((menuEntry) => menuEntry.command === 'perl-lsp.openConfigurationGuide'),
        'openConfigurationGuide palette entry',
      );
      expect(entry).toBeDefined();
      // Should be available globally (no when clause restricting to perl)
      expect(entry.when ?? '').not.toMatch(/editorLangId/);
    });
  });

  describe('documentation parity', () => {
    test('documents the manifest minimum VS Code version and only real settings', () => {
      const setupGuide = readRepoText('docs/EDITORS/VS_CODE_SETUP.md');
      const configReference = readRepoText('docs/reference/CONFIG.md');
      const minimumVersion = pkg.engines.vscode.match(/(\d+\.\d+)/)?.[1];

      expect(minimumVersion).toBeDefined();
      expect(setupGuide).toContain(`VS Code** version ${minimumVersion} or later`);

      for (const document of [setupGuide, configReference]) {
        expect(document).not.toContain('perl-lsp.enableDiagnostics');
        expect(document).not.toContain('perl-lsp.enableRefactoring');
      }
    });
  });

  describe('debugger configuration', () => {
    test('registers perl debugger type', () => {
      const debuggers = pkg.contributes.debuggers;
      expect(debuggers).toBeDefined();
      const perlDebug = required(
        debuggers.find((debuggerContribution) => debuggerContribution.type === 'perl'),
        'perl debugger contribution',
      );
      expect(perlDebug).toBeDefined();
    });

    test('debugger launch requires program property', () => {
      const perlDebug = required(
        pkg.contributes.debuggers.find(
          (debuggerContribution) => debuggerContribution.type === 'perl',
        ),
        'perl debugger contribution',
      );
      expect(perlDebug.configurationAttributes.launch.required).toContain('program');
    });

    test('debugger provides initial configurations', () => {
      const perlDebug = required(
        pkg.contributes.debuggers.find(
          (debuggerContribution) => debuggerContribution.type === 'perl',
        ),
        'perl debugger contribution',
      );
      expect(perlDebug.initialConfigurations.length).toBeGreaterThanOrEqual(2);
    });
  });

  describe('breakpoints', () => {
    test('enables breakpoints for perl language', () => {
      const breakpoints = pkg.contributes.breakpoints;
      expect(breakpoints).toBeDefined();
      const hasPerl = breakpoints.some((breakpoint) => breakpoint.language === 'perl');
      expect(hasPerl).toBe(true);
    });
  });

  describe('keybindings', () => {
    test('defines keybindings for key commands', () => {
      const keybindings = pkg.contributes.keybindings;
      expect(keybindings).toBeDefined();
      const commands = keybindings.map((keybinding) => keybinding.command);
      expect(commands).toContain('perl-lsp.organizeImports');
      expect(commands).toContain('perl-lsp.runTests');
      expect(commands).toContain('perl-lsp.restart');
    });

    test('defines Shift+Alt+V keybinding for extractVariable', () => {
      const keybindings = pkg.contributes.keybindings;
      const kb = required(
        keybindings.find((keybinding) => keybinding.command === 'perl-lsp.extractVariable'),
        'extractVariable keybinding',
      );
      expect(kb).toBeDefined();
      expect(kb.key.toLowerCase()).toBe('shift+alt+v');
    });

    test('defines Shift+Alt+M keybinding for extractMethod', () => {
      const keybindings = pkg.contributes.keybindings;
      const kb = required(
        keybindings.find((keybinding) => keybinding.command === 'perl-lsp.extractMethod'),
        'extractMethod keybinding',
      );
      expect(kb).toBeDefined();
      expect(kb.key.toLowerCase()).toBe('shift+alt+m');
    });

    test('refactoring keybindings are scoped to perl with selection', () => {
      const keybindings = pkg.contributes.keybindings;
      const extractVarKb = required(
        keybindings.find((keybinding) => keybinding.command === 'perl-lsp.extractVariable'),
        'extractVariable keybinding',
      );
      const extractMethodKb = required(
        keybindings.find((keybinding) => keybinding.command === 'perl-lsp.extractMethod'),
        'extractMethod keybinding',
      );
      expect(extractVarKb.when).toContain('editorLangId == perl');
      expect(extractMethodKb.when).toContain('editorLangId == perl');
    });

    test('keybindings are scoped to perl language', () => {
      for (const kb of pkg.contributes.keybindings) {
        expect(kb.when).toContain('editorLangId == perl');
      }
    });
  });

  describe('reportIssue command', () => {
    test('registers perl-lsp.reportIssue command', () => {
      const commandIds = pkg.contributes.commands.map((command) => command.command);
      expect(commandIds).toContain('perl-lsp.reportIssue');
    });

    test('reportIssue has Perl category', () => {
      const cmd = required(
        pkg.contributes.commands.find((command) => command.command === 'perl-lsp.reportIssue'),
        'reportIssue command',
      );
      expect(cmd).toBeDefined();
      expect(cmd.category).toBe('Perl');
    });

    test('reportIssue has the title "Report Issue"', () => {
      const cmd = required(
        pkg.contributes.commands.find((command) => command.command === 'perl-lsp.reportIssue'),
        'reportIssue command',
      );
      expect(cmd).toBeDefined();
      expect(cmd.title).toBe('Report Issue');
    });

    test('reportIssue is listed in commandPalette unconditionally (no when clause)', () => {
      const palette = pkg.contributes.menus.commandPalette;
      const entry = required(
        palette.find((menuEntry) => menuEntry.command === 'perl-lsp.reportIssue'),
        'reportIssue palette entry',
      );
      expect(entry).toBeDefined();
      // Must be unconditionally available — users need to report startup failures
      // even with no Perl file open. A missing/undefined 'when' means always-shown.
      expect(entry.when).toBeUndefined();
    });
  });

  describe('createDebugConfig command', () => {
    test('registers perl-lsp.createDebugConfig command', () => {
      const commandIds = pkg.contributes.commands.map((command) => command.command);
      expect(commandIds).toContain('perl-lsp.createDebugConfig');
    });

    test('createDebugConfig has Perl category', () => {
      const cmd = required(
        pkg.contributes.commands.find(
          (command) => command.command === 'perl-lsp.createDebugConfig',
        ),
        'createDebugConfig command',
      );
      expect(cmd.category).toBe('Perl');
    });

    test('createDebugConfig is listed in commandPalette', () => {
      const palette = pkg.contributes.menus.commandPalette;
      const entry = required(
        palette.find((menuEntry) => menuEntry.command === 'perl-lsp.createDebugConfig'),
        'createDebugConfig palette entry',
      );
      expect(entry).toBeDefined();
    });
  });

  describe('grammar', () => {
    test('registers source.perl scope', () => {
      const grammars = pkg.contributes.grammars;
      const perl = required(
        grammars.find((grammarContribution) => grammarContribution.language === 'perl'),
        'perl grammar contribution',
      );
      expect(perl).toBeDefined();
      expect(perl.scopeName).toBe('source.perl');
    });

    test('registers source.gherkin scope', () => {
      const grammars = pkg.contributes.grammars;
      const gherkin = required(
        grammars.find((grammarContribution) => grammarContribution.language === 'gherkin'),
        'gherkin grammar contribution',
      );
      expect(gherkin).toBeDefined();
      expect(gherkin.scopeName).toBe('source.gherkin');
    });

    test('grammar file exists', () => {
      const grammars = pkg.contributes.grammars;
      const perl = required(
        grammars.find((grammarContribution) => grammarContribution.language === 'perl'),
        'perl grammar contribution',
      );
      const grammarPath = path.join(EXT_ROOT, perl.path);
      expect(fs.existsSync(grammarPath)).toBe(true);
    });

    test('gherkin grammar file exists', () => {
      const grammars = pkg.contributes.grammars;
      const gherkin = required(
        grammars.find((grammarContribution) => grammarContribution.language === 'gherkin'),
        'gherkin grammar contribution',
      );
      const grammarPath = path.join(EXT_ROOT, gherkin.path);
      expect(fs.existsSync(grammarPath)).toBe(true);
    });

    test('grammar includes common XS directives', () => {
      const grammar = readJson<GrammarFile>('syntaxes/perl.tmLanguage.json');
      const keywordPattern = grammar.repository.keywords.patterns
        .map((entry) => entry.match)
        .find(
          (match: string) =>
            typeof match === 'string' &&
            match.includes('MODULE') &&
            match.includes('PACKAGE') &&
            match.includes('PPCODE') &&
            match.includes('INPUT') &&
            match.includes('OUTPUT'),
        );

      expect(keywordPattern).toBeDefined();
    });

    test('grammar includes common SWIG directives', () => {
      const grammar = readJson<GrammarFile>('syntaxes/perl.tmLanguage.json');
      const swigPattern = required(
        grammar.repository.swig.patterns.find((entry) => entry.name === 'keyword.other.perl.swig'),
        'SWIG grammar pattern',
      );

      expect(swigPattern).toBeDefined();
      expect(swigPattern.match).toContain(
        'module|include|inline|header|wrapper|init|perlcode|perl5',
      );
    });

    test('grammar maps SWIG embedded blocks to C and Perl languages', () => {
      const pkg = readJson<PackageManifest>('package.json');
      const grammar = required(
        pkg.contributes.grammars.find(
          (grammarContribution) => grammarContribution.language === 'perl',
        ),
        'perl grammar contribution',
      );
      expect(grammar.embeddedLanguages['meta.embedded.block.c.perl']).toBe('c');
      expect(grammar.embeddedLanguages['meta.embedded.block.perl.perl']).toBe('perl');
    });

    test('gherkin grammar highlights core keywords and step lines', () => {
      const grammar = readJson<GrammarFile>('syntaxes/gherkin.tmLanguage.json');
      const headerPattern = grammar.repository.headers.patterns
        .map((entry) => entry.match)
        .find(
          (match: string) =>
            typeof match === 'string' && match.includes('Scenario') && match.includes('Outline'),
        );
      const stepPattern = grammar.repository.steps.patterns
        .map((entry) => entry.match)
        .find(
          (match: string) =>
            typeof match === 'string' &&
            match.includes('Given') &&
            match.includes('When') &&
            match.includes('Then'),
        );

      expect(headerPattern).toBeDefined();
      expect(stepPattern).toBeDefined();
    });

    test('gherkin grammar highlights tags and tables', () => {
      const grammar = readJson<GrammarFile>('syntaxes/gherkin.tmLanguage.json');
      const tagPattern = required(grammar.repository.tags.patterns[0], 'tag grammar pattern').match;
      const tablePattern = required(
        grammar.repository.tables.patterns[0],
        'table grammar pattern',
      ).match;

      expect(tagPattern).toContain('@');
      expect(tablePattern).toContain('\\|');
    });
  });
});

// ---------------------------------------------------------------------------
// Snippet files
// ---------------------------------------------------------------------------
describe('snippets', () => {
  test('perl.json is valid JSON', () => {
    expect(() => readJson('snippets/perl.json')).not.toThrow();
  });

  test('launch.json is valid JSON', () => {
    expect(() => readJson('snippets/launch.json')).not.toThrow();
  });

  test('each perl snippet has prefix, body, and description', () => {
    const snippets = readJson<SnippetCatalog>('snippets/perl.json');
    for (const snippet of Object.values(snippets)) {
      expect(snippet.prefix).toBeTruthy();
      expect(snippet.body).toBeTruthy();
      expect(snippet.description).toBeTruthy();
    }
  });

  test('each launch snippet has prefix, body, and description', () => {
    const snippets = readJson<SnippetCatalog>('snippets/launch.json');
    for (const snippet of Object.values(snippets)) {
      expect(snippet.prefix).toBeTruthy();
      expect(snippet.body).toBeTruthy();
      expect(snippet.description).toBeTruthy();
    }
  });

  test('perl snippets cover fundamental constructs', () => {
    const snippets = readJson<SnippetCatalog>('snippets/perl.json');
    const allPrefixes = Object.values(snippets).flatMap((snippet) =>
      Array.isArray(snippet.prefix) ? snippet.prefix : [snippet.prefix],
    );
    expect(allPrefixes).toContain('sub');
    expect(allPrefixes).toContain('if');
    expect(allPrefixes).toContain('while');
    expect(allPrefixes).toContain('for');
    expect(allPrefixes).toContain('package');
    expect(allPrefixes).toContain('use');
    expect(allPrefixes).toContain('my');
  });

  test('test snippets cover Test::More basics', () => {
    const snippets = readJson<SnippetCatalog>('snippets/perl.json');
    const allPrefixes = Object.values(snippets).flatMap((snippet) =>
      Array.isArray(snippet.prefix) ? snippet.prefix : [snippet.prefix],
    );
    expect(allPrefixes).toContain('ok');
    expect(allPrefixes).toContain('is');
    expect(allPrefixes).toContain('is_deeply');
    expect(allPrefixes).toContain('done_testing');
    expect(allPrefixes).toContain('subtest');
  });
});

// ---------------------------------------------------------------------------
// Package metadata
// ---------------------------------------------------------------------------
describe('package.json metadata', () => {
  let pkg: PackageManifest;

  beforeAll(() => {
    pkg = readJson<PackageManifest>('package.json');
  });

  test('has a valid name', () => {
    expect(pkg.name).toBe('perl-lsp-rs');
  });

  test('has a valid version (semver)', () => {
    expect(pkg.version).toMatch(/^\d+\.\d+\.\d+/);
  });

  test('requires vscode ^1.88.0 or higher', () => {
    expect(pkg.engines.vscode).toBeTruthy();
  });

  test('main entry point is ./out/extension.js', () => {
    expect(pkg.main).toBe('./out/extension.js');
  });

  test('has required dependencies', () => {
    expect(pkg.dependencies['vscode-languageclient']).toBeTruthy();
  });

  test('publisher is EffortlessMetrics', () => {
    expect(pkg.publisher).toBe('EffortlessMetrics');
  });

  test('extension is workspace-kind', () => {
    expect(pkg.extensionKind).toContain('workspace');
  });

  test('supports untrusted workspaces', () => {
    expect(pkg.capabilities.untrustedWorkspaces.supported).toBe(true);
  });
});
