/**
 * Contract tests for the ESLint configuration.
 *
 * These tests verify that:
 *   - The ESLint flat config file exists
 *   - It is valid JavaScript (parseable)
 *   - It exports an array (flat config format)
 *   - The npm lint script is wired up in package.json
 *   - ESLint devDependencies are present in package.json
 */

import * as fs from 'fs';
import * as path from 'path';

const EXT_ROOT = path.resolve(__dirname, '..', '..');

describe('ESLint configuration', () => {
  test('eslint.config.js exists at extension root', () => {
    const configPath = path.join(EXT_ROOT, 'eslint.config.js');
    expect(fs.existsSync(configPath)).toBe(true);
  });

  test('eslint.config.js is valid JavaScript (require-able)', () => {
    const configPath = path.join(EXT_ROOT, 'eslint.config.js');
    // Should not throw when required
    expect(() => require(configPath)).not.toThrow();
  });

  test('eslint.config.js exports an array (flat config format)', () => {
    const configPath = path.join(EXT_ROOT, 'eslint.config.js');
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const config = require(configPath);
    expect(Array.isArray(config)).toBe(true);
    expect(config.length).toBeGreaterThan(0);
  });

  test('package.json has lint script', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    expect(pkg.scripts).toHaveProperty('lint');
    expect(pkg.scripts.lint).toContain('eslint');
  });

  test('package.json has eslint devDependencies', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    const allDeps = { ...pkg.devDependencies, ...pkg.dependencies };
    expect(allDeps).toHaveProperty('eslint');
    expect(allDeps).toHaveProperty('@typescript-eslint/eslint-plugin');
    expect(allDeps).toHaveProperty('@typescript-eslint/parser');
  });
});
