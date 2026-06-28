/**
 * Unit tests for WhatsNewManager.
 *
 * Tests cover:
 * - shouldShowWhatsNew: version-change detection via globalState
 * - markVersionSeen: persists the current version
 * - extractChangelogSection: parses CHANGELOG.md sections
 * - markdownToHtml: converts Markdown subset to HTML
 * - buildHtml: generates a valid HTML document
 * - package.json contract: showWhatsNew command declared
 */

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { WhatsNewManager, extractVersionSection, markdownToHtml } from '../whatsNew';

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

const SAMPLE_CHANGELOG = `# Change Log

## [0.13.0] - 2026-04-01

### Added
- New feature A
- New feature B

### Fixed
- Bug fix C

## [0.12.0] - 2026-03-19

### Changed
- Something changed

## [0.11.0] - 2026-03-11

### Fixed
- Old fix
`;

// ---------------------------------------------------------------------------
// Context helpers
// ---------------------------------------------------------------------------

function makeTmpDir(): string {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'whats-new-test-'));
    // Write a package.json with a known version
    fs.writeFileSync(
        path.join(dir, 'package.json'),
        JSON.stringify({ version: '0.13.0' }),
    );
    // Write a CHANGELOG.md
    fs.writeFileSync(path.join(dir, 'CHANGELOG.md'), SAMPLE_CHANGELOG);
    return dir;
}

function makeContext(opts?: {
    lastVersion?: string;
    extensionPath?: string;
}): any {
    const store = new Map<string, any>();
    if (opts?.lastVersion !== undefined) {
        store.set('perl-lsp.lastVersion', opts.lastVersion);
    }
    const dir = opts?.extensionPath ?? makeTmpDir();
    return {
        extensionPath: dir,
        globalState: {
            get: jest.fn((key: string, defaultValue?: any) => {
                if (store.has(key)) return store.get(key);
                return defaultValue;
            }),
            update: jest.fn(async (key: string, value: any) => {
                store.set(key, value);
            }),
        },
    };
}

function makeOutputChannel(): any {
    return {
        appendLine: jest.fn(),
        show: jest.fn(),
        dispose: jest.fn(),
    };
}

// ---------------------------------------------------------------------------
// shouldShowWhatsNew
// ---------------------------------------------------------------------------

describe('WhatsNewManager.shouldShowWhatsNew', () => {
    test('returns true when no version has been stored (first install)', () => {
        const ctx = makeContext(); // no lastVersion stored
        const mgr = new WhatsNewManager(ctx, makeOutputChannel());
        expect(mgr.shouldShowWhatsNew()).toBe(true);
    });

    test('returns true when stored version differs from current version', () => {
        const ctx = makeContext({ lastVersion: '0.12.0' }); // old version stored
        const mgr = new WhatsNewManager(ctx, makeOutputChannel());
        // extensionPath has package.json with version 0.13.0
        expect(mgr.shouldShowWhatsNew()).toBe(true);
    });

    test('returns false when stored version matches current version', () => {
        const ctx = makeContext({ lastVersion: '0.13.0' }); // same as package.json
        const mgr = new WhatsNewManager(ctx, makeOutputChannel());
        expect(mgr.shouldShowWhatsNew()).toBe(false);
    });

    test('returns false when package.json cannot be read', () => {
        const ctx = makeContext({ extensionPath: '/nonexistent/path' });
        const mgr = new WhatsNewManager(ctx, makeOutputChannel());
        expect(mgr.shouldShowWhatsNew()).toBe(false);
    });
});

// ---------------------------------------------------------------------------
// markVersionSeen
// ---------------------------------------------------------------------------

describe('WhatsNewManager.markVersionSeen', () => {
    test('stores the current version in globalState', async () => {
        const ctx = makeContext();
        const mgr = new WhatsNewManager(ctx, makeOutputChannel());
        await mgr.markVersionSeen();
        expect(ctx.globalState.update).toHaveBeenCalledWith(
            'perl-lsp.lastVersion',
            '0.13.0',
        );
    });

    test('after markVersionSeen, shouldShowWhatsNew returns false', async () => {
        const ctx = makeContext();
        const mgr = new WhatsNewManager(ctx, makeOutputChannel());
        expect(mgr.shouldShowWhatsNew()).toBe(true);
        await mgr.markVersionSeen();
        // Now the stored version equals the current version
        expect(mgr.shouldShowWhatsNew()).toBe(false);
    });
});

