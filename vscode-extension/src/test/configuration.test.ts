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

function readJson(relativePath: string): any {
  const fullPath = path.join(EXT_ROOT, relativePath);
  return JSON.parse(fs.readFileSync(fullPath, 'utf8'));
}

// ---------------------------------------------------------------------------
// language-configuration.json
// ---------------------------------------------------------------------------
describe('language-configuration.json', () => {
  let langConfig: any;

  beforeAll(() => {
    langConfig = readJson('language-configuration.json');
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
    const opens = (langConfig.autoClosingPairs as any[]).map((p: any) => p.open);
    expect(opens).toContain('{');
    expect(opens).toContain('[');
    expect(opens).toContain('(');
    expect(opens).toContain('"');
    expect(opens).toContain("'");
  });

  test('auto-closing single quotes are suppressed inside strings and comments', () => {
    const singleQuote = (langConfig.autoClosingPairs as any[]).find(
      (p: any) => p.open === "'"
    );
    expect(singleQuote).toBeDefined();
    expect(singleQuote.notIn).toContain('string');
    expect(singleQuote.notIn).toContain('comment');
  });

  test('has surrounding pairs for common delimiters', () => {
    const pairs = (langConfig.surroundingPairs as [string, string][]).map(
      ([o, c]) => `${o}${c}`
    );
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
  let langConfig: any;

  beforeAll(() => {
    langConfig = readJson('gherkin-language-configuration.json');
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
    const opens = (langConfig.autoClosingPairs as any[]).map((p: any) => p.open);
    expect(opens).toContain('"');
    expect(opens).toContain("'");
  });
});

// ---------------------------------------------------------------------------
// package.json contributes
// ---------------------------------------------------------------------------
describe('package.json contributes', () => {
  let pkg: any;

  beforeAll(() => {
    pkg = readJson('package.json');
  });

  describe('language registration', () => {
    test('registers perl language', () => {
      const langs = pkg.contributes.languages;
      expect(langs).toBeDefined();
      const perl = langs.find((l: any) => l.id === 'perl');
      expect(perl).toBeDefined();
    });

    test('registers gherkin language for feature files', () => {
      const langs = pkg.contributes.languages;
      const gherkin = langs.find((l: any) => l.id === 'gherkin');
      expect(gherkin).toBeDefined();
      expect(gherkin.aliases).toContain('Gherkin');
      expect(gherkin.aliases).toContain('Cucumber');
      expect(gherkin.extensions).toContain('.feature');
      expect(gherkin.configuration).toBe('./gherkin-language-configuration.json');
    });

    test('perl language has expected file extensions', () => {
      const perl = pkg.contributes.languages.find((l: any) => l.id === 'perl');
      const exts: string[] = perl.extensions;
      expect(exts).toContain('.pl');
      expect(exts).toContain('.pm');
      expect(exts).toContain('.xs');
      expect(exts).toContain('.xsi');
      expect(exts).toContain('.t');
      expect(exts).toContain('.pod');
      expect(exts).toContain('.psgi');
      expect(exts).toContain('.cgi');
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
      const perl = pkg.contributes.languages.find((l: any) => l.id === 'perl');
      const exts: string[] = perl.extensions;
      expect(exts.filter((ext: string) => ext === '.xs' || ext === '.i')).toHaveLength(2);
    });

    test('perl language has shebang first-line detection', () => {
      const perl = pkg.contributes.languages.find((l: any) => l.id === 'perl');
      expect(perl.firstLine).toBeTruthy();
      const pattern = new RegExp(perl.firstLine);
      expect(pattern.test('#!/usr/bin/perl')).toBe(true);
      expect(pattern.test('#!/usr/bin/env perl')).toBe(true);
    });

    test('perl language aliases include "Perl" and "Perl 5"', () => {
      const perl = pkg.contributes.languages.find((l: any) => l.id === 'perl');
      expect(perl.aliases).toContain('Perl');
      expect(perl.aliases).toContain('Perl 5');
    });
  });

  describe('activation events', () => {
    test('activates on perl language', () => {
      expect(pkg.activationEvents).toContain('onLanguage:perl');
    });

    test('activates on gherkin language', () => {
      expect(pkg.activationEvents).toContain('onLanguage:gherkin');
    });

    test('activates on restart command', () => {
      expect(pkg.activationEvents).toContain('onCommand:perl-lsp.restart');
    });

    test('activates on reinstall command', () => {
      expect(pkg.activationEvents).toContain('onCommand:perl-lsp.reinstall');
    });
  });

  describe('commands', () => {
    test('registers expected commands', () => {
      const commandIds = pkg.contributes.commands.map((c: any) => c.command);
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
      const commandIds = pkg.contributes.commands.map((c: any) => c.command);
      expect(commandIds).toContain('perl-lsp.extractVariable');
      expect(commandIds).toContain('perl-lsp.extractMethod');
      expect(commandIds).toContain('perl-lsp.showRefactoringOptions');
    });

    test('refactoring commands have descriptive titles', () => {
      const cmds: any[] = pkg.contributes.commands;
      const extractVar = cmds.find((c: any) => c.command === 'perl-lsp.extractVariable');
      const extractMethod = cmds.find((c: any) => c.command === 'perl-lsp.extractMethod');
      const showRefactoring = cmds.find((c: any) => c.command === 'perl-lsp.showRefactoringOptions');
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
    let properties: Record<string, any>;
    let configSections: any[];

    beforeAll(() => {
      const configuration = pkg.contributes.configuration;
      // Support both array (grouped) and legacy single-object formats.
      configSections = Array.isArray(configuration) ? configuration : [configuration];
      properties = Object.assign(
        {},
        ...configSections.map((s: any) => s.properties ?? {})
      );
    });

    // --- Grouping structure ---

    test('configuration is an array with three named groups', () => {
      expect(Array.isArray(pkg.contributes.configuration)).toBe(true);
      const titles: string[] = configSections.map((s: any) => s.title);
      expect(titles.some(t => /core/i.test(t))).toBe(true);
      expect(titles.some(t => /editor/i.test(t))).toBe(true);
      expect(titles.some(t => /advanced/i.test(t))).toBe(true);
    });

    test('Core group contains serverPath, autoDownload, includePaths, enableDiagnostics', () => {
      const coreSection = configSections.find((s: any) => /core/i.test(s.title));
      expect(coreSection).toBeDefined();
      const keys = Object.keys(coreSection.properties);
      expect(keys).toContain('perl-lsp.serverPath');
      expect(keys).toContain('perl-lsp.autoDownload');
      expect(keys).toContain('perl-lsp.includePaths');
      expect(keys).toContain('perl-lsp.enableDiagnostics');
    });

    test('Editor group contains formatting, refactoring, and test integration settings', () => {
      const editorSection = configSections.find((s: any) => /editor/i.test(s.title));
      expect(editorSection).toBeDefined();
      const keys = Object.keys(editorSection.properties);
      expect(keys).toContain('perl-lsp.enableFormatting');
      expect(keys).toContain('perl-lsp.formatOnSave');
      expect(keys).toContain('perl-lsp.enableRefactoring');
      expect(keys).toContain('perl-lsp.enableTestIntegration');
      expect(keys).toContain('perl-lsp.perlcritic.enabled');
      expect(keys).toContain('perl-lsp.perlcritic.severity');
      expect(keys).toContain('perl-lsp.perlcritic.profile');
      expect(keys).toContain('perl-lsp.perlcritic.theme');
    });

    test('Advanced group contains featureProfile, trace.server, channel, downloadBaseUrl', () => {
      const advancedSection = configSections.find((s: any) => /advanced/i.test(s.title));
      expect(advancedSection).toBeDefined();
      const keys = Object.keys(advancedSection.properties);
      expect(keys).toContain('perl-lsp.featureProfile');
      expect(keys).toContain('perl-lsp.trace.server');
      expect(keys).toContain('perl-lsp.channel');
      expect(keys).toContain('perl-lsp.downloadBaseUrl');
    });

    test('all settings have an order field', () => {
      for (const [key, setting] of Object.entries<any>(properties)) {
        expect(typeof setting.order).toBe('number');
      }
    });

    test('all settings have a description field (plain-text fallback for non-markdown contexts)', () => {
      for (const [key, setting] of Object.entries<any>(properties)) {
        expect(typeof setting.description).toBe('string');
        expect(setting.description.length).toBeGreaterThan(10);
      }
    });

    test('all settings have a type field', () => {
      for (const [key, setting] of Object.entries<any>(properties)) {
        expect(setting.type).toBeTruthy();
      }
    });

    test('all settings have a default value', () => {
      for (const [key, setting] of Object.entries<any>(properties)) {
        expect(setting).toHaveProperty('default');
      }
    });

    // --- Individual setting contracts ---

    test('defines serverPath setting', () => {
      expect(properties['perl-lsp.serverPath']).toBeDefined();
      expect(properties['perl-lsp.serverPath'].type).toBe('string');
    });

    test('defines autoDownload setting with default true', () => {
      expect(properties['perl-lsp.autoDownload']).toBeDefined();
      expect(properties['perl-lsp.autoDownload'].default).toBe(true);
    });

    test('defines trace.server with valid enum values', () => {
      const trace = properties['perl-lsp.trace.server'];
      expect(trace).toBeDefined();
      expect(trace.enum).toContain('off');
      expect(trace.enum).toContain('messages');
      expect(trace.enum).toContain('verbose');
    });

    test('defines channel setting with valid enum', () => {
      const channel = properties['perl-lsp.channel'];
      expect(channel).toBeDefined();
      expect(channel.enum).toContain('latest');
      expect(channel.enum).toContain('stable');
      expect(channel.enum).toContain('tag');
    });

    test('defines featureProfile setting with valid enum', () => {
      const profile = properties['perl-lsp.featureProfile'];
      expect(profile).toBeDefined();
      expect(profile.enum).toContain('auto');
      expect(profile.enum).toContain('ga');
      expect(profile.enum).toContain('all');
    });

    test('featureProfile has enumDescriptions for every enum value', () => {
      const profile = properties['perl-lsp.featureProfile'];
      expect(Array.isArray(profile.enumDescriptions)).toBe(true);
      expect(profile.enumDescriptions.length).toBe(profile.enum.length);
      for (const desc of profile.enumDescriptions) {
        expect(typeof desc).toBe('string');
        expect(desc.length).toBeGreaterThan(0);
      }
    });

    test('trace.server has enumDescriptions matching its enum values', () => {
      const trace = properties['perl-lsp.trace.server'];
      expect(Array.isArray(trace.enumDescriptions)).toBe(true);
      expect(trace.enumDescriptions.length).toBe(trace.enum.length);
      for (const desc of trace.enumDescriptions) {
        expect(typeof desc).toBe('string');
        expect(desc.length).toBeGreaterThan(0);
      }
    });

    test('channel has enumDescriptions matching its enum values', () => {
      const channel = properties['perl-lsp.channel'];
      expect(Array.isArray(channel.enumDescriptions)).toBe(true);
      expect(channel.enumDescriptions.length).toBe(channel.enum.length);
      for (const desc of channel.enumDescriptions) {
        expect(typeof desc).toBe('string');
        expect(desc.length).toBeGreaterThan(0);
      }
    });

    test('defines enableDiagnostics with default true', () => {
      expect(properties['perl-lsp.enableDiagnostics'].default).toBe(true);
    });

    test('defines enableSemanticTokens with default true', () => {
      expect(properties['perl-lsp.enableSemanticTokens'].default).toBe(true);
    });

    test('defines formatOnSave with default false', () => {
      expect(properties['perl-lsp.formatOnSave'].default).toBe(false);
    });

    test('defines includePaths with sensible defaults', () => {
      const includePaths = properties['perl-lsp.includePaths'];
      expect(includePaths.default).toContain('lib');
      expect(includePaths.default).toContain('local/lib/perl5');
    });

    test('defines perlcritic.enabled with default false', () => {
      const setting = properties['perl-lsp.perlcritic.enabled'];
      expect(setting).toBeDefined();
      expect(setting.type).toBe('boolean');
      expect(setting.default).toBe(false);
    });

    test('defines perlcritic.severity as a 1-5 picker with default 3', () => {
      const setting = properties['perl-lsp.perlcritic.severity'];
      expect(setting).toBeDefined();
      expect(setting.type).toBe('number');
      expect(setting.enum).toEqual([1, 2, 3, 4, 5]);
      expect(setting.enumDescriptions).toHaveLength(5);
      expect(setting.default).toBe(3);
    });

    test('defines perlcritic.profile as a string setting', () => {
      const setting = properties['perl-lsp.perlcritic.profile'];
      expect(setting).toBeDefined();
      expect(setting.type).toBe('string');
      expect(setting.default).toBe('');
    });

    test('defines perlcritic.theme as a string setting', () => {
      const setting = properties['perl-lsp.perlcritic.theme'];
      expect(setting).toBeDefined();
      expect(setting.type).toBe('string');
      expect(setting.default).toBe('');
    });

    test('includePaths markdownDescription mentions module-not-found guidance', () => {
      const desc: string = properties['perl-lsp.includePaths'].markdownDescription;
      // Must mention the "Can't locate" symptom so users know what to search for
      expect(desc).toMatch(/can't locate/i);
    });

    test('includePaths has items schema typed as string', () => {
      const includePaths = properties['perl-lsp.includePaths'];
      expect(includePaths.items).toBeDefined();
      expect(includePaths.items.type).toBe('string');
    });

    test('defines downloadBaseUrl for internal hosting', () => {
      const setting = properties['perl-lsp.downloadBaseUrl'];
      expect(setting).toBeDefined();
      expect(setting.type).toBe('string');
      expect(setting.scope).toBe('machine');
    });

    test('defines autoPopulateNewFiles with default true', () => {
      const setting = properties['perl-lsp.autoPopulateNewFiles'];
      expect(setting).toBeDefined();
      expect(setting.type).toBe('boolean');
      expect(setting.default).toBe(true);
    });

    test('defines updateCheckInterval setting used by background update checker', () => {
      const setting = properties['perl-lsp.updateCheckInterval'];
      expect(setting).toBeDefined();
      expect(setting.type).toBe('number');
      expect(setting.default).toBe(24);
      // minimum of 0 means "disable"
      expect(setting.minimum).toBe(0);
    });

    test('defines autoUpdate setting used by silent updater', () => {
      const setting = properties['perl-lsp.autoUpdate'];
      expect(setting).toBeDefined();
      expect(setting.type).toBe('boolean');
      expect(setting.default).toBe(false);
    });

    test('machine-scoped settings use scope machine', () => {
      // Settings that store binary/system paths must be machine-scoped so
      // remote/container environments get the correct binary path.
      const machineScoped = ['perl-lsp.serverPath', 'perl-lsp.downloadBaseUrl', 'perl-lsp.channel', 'perl-lsp.versionTag', 'perl-lsp.autoDownload', 'perl-lsp.updateCheckInterval', 'perl-lsp.autoUpdate'];
      for (const key of machineScoped) {
        expect(properties[key]?.scope).toBe('machine');
      }
    });

    test('resource-scoped settings use scope resource', () => {
      // Per-file/workspace settings should be resource-scoped so they can be
      // overridden in workspace and folder settings.
      const resourceScoped = ['perl-lsp.includePaths', 'perl-lsp.enableDiagnostics', 'perl-lsp.enableSemanticTokens', 'perl-lsp.enableFormatting', 'perl-lsp.formatOnSave', 'perl-lsp.perltidyConfig', 'perl-lsp.perlcritic.enabled', 'perl-lsp.perlcritic.severity', 'perl-lsp.perlcritic.profile', 'perl-lsp.perlcritic.theme', 'perl-lsp.enableRefactoring', 'perl-lsp.enableTestIntegration', 'perl-lsp.autoPopulateNewFiles'];
      for (const key of resourceScoped) {
        expect(properties[key]?.scope).toBe('resource');
      }
    });

    test('disabledFeatures items have an enum for VS Code settings UI picker', () => {
      const setting = properties['perl-lsp.disabledFeatures'];
      expect(setting.items?.enum).toBeDefined();
      expect(Array.isArray(setting.items.enum)).toBe(true);
      expect(setting.items.enum.length).toBeGreaterThan(0);
    });
  });

  describe('openConfigurationGuide command', () => {
    test('registers perl-lsp.openConfigurationGuide command', () => {
      const commandIds = pkg.contributes.commands.map((c: any) => c.command);
      expect(commandIds).toContain('perl-lsp.openConfigurationGuide');
    });

    test('openConfigurationGuide has Perl category', () => {
      const cmd = pkg.contributes.commands.find(
        (c: any) => c.command === 'perl-lsp.openConfigurationGuide'
      );
      expect(cmd.category).toBe('Perl');
    });

    test('openConfigurationGuide is listed in commandPalette without language restriction', () => {
      const palette = pkg.contributes.menus.commandPalette;
      const entry = palette.find((e: any) => e.command === 'perl-lsp.openConfigurationGuide');
      expect(entry).toBeDefined();
      // Should be available globally (no when clause restricting to perl)
      expect(entry.when ?? '').not.toMatch(/editorLangId/);
    });

    test('openConfigurationGuide has an activation event', () => {
      expect(pkg.activationEvents).toContain('onCommand:perl-lsp.openConfigurationGuide');
    });
  });

  describe('debugger configuration', () => {
    test('registers perl debugger type', () => {
      const debuggers = pkg.contributes.debuggers;
      expect(debuggers).toBeDefined();
      const perlDebug = debuggers.find((d: any) => d.type === 'perl');
      expect(perlDebug).toBeDefined();
    });

    test('debugger launch requires program property', () => {
      const perlDebug = pkg.contributes.debuggers.find((d: any) => d.type === 'perl');
      expect(perlDebug.configurationAttributes.launch.required).toContain('program');
    });

    test('debugger provides initial configurations', () => {
      const perlDebug = pkg.contributes.debuggers.find((d: any) => d.type === 'perl');
      expect(perlDebug.initialConfigurations.length).toBeGreaterThanOrEqual(2);
    });
  });

  describe('breakpoints', () => {
    test('enables breakpoints for perl language', () => {
      const breakpoints = pkg.contributes.breakpoints;
      expect(breakpoints).toBeDefined();
      const hasPerl = breakpoints.some((b: any) => b.language === 'perl');
      expect(hasPerl).toBe(true);
    });
  });

  describe('keybindings', () => {
    test('defines keybindings for key commands', () => {
      const keybindings = pkg.contributes.keybindings;
      expect(keybindings).toBeDefined();
      const commands = keybindings.map((k: any) => k.command);
      expect(commands).toContain('perl-lsp.organizeImports');
      expect(commands).toContain('perl-lsp.runTests');
      expect(commands).toContain('perl-lsp.restart');
    });

    test('defines Shift+Alt+V keybinding for extractVariable', () => {
      const keybindings: any[] = pkg.contributes.keybindings;
      const kb = keybindings.find((k: any) => k.command === 'perl-lsp.extractVariable');
      expect(kb).toBeDefined();
      expect(kb.key.toLowerCase()).toBe('shift+alt+v');
    });

    test('defines Shift+Alt+M keybinding for extractMethod', () => {
      const keybindings: any[] = pkg.contributes.keybindings;
      const kb = keybindings.find((k: any) => k.command === 'perl-lsp.extractMethod');
      expect(kb).toBeDefined();
      expect(kb.key.toLowerCase()).toBe('shift+alt+m');
    });

    test('refactoring keybindings are scoped to perl with selection', () => {
      const keybindings: any[] = pkg.contributes.keybindings;
      const extractVarKb = keybindings.find((k: any) => k.command === 'perl-lsp.extractVariable');
      const extractMethodKb = keybindings.find((k: any) => k.command === 'perl-lsp.extractMethod');
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
      const commandIds = pkg.contributes.commands.map((c: any) => c.command);
      expect(commandIds).toContain('perl-lsp.reportIssue');
    });

    test('reportIssue has Perl category', () => {
      const cmd = pkg.contributes.commands.find(
        (c: any) => c.command === 'perl-lsp.reportIssue'
      );
      expect(cmd).toBeDefined();
      expect(cmd.category).toBe('Perl');
    });

    test('reportIssue has the title "Report Issue"', () => {
      const cmd = pkg.contributes.commands.find(
        (c: any) => c.command === 'perl-lsp.reportIssue'
      );
      expect(cmd).toBeDefined();
      expect(cmd.title).toBe('Report Issue');
    });

    test('reportIssue is listed in commandPalette unconditionally (no when clause)', () => {
      const palette = pkg.contributes.menus.commandPalette;
      const entry = palette.find((e: any) => e.command === 'perl-lsp.reportIssue');
      expect(entry).toBeDefined();
      // Must be unconditionally available — users need to report startup failures
      // even with no Perl file open. A missing/undefined 'when' means always-shown.
      expect(entry.when).toBeUndefined();
    });
  });

  describe('createDebugConfig command', () => {
    test('registers perl-lsp.createDebugConfig command', () => {
      const commandIds = pkg.contributes.commands.map((c: any) => c.command);
      expect(commandIds).toContain('perl-lsp.createDebugConfig');
    });

    test('createDebugConfig has Perl category', () => {
      const cmd = pkg.contributes.commands.find(
        (c: any) => c.command === 'perl-lsp.createDebugConfig'
      );
      expect(cmd.category).toBe('Perl');
    });

    test('createDebugConfig is listed in commandPalette', () => {
      const palette = pkg.contributes.menus.commandPalette;
      const entry = palette.find((e: any) => e.command === 'perl-lsp.createDebugConfig');
      expect(entry).toBeDefined();
    });

    test('createDebugConfig has an activation event', () => {
      expect(pkg.activationEvents).toContain('onCommand:perl-lsp.createDebugConfig');
    });
  });

  describe('grammar', () => {
    test('registers source.perl scope', () => {
      const grammars = pkg.contributes.grammars;
      const perl = grammars.find((g: any) => g.language === 'perl');
      expect(perl).toBeDefined();
      expect(perl.scopeName).toBe('source.perl');
    });

    test('registers source.gherkin scope', () => {
      const grammars = pkg.contributes.grammars;
      const gherkin = grammars.find((g: any) => g.language === 'gherkin');
      expect(gherkin).toBeDefined();
      expect(gherkin.scopeName).toBe('source.gherkin');
    });

    test('grammar file exists', () => {
      const grammars = pkg.contributes.grammars;
      const perl = grammars.find((g: any) => g.language === 'perl');
      const grammarPath = path.join(EXT_ROOT, perl.path);
      expect(fs.existsSync(grammarPath)).toBe(true);
    });

    test('gherkin grammar file exists', () => {
      const grammars = pkg.contributes.grammars;
      const gherkin = grammars.find((g: any) => g.language === 'gherkin');
      const grammarPath = path.join(EXT_ROOT, gherkin.path);
      expect(fs.existsSync(grammarPath)).toBe(true);
    });

    test('grammar includes common XS directives', () => {
      const grammar = readJson('syntaxes/perl.tmLanguage.json');
      const keywordPattern = grammar.repository.keywords.patterns
        .map((entry: any) => entry.match)
        .find((match: string) =>
          typeof match === 'string' &&
          match.includes('MODULE') &&
          match.includes('PACKAGE') &&
          match.includes('PPCODE') &&
          match.includes('INPUT') &&
          match.includes('OUTPUT')
        );

      expect(keywordPattern).toBeDefined();
    });

    test('grammar includes common SWIG directives', () => {
      const grammar = readJson('syntaxes/perl.tmLanguage.json');
      const swigPattern = grammar.repository.swig.patterns.find(
        (entry: any) => entry.name === 'keyword.other.perl.swig'
      );

      expect(swigPattern).toBeDefined();
      expect(swigPattern.match).toContain('module|include|inline|header|wrapper|init|perlcode|perl5');
    });

    test('grammar maps SWIG embedded blocks to C and Perl languages', () => {
      const pkg = readJson('package.json');
      const grammar = pkg.contributes.grammars.find((g: any) => g.language === 'perl');
      expect(grammar.embeddedLanguages['meta.embedded.block.c.perl']).toBe('c');
      expect(grammar.embeddedLanguages['meta.embedded.block.perl.perl']).toBe('perl');
    });

    test('gherkin grammar highlights core keywords and step lines', () => {
      const grammar = readJson('syntaxes/gherkin.tmLanguage.json');
      const headerPattern = grammar.repository.headers.patterns
        .map((entry: any) => entry.match)
        .find((match: string) =>
          typeof match === 'string' &&
          match.includes('Scenario') &&
          match.includes('Outline')
        );
      const stepPattern = grammar.repository.steps.patterns
        .map((entry: any) => entry.match)
        .find((match: string) =>
          typeof match === 'string' &&
          match.includes('Given') &&
          match.includes('When') &&
          match.includes('Then')
        );

      expect(headerPattern).toBeDefined();
      expect(stepPattern).toBeDefined();
    });

    test('gherkin grammar highlights tags and tables', () => {
      const grammar = readJson('syntaxes/gherkin.tmLanguage.json');
      const tagPattern = grammar.repository.tags.patterns[0]?.match;
      const tablePattern = grammar.repository.tables.patterns[0]?.match;

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
    const snippets = readJson('snippets/perl.json');
    for (const [name, snippet] of Object.entries<any>(snippets)) {
      expect(snippet.prefix).toBeTruthy();
      expect(snippet.body).toBeTruthy();
      expect(snippet.description).toBeTruthy();
    }
  });

  test('each launch snippet has prefix, body, and description', () => {
    const snippets = readJson('snippets/launch.json');
    for (const [name, snippet] of Object.entries<any>(snippets)) {
      expect(snippet.prefix).toBeTruthy();
      expect(snippet.body).toBeTruthy();
      expect(snippet.description).toBeTruthy();
    }
  });

  test('perl snippets cover fundamental constructs', () => {
    const snippets = readJson('snippets/perl.json');
    const allPrefixes = Object.values<any>(snippets).flatMap((s: any) =>
      Array.isArray(s.prefix) ? s.prefix : [s.prefix]
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
    const snippets = readJson('snippets/perl.json');
    const allPrefixes = Object.values<any>(snippets).flatMap((s: any) =>
      Array.isArray(s.prefix) ? s.prefix : [s.prefix]
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
  let pkg: any;

  beforeAll(() => {
    pkg = readJson('package.json');
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
