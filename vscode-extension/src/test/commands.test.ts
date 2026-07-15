/**
 * Contract tests for the command-palette feature set:
 *   - perl-lsp.checkSyntax
 *   - perl-lsp.runCurrentTest
 *   - perl-lsp.runTestAtCursor
 *   - perl-lsp.runAllTests
 *   - perl-lsp.formatDocument
 *   - perl-lsp.runPerlCritic
 *   - perl-lsp.setPerlCriticSeverity
 *   - perl-lsp.showIncPaths
 *   - perl-lsp.openModule
 *   - perl-lsp.showParserAst
 *
 * and the earlier four previously-unimplemented VSCode commands:
 *   - perl-lsp.extractVariable   (Shift+Alt+V)
 *   - perl-lsp.extractMethod     (Shift+Alt+M)
 *   - perl-lsp.showRefactoringOptions
 *   - perl-lsp.createDebugConfig
 *
 * These are static contract tests — they verify that package.json declares
 * the commands with correct metadata (no live VSCode extension host needed).
 * The implementation is verified in extension.ts; these tests guard regressions
 * to the manifest contract that users and keybinding tables depend on.
 */

import * as fs from 'fs';
import * as path from 'path';

const EXT_ROOT = path.resolve(__dirname, '..', '..');

type CommandContribution = { command: string; title?: string; category?: string };
type MenuEntry = { command: string; when?: string };
type Keybinding = { command: string; key: string; when?: string };
// Manifest arrays are validated by the surrounding contract assertions. This
// preserves the existing test flow after replacing untyped JSON values while
// keeping the post-assertion property access concise.
type ManifestArray<T> = Omit<T[], 'find'> & {
  find(predicate: (value: T) => boolean): T;
};
type MenuContributions = {
  commandPalette: ManifestArray<MenuEntry>;
  'editor/context': ManifestArray<MenuEntry>;
};
type ExtensionManifest = {
  activationEvents: string[];
  contributes: {
    commands: ManifestArray<CommandContribution>;
    keybindings: ManifestArray<Keybinding>;
    menus: MenuContributions;
  };
};

function readPackageJson(): ExtensionManifest {
  return JSON.parse(
    fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'),
  ) as ExtensionManifest;
}

// ---------------------------------------------------------------------------
// command palette Perl commands (issue #2058)
// ---------------------------------------------------------------------------
const NEW_COMMAND_IDS = [
  'perl-lsp.checkSyntax',
  'perl-lsp.runCurrentTest',
  'perl-lsp.runTestAtCursor',
  'perl-lsp.runAllTests',
  'perl-lsp.formatDocument',
  'perl-lsp.runPerlCritic',
  'perl-lsp.setPerlCriticSeverity',
  'perl-lsp.showIncPaths',
  'perl-lsp.openModule',
  'perl-lsp.showParserAst',
];