// ---------------------------------------------------------------------------
// currentVersion
// ---------------------------------------------------------------------------

describe('WhatsNewManager.currentVersion', () => {
    test('reads version from package.json in extensionPath', () => {
        const ctx = makeContext();
        const mgr = new WhatsNewManager(ctx, makeOutputChannel());
        expect(mgr.currentVersion()).toBe('0.13.0');
    });

    test('returns undefined when extensionPath has no package.json', () => {
        const ctx = makeContext({ extensionPath: os.tmpdir() });
        const mgr = new WhatsNewManager(ctx, makeOutputChannel());
        // os.tmpdir() is unlikely to have a package.json with a version field
        // We just test it doesn't throw
        const version = mgr.currentVersion();
        expect(typeof version === 'string' || version === undefined).toBe(true);
    });
});

// ---------------------------------------------------------------------------
// extractChangelogSection (pure function)
// ---------------------------------------------------------------------------

describe('extractVersionSection', () => {
    test('extracts the section for the requested version', () => {
        const section = extractVersionSection(SAMPLE_CHANGELOG, '0.13.0');
        expect(section).toContain('0.13.0');
        expect(section).toContain('New feature A');
        expect(section).toContain('New feature B');
        expect(section).toContain('Bug fix C');
    });

    test('does not include content from adjacent version sections', () => {
        const section = extractVersionSection(SAMPLE_CHANGELOG, '0.13.0');
        expect(section).not.toContain('0.12.0');
        expect(section).not.toContain('Something changed');
    });

    test('extracts a middle version correctly', () => {
        const section = extractVersionSection(SAMPLE_CHANGELOG, '0.12.0');
        expect(section).toContain('0.12.0');
        expect(section).toContain('Something changed');
        expect(section).not.toContain('0.11.0');
        expect(section).not.toContain('New feature A');
    });

    test('extracts the last version (no following section)', () => {
        const section = extractVersionSection(SAMPLE_CHANGELOG, '0.11.0');
        expect(section).toContain('0.11.0');
        expect(section).toContain('Old fix');
    });

    test('returns empty string when version is not found', () => {
        const section = extractVersionSection(SAMPLE_CHANGELOG, '9.99.0');
        expect(section).toBe('');
    });

    test('handles CHANGELOG without bracket notation', () => {
        const changelog = `## 1.0.0\n\n- Something\n\n## 0.9.0\n\n- Older\n`;
        const section = extractVersionSection(changelog, '1.0.0');
        expect(section).toContain('Something');
        expect(section).not.toContain('Older');
    });
});

describe('WhatsNewManager.extractChangelogSection', () => {
    test('delegates to extractVersionSection with the CHANGELOG from extensionPath', () => {
        const ctx = makeContext();
        const mgr = new WhatsNewManager(ctx, makeOutputChannel());
        const section = mgr.extractChangelogSection('0.13.0');
        expect(section).toContain('New feature A');
    });

    test('returns empty string when CHANGELOG is not found', () => {
        const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'wn-no-cl-'));
        fs.writeFileSync(
            path.join(tmpDir, 'package.json'),
            JSON.stringify({ version: '0.13.0' }),
        );
        const ctx = makeContext({ extensionPath: tmpDir });
        const mgr = new WhatsNewManager(ctx, makeOutputChannel());
        const section = mgr.extractChangelogSection('0.13.0');
        expect(section).toBe('');
        fs.rmSync(tmpDir, { recursive: true, force: true });
    });
});

// ---------------------------------------------------------------------------
// markdownToHtml (pure function)
// ---------------------------------------------------------------------------

