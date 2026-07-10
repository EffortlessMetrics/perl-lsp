/**
 * Contract tests for the Oxlint configuration.
 *
 * PREP-2 of the TS7 migration (#3662) replaced ESLint + @typescript-eslint
 * with Oxlint (syntactic rules) + oxlint-tsgolint (type-aware rules, backed
 * by typescript-go / the TS7 engine — not the classic `typescript` npm
 * compiler API that blocked ESLint from ever supporting TS7).
 *
 * These tests verify that:
 *   - The Oxlint config file exists and is valid JSON
 *   - It declares the typescript plugin and the six translated rules
 *   - The npm lint script is wired up to oxlint --type-aware
 *   - oxlint / oxlint-tsgolint devDependencies are present
 *   - No ESLint / @typescript-eslint dependency remains (PREP-2 removed the
 *     TS6-compiler-API lint consumer entirely)
 */

import * as fs from 'fs';
import * as path from 'path';

const EXT_ROOT = path.resolve(__dirname, '..', '..');

describe('Oxlint configuration', () => {
  test('.oxlintrc.json exists at extension root', () => {
    const configPath = path.join(EXT_ROOT, '.oxlintrc.json');
    expect(fs.existsSync(configPath)).toBe(true);
  });

  test('.oxlintrc.json is valid JSON and declares the typescript plugin', () => {
    const configPath = path.join(EXT_ROOT, '.oxlintrc.json');
    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    expect(config.plugins).toContain('typescript');
  });

  test('.oxlintrc.json declares exactly the six rules translated from the former ESLint config (no more, no less)', () => {
    const configPath = path.join(EXT_ROOT, '.oxlintrc.json');
    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    const ruleNames = Object.keys(config.rules).sort();
    const expectedRuleNames = [
      'typescript/no-explicit-any',
      'typescript/consistent-type-imports',
      'typescript/no-floating-promises',
      'no-unused-vars',
      'no-console',
      'eqeqeq',
    ].sort();
    // Exact-set equality (not arrayContaining): a silently added or removed
    // rule — config drift, or a future default the schema starts requiring
    // — must fail this test, not just a missing one.
    expect(ruleNames).toEqual(expectedRuleNames);
  });

  test('package.json lint script runs oxlint in type-aware mode', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    expect(pkg.scripts).toHaveProperty('lint');
    expect(pkg.scripts.lint).toContain('oxlint');
    expect(pkg.scripts.lint).toContain('--type-aware');
  });

  test('package.json has oxlint + oxlint-tsgolint devDependencies', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    expect(pkg.devDependencies).toHaveProperty('oxlint');
    expect(pkg.devDependencies).toHaveProperty('oxlint-tsgolint');
  });

  test('package.json has no ESLint / @typescript-eslint dependency (PREP-2 removed them)', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    const allDeps = { ...pkg.devDependencies, ...pkg.dependencies };
    expect(allDeps).not.toHaveProperty('eslint');
    expect(allDeps).not.toHaveProperty('@typescript-eslint/eslint-plugin');
    expect(allDeps).not.toHaveProperty('@typescript-eslint/parser');
  });

  test('eslint.config.js no longer exists at extension root', () => {
    const configPath = path.join(EXT_ROOT, 'eslint.config.js');
    expect(fs.existsSync(configPath)).toBe(false);
  });
});
