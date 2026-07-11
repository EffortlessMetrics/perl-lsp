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