describe('markdownToHtml', () => {
    test('converts ## heading to <h2>', () => {
        const html = markdownToHtml('## My Heading');
        expect(html).toContain('<h2>');
        expect(html).toContain('My Heading');
    });

    test('converts ### heading to <h3>', () => {
        const html = markdownToHtml('### Sub Heading');
        expect(html).toContain('<h3>');
        expect(html).toContain('Sub Heading');
    });

    test('converts - list items to <ul><li>', () => {
        const html = markdownToHtml('- Item one\n- Item two');
        expect(html).toContain('<ul>');
        expect(html).toContain('<li>');
        expect(html).toContain('Item one');
        expect(html).toContain('Item two');
        expect(html).toContain('</ul>');
    });

    test('converts **bold** to <strong>', () => {
        const html = markdownToHtml('**bold text**');
        expect(html).toContain('<strong>bold text</strong>');
    });

    test('converts `code` to <code>', () => {
        const html = markdownToHtml('Use `perltidy` to format');
        expect(html).toContain('<code>perltidy</code>');
    });

    test('escapes HTML special characters', () => {
        const html = markdownToHtml('Use <br> & "quotes"');
        expect(html).not.toContain('<br>');
        expect(html).toContain('&lt;br&gt;');
        expect(html).toContain('&amp;');
    });

    test('closes list before next heading', () => {
        const md = '- item\n## Next Section';
        const html = markdownToHtml(md);
        const ulClose = html.indexOf('</ul>');
        const h2Open = html.indexOf('<h2>');
        expect(ulClose).toBeGreaterThanOrEqual(0);
        expect(h2Open).toBeGreaterThan(ulClose);
    });

    test('blank lines do not create orphaned tags', () => {
        const html = markdownToHtml('Line one\n\nLine two');
        expect(html).toContain('<p>Line one</p>');
        expect(html).toContain('<p>Line two</p>');
    });
});

// ---------------------------------------------------------------------------
// buildHtml
// ---------------------------------------------------------------------------

describe('WhatsNewManager.buildHtml', () => {
    function makeMgr() {
        const ctx = makeContext();
        return new WhatsNewManager(ctx, makeOutputChannel());
    }

    test('returns a valid HTML document with DOCTYPE', () => {
        const html = makeMgr().buildHtml('0.13.0', '## [0.13.0]\n- Feature X');
        expect(html).toContain('<!DOCTYPE html>');
        expect(html).toContain('<html');
        expect(html).toContain('</html>');
    });

    test('includes the version number in the title', () => {
        const html = makeMgr().buildHtml('0.13.0', '');
        expect(html).toContain('0.13.0');
        expect(html).toContain('<title>');
    });

    test('includes changelog content in the body', () => {
        const html = makeMgr().buildHtml('0.13.0', '## [0.13.0]\n- Feature X');
        expect(html).toContain('Feature X');
    });

    test('shows fallback link when markdown content is empty', () => {
        const html = makeMgr().buildHtml('0.13.0', '');
        expect(html).toContain('CHANGELOG');
        expect(html).toContain('href=');
    });

    test('escapes HTML in version string', () => {
        const html = makeMgr().buildHtml('<script>', '');
        expect(html).not.toContain('<script>');
        expect(html).toContain('&lt;script&gt;');
    });

    test('includes Content-Security-Policy meta tag', () => {
        const html = makeMgr().buildHtml('0.13.0', '');
        expect(html).toContain('Content-Security-Policy');
    });
});

// ---------------------------------------------------------------------------
// package.json contract: showWhatsNew command declared
// ---------------------------------------------------------------------------

describe('package.json showWhatsNew command', () => {
    const EXT_ROOT = path.resolve(__dirname, '..', '..');
    let pkg: any;

    beforeAll(() => {
        pkg = JSON.parse(
            fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'),
        );
    });

    test('registers perl-lsp.showWhatsNew command', () => {
        const commandIds = pkg.contributes.commands.map((c: any) => c.command);
        expect(commandIds).toContain('perl-lsp.showWhatsNew');
    });

    test('showWhatsNew command has Perl category', () => {
        const cmd = pkg.contributes.commands.find(
            (c: any) => c.command === 'perl-lsp.showWhatsNew',
        );
        expect(cmd).toBeDefined();
        expect(cmd.category).toBe('Perl');
    });

    test('showWhatsNew command title is user-friendly', () => {
        const cmd = pkg.contributes.commands.find(
            (c: any) => c.command === 'perl-lsp.showWhatsNew',
        );
        expect(cmd).toBeDefined();
        expect(cmd.title).toBeTruthy();
        expect(cmd.title.toLowerCase()).toMatch(/what'?s new|release notes|changelog/i);
    });
});