describe('perl-lsp command palette commands (issue #2058)', () => {
  let pkg: ExtensionManifest;
  let commandIds: string[];
  let paletteEntries: ManifestArray<MenuEntry>;

  beforeAll(() => {
    pkg = readPackageJson();
    commandIds = pkg.contributes.commands.map((c: CommandContribution) => c.command);
    paletteEntries = pkg.contributes.menus?.commandPalette ?? [];
  });

  describe('commands', () => {
    for (const id of NEW_COMMAND_IDS) {
      test(`${id} is registered`, () => {
        expect(commandIds).toContain(id);
      });
    }

    test('new commands are all classified as Perl commands', () => {
      for (const id of NEW_COMMAND_IDS) {
        const cmd = pkg.contributes.commands.find((c: CommandContribution) => c.command === id);
        expect(cmd?.category).toBe('Perl');
        expect(cmd?.title).toBeTruthy();
      }
    });
  });

  describe('command palette entries', () => {
    for (const id of NEW_COMMAND_IDS) {
      test(`${id} appears in command palette`, () => {
        const entry = paletteEntries.find((e: MenuEntry) => e.command === id);
        expect(entry).toBeDefined();
      });
    }

    test('checkSyntax is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: MenuEntry) => e.command === 'perl-lsp.checkSyntax');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('runCurrentTest is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: MenuEntry) => e.command === 'perl-lsp.runCurrentTest');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('runTestAtCursor is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: MenuEntry) => e.command === 'perl-lsp.runTestAtCursor');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('runAllTests is guarded by workspaceFolderCount >= 1', () => {
      const entry = paletteEntries.find((e: MenuEntry) => e.command === 'perl-lsp.runAllTests');
      expect(entry?.when).toContain('workspaceFolderCount >= 1');
    });

    test('formatDocument is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: MenuEntry) => e.command === 'perl-lsp.formatDocument');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('runPerlCritic is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: MenuEntry) => e.command === 'perl-lsp.runPerlCritic');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('setPerlCriticSeverity is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find(
        (e: MenuEntry) => e.command === 'perl-lsp.setPerlCriticSeverity',
      );
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('showIncPaths is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: MenuEntry) => e.command === 'perl-lsp.showIncPaths');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('openModule is guarded by workspaceFolderCount >= 1', () => {
      const entry = paletteEntries.find((e: MenuEntry) => e.command === 'perl-lsp.openModule');
      expect(entry?.when).toContain('workspaceFolderCount >= 1');
    });

    test('showParserAst is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: MenuEntry) => e.command === 'perl-lsp.showParserAst');
      expect(entry?.when).toContain('editorLangId == perl');
    });
  });

  describe('activation events', () => {
    test('contributed commands do not declare onCommand activation events', () => {
      // VS Code 1.74+ implicitly activates extensions when contributed commands are
      // invoked, so explicit `onCommand:*` activation events are redundant. This
      // extension targets `^1.88.0` (see package.json `engines.vscode`).
      const commandActivationEvents = pkg.activationEvents.filter((event: string) =>
        event.startsWith('onCommand:'),
      );

      expect(commandActivationEvents).toEqual([]);
    });
  });
});

// ---------------------------------------------------------------------------
// extractVariable
// ---------------------------------------------------------------------------
describe('perl-lsp.extractVariable command', () => {
  let pkg: ExtensionManifest;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('is declared in contributes.commands', () => {
    const ids = pkg.contributes.commands.map((c: CommandContribution) => c.command);
    expect(ids).toContain('perl-lsp.extractVariable');
  });

  test('has title "Extract Variable"', () => {
    const cmd = pkg.contributes.commands.find(
      (c: CommandContribution) => c.command === 'perl-lsp.extractVariable',
    );
    expect(cmd).toBeDefined();
    expect(cmd.title).toBe('Extract Variable');
  });

  test('has Perl category', () => {
    const cmd = pkg.contributes.commands.find(
      (c: CommandContribution) => c.command === 'perl-lsp.extractVariable',
    );
    expect(cmd.category).toBe('Perl');
  });

  test('is listed in commandPalette restricted to perl with a selection', () => {
    const palette = pkg.contributes.menus.commandPalette;
    const entry = palette.find((e: MenuEntry) => e.command === 'perl-lsp.extractVariable');
    expect(entry).toBeDefined();
    expect(entry.when).toContain('editorLangId == perl');
    expect(entry.when).toContain('editorHasSelection');
  });

  test('has Shift+Alt+V keybinding scoped to perl with selection', () => {
    const keybindings: ManifestArray<Keybinding> = pkg.contributes.keybindings;
    const kb = keybindings.find((k: Keybinding) => k.command === 'perl-lsp.extractVariable');
    expect(kb).toBeDefined();
    expect(kb.key.toLowerCase()).toBe('shift+alt+v');
    expect(kb.when).toContain('editorLangId == perl');
    expect(kb.when).toContain('editorHasSelection');
  });
});

