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

function readPackageJson(): any {
  return JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
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
  let pkg: any;
  let commandIds: string[];
  let paletteEntries: any[];

  beforeAll(() => {
    pkg = readPackageJson();
    commandIds = pkg.contributes.commands.map((c: any) => c.command);
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
        const cmd = pkg.contributes.commands.find((c: any) => c.command === id);
        expect(cmd?.category).toBe('Perl');
        expect(cmd?.title).toBeTruthy();
      }
    });
  });

  describe('command palette entries', () => {
    for (const id of NEW_COMMAND_IDS) {
      test(`${id} appears in command palette`, () => {
        const entry = paletteEntries.find((e: any) => e.command === id);
        expect(entry).toBeDefined();
      });
    }

    test('checkSyntax is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.checkSyntax');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('runCurrentTest is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.runCurrentTest');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('runTestAtCursor is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.runTestAtCursor');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('runAllTests is guarded by workspaceFolderCount >= 1', () => {
      const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.runAllTests');
      expect(entry?.when).toContain('workspaceFolderCount >= 1');
    });

    test('formatDocument is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.formatDocument');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('runPerlCritic is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.runPerlCritic');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('setPerlCriticSeverity is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.setPerlCriticSeverity');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('showIncPaths is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.showIncPaths');
      expect(entry?.when).toContain('editorLangId == perl');
    });

    test('openModule is guarded by workspaceFolderCount >= 1', () => {
      const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.openModule');
      expect(entry?.when).toContain('workspaceFolderCount >= 1');
    });

    test('showParserAst is guarded by editorLangId == perl', () => {
      const entry = paletteEntries.find((e: any) => e.command === 'perl-lsp.showParserAst');
      expect(entry?.when).toContain('editorLangId == perl');
    });
  });

  describe('activation events', () => {
    test('every contributed command has an activation event', () => {
      for (const command of pkg.contributes.commands) {
        expect(pkg.activationEvents).toContain(`onCommand:${command.command}`);
      }
    });

    for (const id of NEW_COMMAND_IDS) {
      test(`activates on ${id} command`, () => {
        expect(pkg.activationEvents).toContain(`onCommand:${id}`);
      });
    }
  });
});

