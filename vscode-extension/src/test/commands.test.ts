/**
 * Contract tests for the four previously-unimplemented VSCode commands:
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