// ---------------------------------------------------------------------------
// runTestAtCursor
// ---------------------------------------------------------------------------
describe('perl-lsp.runTestAtCursor command', () => {
  let pkg: ExtensionManifest;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('is declared in contributes.commands', () => {
    const ids = pkg.contributes.commands.map((c: CommandContribution) => c.command);
    expect(ids).toContain('perl-lsp.runTestAtCursor');
  });

  test('has title "Run Test at Cursor"', () => {
    const cmd = pkg.contributes.commands.find(
      (c: CommandContribution) => c.command === 'perl-lsp.runTestAtCursor',
    );
    expect(cmd).toBeDefined();
    expect(cmd.title).toBe('Run Test at Cursor');
  });

  test('has a command palette entry guarded by editorLangId == perl', () => {
    const palette = pkg.contributes.menus.commandPalette;
    const entry = palette.find((e: MenuEntry) => e.command === 'perl-lsp.runTestAtCursor');
    expect(entry).toBeDefined();
    expect(entry.when).toContain('editorLangId == perl');
  });

  test('appears in the editor context menu', () => {
    const contextMenu = pkg.contributes.menus['editor/context'];
    const entry = contextMenu.find((e: MenuEntry) => e.command === 'perl-lsp.runTestAtCursor');
    expect(entry).toBeDefined();
    expect(entry.when).toContain('editorLangId == perl');
  });

  test('has a keyboard shortcut', () => {
    const keybindings: ManifestArray<Keybinding> = pkg.contributes.keybindings;
    const kb = keybindings.find((k: Keybinding) => k.command === 'perl-lsp.runTestAtCursor');
    expect(kb).toBeDefined();
    expect(kb.key.toLowerCase()).toBe('ctrl+alt+shift+t');
    expect(kb.when).toContain('editorLangId == perl');
  });
});

// ---------------------------------------------------------------------------
// extractMethod
// ---------------------------------------------------------------------------
describe('perl-lsp.extractMethod command', () => {
  let pkg: ExtensionManifest;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('is declared in contributes.commands', () => {
    const ids = pkg.contributes.commands.map((c: CommandContribution) => c.command);
    expect(ids).toContain('perl-lsp.extractMethod');
  });

  test('has title "Extract Method"', () => {
    const cmd = pkg.contributes.commands.find(
      (c: CommandContribution) => c.command === 'perl-lsp.extractMethod',
    );
    expect(cmd).toBeDefined();
    expect(cmd.title).toBe('Extract Method');
  });

  test('has Perl category', () => {
    const cmd = pkg.contributes.commands.find(
      (c: CommandContribution) => c.command === 'perl-lsp.extractMethod',
    );
    expect(cmd.category).toBe('Perl');
  });

  test('is listed in commandPalette restricted to perl with a selection', () => {
    const palette = pkg.contributes.menus.commandPalette;
    const entry = palette.find((e: MenuEntry) => e.command === 'perl-lsp.extractMethod');
    expect(entry).toBeDefined();
    expect(entry.when).toContain('editorLangId == perl');
    expect(entry.when).toContain('editorHasSelection');
  });

  test('has Shift+Alt+M keybinding scoped to perl with selection', () => {
    const keybindings: ManifestArray<Keybinding> = pkg.contributes.keybindings;
    const kb = keybindings.find((k: Keybinding) => k.command === 'perl-lsp.extractMethod');
    expect(kb).toBeDefined();
    expect(kb.key.toLowerCase()).toBe('shift+alt+m');
    expect(kb.when).toContain('editorLangId == perl');
    expect(kb.when).toContain('editorHasSelection');
  });
});

// ---------------------------------------------------------------------------
// showRefactoringOptions
// ---------------------------------------------------------------------------
describe('perl-lsp.showRefactoringOptions command', () => {
  let pkg: ExtensionManifest;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('is declared in contributes.commands', () => {
    const ids = pkg.contributes.commands.map((c: CommandContribution) => c.command);
    expect(ids).toContain('perl-lsp.showRefactoringOptions');
  });

  test('has title "Show Refactoring Options"', () => {
    const cmd = pkg.contributes.commands.find(
      (c: CommandContribution) => c.command === 'perl-lsp.showRefactoringOptions',
    );
    expect(cmd).toBeDefined();
    expect(cmd.title).toBe('Show Refactoring Options');
  });

  test('has Perl category', () => {
    const cmd = pkg.contributes.commands.find(
      (c: CommandContribution) => c.command === 'perl-lsp.showRefactoringOptions',
    );
    expect(cmd.category).toBe('Perl');
  });

  test('is listed in commandPalette restricted to perl', () => {
    const palette = pkg.contributes.menus.commandPalette;
    const entry = palette.find((e: MenuEntry) => e.command === 'perl-lsp.showRefactoringOptions');
    expect(entry).toBeDefined();
    expect(entry.when).toContain('editorLangId == perl');
  });
});

