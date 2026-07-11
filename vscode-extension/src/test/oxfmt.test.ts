/**
 * Contract tests for the Oxfmt configuration.
 *
 * PREP-3 of the TS7 migration (#3662) adopted Oxfmt as the sole formatter
 * for vscode-extension/** — no Prettier, no other formatter. These tests
 * verify that:
 *   - The Oxfmt config file exists and is valid JSON
 *   - `sortPackageJson` is explicitly disabled (deliberate: Oxfmt's default
 *     package.json key/array reformatting is a large, unrelated-looking
 *     diff on this repo's curated key order — see
 *     docs/migrations/ts7-prep-3-oxfmt-receipts.md)
 *   - `singleQuote` is explicitly enabled (matches the extension's existing
 *     quote convention; Oxfmt's own default is double quotes)
 *   - Generated/vendored/fixture surfaces are excluded via `ignorePatterns`
 *   - The npm fmt / fmt:check scripts are wired up to oxfmt
 *   - oxfmt is present in devDependencies, exactly pinned (no ^/~)
 *   - No Prettier dependency was introduced
 */

import { execFileSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

const EXT_ROOT = path.resolve(__dirname, '..', '..');

describe('Oxfmt configuration', () => {
  test('.oxfmtrc.json exists at extension root', () => {
    const configPath = path.join(EXT_ROOT, '.oxfmtrc.json');
    expect(fs.existsSync(configPath)).toBe(true);
  });

  test('.oxfmtrc.json is valid JSON', () => {
    const configPath = path.join(EXT_ROOT, '.oxfmtrc.json');
    expect(() => JSON.parse(fs.readFileSync(configPath, 'utf8'))).not.toThrow();
  });

  test('.oxfmtrc.json disables sortPackageJson (deliberate — avoids reordering package.json)', () => {
    const configPath = path.join(EXT_ROOT, '.oxfmtrc.json');
    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    expect(config.sortPackageJson).toBe(false);
  });

  test('.oxfmtrc.json enables singleQuote (matches the existing source convention)', () => {
    const configPath = path.join(EXT_ROOT, '.oxfmtrc.json');
    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    expect(config.singleQuote).toBe(true);
  });

  test('.oxfmtrc.json excludes generated/vendored/fixture surfaces from formatting', () => {
    const configPath = path.join(EXT_ROOT, '.oxfmtrc.json');
    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    expect(config.ignorePatterns).toEqual(
      expect.arrayContaining(['syntaxes/**', 'test/grammar/fixtures/**', 'package-lock.json']),
    );
  });

  test('package.json fmt scripts run oxfmt (write and check)', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    expect(pkg.scripts).toHaveProperty('fmt');
    expect(pkg.scripts.fmt).toContain('oxfmt');
    expect(pkg.scripts.fmt).toContain('--write');
    expect(pkg.scripts).toHaveProperty('fmt:check');
    expect(pkg.scripts['fmt:check']).toContain('oxfmt');
    expect(pkg.scripts['fmt:check']).toContain('--check');
  });

  test('package.json has oxfmt devDependency, exactly pinned (no ^/~)', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    expect(pkg.devDependencies).toHaveProperty('oxfmt');
    expect(pkg.devDependencies.oxfmt).toMatch(/^\d+\.\d+\.\d+$/);
  });

  test('package.json has no Prettier dependency (Oxfmt is the sole formatter)', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    const allDeps = { ...pkg.devDependencies, ...pkg.dependencies };
    expect(allDeps).not.toHaveProperty('prettier');
    for (const name of Object.keys(allDeps)) {
      expect(name.toLowerCase()).not.toContain('prettier');
    }
  });
});

describe('VSIX packaging does not ship dev-tooling native binaries', () => {
  // `vsce ls` is the packaging tool's own dry-run manifest of every file that
  // would be included in the published .vsix. Oxfmt (like Oxlint) ships
  // native platform bindings via optionalDependencies under devDependencies
  // — vsce is expected to exclude devDependencies-only packages from the
  // package, but that exclusion is vsce's dependency-analysis behavior, not
  // this repo's own code. A future change that moves oxfmt/oxlint to
  // `dependencies` by mistake, or an .vscodeignore edit that accidentally
  // widens node_modules inclusion, would silently bundle a multi-megabyte
  // native binary into the published extension with no other signal. This
  // test asserts against the real packaging manifest, not just config
  // intent, so that regression is caught here instead of in a published
  // release.
  test('vsce ls manifest contains no oxfmt/oxlint/prettier node_modules entries', () => {
    // Invoke vsce's own JS entry point directly via `node`, not `npx` (which
    // needs a shell on Windows to resolve — shell:true string-concatenates
    // args instead of escaping them, a real risk once paths contain spaces).
    const vsceEntry = path.join(EXT_ROOT, 'node_modules', '@vscode', 'vsce', 'vsce');
    const output = execFileSync(process.execPath, [vsceEntry, 'ls'], {
      cwd: EXT_ROOT,
      encoding: 'utf8',
    });
    const files = output.split(/\r?\n/).filter((line) => line.trim().length > 0);

    expect(files.length).toBeGreaterThan(0);

    const devToolingLeaks = files.filter((f) =>
      /node_modules[\\/](@)?(oxfmt|oxlint|prettier)/i.test(f),
    );
    expect(devToolingLeaks).toEqual([]);

    // The two tiny rc config files are expected and harmless (not code, not
    // a binary) — assert their presence so this test also documents that
    // deliberate, known inclusion rather than silently tolerating it.
    expect(files).toEqual(expect.arrayContaining(['.oxfmtrc.json', '.oxlintrc.json']));
  }, 30000);
});