// ---------------------------------------------------------------------------
// extractVariable
// ---------------------------------------------------------------------------
describe('perl-lsp.extractVariable command', () => {
  let pkg: any;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('is declared in contributes.commands', () => {
    const ids = pkg.contributes.commands.map((c: any) => c.command);
    expect(ids).toContain('perl-lsp.extractVariable');
  });

  test('has title "Extract Variable"', () => {
    const cmd = pkg.contributes.commands.find((c: any) => c.command === 'perl-lsp.extractVariable');
    expect(cmd).toBeDefined();
    expect(cmd.title).toBe('Extract Variable');
  });

  test('has Perl category', () => {
    const cmd = pkg.contributes.commands.find((c: any) => c.command === 'perl-lsp.extractVariable');
    expect(cmd.category).toBe('Perl');
  });

  test('is listed in commandPalette restricted to perl with a selection', () => {
    const palette = pkg.contributes.menus.commandPalette;
    const entry = palette.find((e: any) => e.command === 'perl-lsp.extractVariable');
    expect(entry).toBeDefined();
    expect(entry.when).toContain('editorLangId == perl');
    expect(entry.when).toContain('editorHasSelection');
  });

  test('has Shift+Alt+V keybinding scoped to perl with selection', () => {
    const keybindings: any[] = pkg.contributes.keybindings;
    const kb = keybindings.find((k: any) => k.command === 'perl-lsp.extractVariable');
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
  let pkg: any;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('is declared in contributes.commands', () => {
    const ids = pkg.contributes.commands.map((c: any) => c.command);
    expect(ids).toContain('perl-lsp.runTestAtCursor');
  });

  test('has title "Run Test at Cursor"', () => {
    const cmd = pkg.contributes.commands.find((c: any) => c.command === 'perl-lsp.runTestAtCursor');
    expect(cmd).toBeDefined();
    expect(cmd.title).toBe('Run Test at Cursor');
  });

  test('has a command palette entry guarded by editorLangId == perl', () => {
    const palette = pkg.contributes.menus.commandPalette;
    const entry = palette.find((e: any) => e.command === 'perl-lsp.runTestAtCursor');
    expect(entry).toBeDefined();
    expect(entry.when).toContain('editorLangId == perl');
  });

  test('appears in the editor context menu', () => {
    const contextMenu = pkg.contributes.menus['editor/context'];
    const entry = contextMenu.find((e: any) => e.command === 'perl-lsp.runTestAtCursor');
    expect(entry).toBeDefined();
    expect(entry.when).toContain('editorLangId == perl');
  });

  test('has a keyboard shortcut', () => {
    const keybindings: any[] = pkg.contributes.keybindings;
    const kb = keybindings.find((k: any) => k.command === 'perl-lsp.runTestAtCursor');
    expect(kb).toBeDefined();
    expect(kb.key.toLowerCase()).toBe('ctrl+alt+shift+t');
    expect(kb.when).toContain('editorLangId == perl');
  });
});

// ---------------------------------------------------------------------------
// extractMethod
// ---------------------------------------------------------------------------
describe('perl-lsp.extractMethod command', () => {
  let pkg: any;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('is declared in contributes.commands', () => {
    const ids = pkg.contributes.commands.map((c: any) => c.command);
    expect(ids).toContain('perl-lsp.extractMethod');
  });

  test('has title "Extract Method"', () => {
    const cmd = pkg.contributes.commands.find((c: any) => c.command === 'perl-lsp.extractMethod');
    expect(cmd).toBeDefined();
    expect(cmd.title).toBe('Extract Method');
  });

  test('has Perl category', () => {
    const cmd = pkg.contributes.commands.find((c: any) => c.command === 'perl-lsp.extractMethod');
    expect(cmd.category).toBe('Perl');
  });

  test('is listed in commandPalette restricted to perl with a selection', () => {
    const palette = pkg.contributes.menus.commandPalette;
    const entry = palette.find((e: any) => e.command === 'perl-lsp.extractMethod');
    expect(entry).toBeDefined();
    expect(entry.when).toContain('editorLangId == perl');
    expect(entry.when).toContain('editorHasSelection');
  });

  test('has Shift+Alt+M keybinding scoped to perl with selection', () => {
    const keybindings: any[] = pkg.contributes.keybindings;
    const kb = keybindings.find((k: any) => k.command === 'perl-lsp.extractMethod');
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
  let pkg: any;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('is declared in contributes.commands', () => {
    const ids = pkg.contributes.commands.map((c: any) => c.command);
    expect(ids).toContain('perl-lsp.showRefactoringOptions');
  });

  test('has title "Show Refactoring Options"', () => {
    const cmd = pkg.contributes.commands.find(
      (c: any) => c.command === 'perl-lsp.showRefactoringOptions'
    );
    expect(cmd).toBeDefined();
    expect(cmd.title).toBe('Show Refactoring Options');
  });

  test('has Perl category', () => {
    const cmd = pkg.contributes.commands.find(
      (c: any) => c.command === 'perl-lsp.showRefactoringOptions'
    );
    expect(cmd.category).toBe('Perl');
  });

  test('is listed in commandPalette restricted to perl', () => {
    const palette = pkg.contributes.menus.commandPalette;
    const entry = palette.find((e: any) => e.command === 'perl-lsp.showRefactoringOptions');
    expect(entry).toBeDefined();
    expect(entry.when).toContain('editorLangId == perl');
  });
});

// ---------------------------------------------------------------------------
// createDebugConfig
// ---------------------------------------------------------------------------
describe('perl-lsp.createDebugConfig command', () => {
  let pkg: any;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('is declared in contributes.commands', () => {
    const ids = pkg.contributes.commands.map((c: any) => c.command);
    expect(ids).toContain('perl-lsp.createDebugConfig');
  });

  test('has title "Create Debug Configuration"', () => {
    const cmd = pkg.contributes.commands.find((c: any) => c.command === 'perl-lsp.createDebugConfig');
    expect(cmd).toBeDefined();
    expect(cmd.title).toBe('Create Debug Configuration');
  });

  test('has Perl category', () => {
    const cmd = pkg.contributes.commands.find((c: any) => c.command === 'perl-lsp.createDebugConfig');
    expect(cmd.category).toBe('Perl');
  });

  test('is listed in commandPalette with workspace restriction', () => {
    const palette = pkg.contributes.menus.commandPalette;
    const entry = palette.find((e: any) => e.command === 'perl-lsp.createDebugConfig');
    expect(entry).toBeDefined();
    // Available when at least one workspace folder is open
    expect(entry.when).toContain('workspaceFolderCount');
  });

  test('has an activation event', () => {
    expect(pkg.activationEvents).toContain('onCommand:perl-lsp.createDebugConfig');
  });
});

// ---------------------------------------------------------------------------
// No duplicate activation events
// ---------------------------------------------------------------------------
describe('package.json activationEvents', () => {
  let pkg: any;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('has no duplicate activation events', () => {
    const events: string[] = pkg.activationEvents;
    const unique = new Set(events);
    expect(unique.size).toBe(events.length);
  });
});