// ---------------------------------------------------------------------------
// createDebugConfig
// ---------------------------------------------------------------------------
describe('perl-lsp.createDebugConfig command', () => {
  let pkg: ExtensionManifest;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('is declared in contributes.commands', () => {
    const ids = pkg.contributes.commands.map((c: CommandContribution) => c.command);
    expect(ids).toContain('perl-lsp.createDebugConfig');
  });

  test('has title "Create Debug Configuration"', () => {
    const cmd = pkg.contributes.commands.find(
      (c: CommandContribution) => c.command === 'perl-lsp.createDebugConfig',
    );
    expect(cmd).toBeDefined();
    expect(cmd.title).toBe('Create Debug Configuration');
  });

  test('has Perl category', () => {
    const cmd = pkg.contributes.commands.find(
      (c: CommandContribution) => c.command === 'perl-lsp.createDebugConfig',
    );
    expect(cmd.category).toBe('Perl');
  });

  test('is listed in commandPalette with workspace restriction', () => {
    const palette = pkg.contributes.menus.commandPalette;
    const entry = palette.find((e: MenuEntry) => e.command === 'perl-lsp.createDebugConfig');
    expect(entry).toBeDefined();
    // Available when at least one workspace folder is open
    expect(entry.when).toContain('workspaceFolderCount');
  });
});

// ---------------------------------------------------------------------------
// trust explanation commands
// ---------------------------------------------------------------------------
describe('perl-lsp trust explanation commands', () => {
  let pkg: ExtensionManifest;
  let commandIds: string[];
  let paletteEntries: ManifestArray<MenuEntry>;

  beforeAll(() => {
    pkg = readPackageJson();
    commandIds = pkg.contributes.commands.map((c: CommandContribution) => c.command);
    paletteEntries = pkg.contributes.menus.commandPalette;
  });

  test.each([
    ['perl-lsp.explainProviderDecision', 'Explain Provider Decision'],
    ['perl-lsp.previewSafeDelete', 'Preview Safe Delete'],
    ['perl-lsp.previewPackageRename', 'Preview Package Rename'],
    ['perl-lsp.copyProviderDecisionReceipt', 'Copy Provider Decision Receipt'],
    ['perl-lsp.showWorkspaceTrustReport', 'Show Workspace Trust Report'],
    ['perl-lsp.explainMissingModuleLookup', 'Explain Missing Module Lookup'],
    ['perl-lsp.explainDiagnostic', 'Explain This Diagnostic'],
  ])('%s is declared as a Perl LSP command', (id, title) => {
    const cmd = pkg.contributes.commands.find((c: CommandContribution) => c.command === id);
    expect(commandIds).toContain(id);
    expect(cmd).toBeDefined();
    expect(cmd.title).toBe(title);
    expect(cmd.category).toBe('Perl LSP');
  });

  test.each([
    'perl-lsp.explainProviderDecision',
    'perl-lsp.previewSafeDelete',
    'perl-lsp.previewPackageRename',
    'perl-lsp.copyProviderDecisionReceipt',
    'perl-lsp.explainMissingModuleLookup',
    'perl-lsp.explainDiagnostic',
  ])('%s is available from the Perl command palette', (id) => {
    const entry = paletteEntries.find((e: MenuEntry) => e.command === id);
    expect(entry).toBeDefined();
    expect(entry.when).toContain('editorLangId == perl');
  });

  test('workspace trust report is available when a workspace is open', () => {
    const entry = paletteEntries.find(
      (e: MenuEntry) => e.command === 'perl-lsp.showWorkspaceTrustReport',
    );
    expect(entry).toBeDefined();
    expect(entry.when).toContain('workspaceFolderCount');
  });
});

// ---------------------------------------------------------------------------
// No duplicate activation events
// ---------------------------------------------------------------------------
describe('package.json activationEvents', () => {
  let pkg: ExtensionManifest;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('has no duplicate activation events', () => {
    const events: string[] = pkg.activationEvents;
    const unique = new Set(events);
    expect(unique.size).toBe(events.length);
  });
});
