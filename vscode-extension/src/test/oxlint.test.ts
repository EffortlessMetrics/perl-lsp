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
 *   - The npm lint script is wired up to the type-aware warning ratchet
 *   - The committed warning inventory is internally consistent
 *   - oxlint / oxlint-tsgolint devDependencies are present
 *   - No ESLint / @typescript-eslint dependency remains (PREP-2 removed the
 *     TS6-compiler-API lint consumer entirely)
 */

import * as fs from 'fs';
import * as path from 'path';
import {
  buildInventory,
  compareInventory,
  failureExitCode,
  surfaceForFile,
} from '../../scripts/check-oxlint-warning-budget';

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

  test('package.json lint script runs the type-aware warning ratchet', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    expect(pkg.scripts).toHaveProperty('lint');
    expect(pkg.scripts.lint).toContain('check-oxlint-warning-budget');
    const checker = fs.readFileSync(
      path.join(EXT_ROOT, 'scripts', 'check-oxlint-warning-budget.js'),
      'utf8',
    );
    expect(checker).toContain("'--type-aware'");
    expect(checker).toContain("'--format', 'json'");
  });

  test('warning baseline is machine-readable and internally consistent', () => {
    const baseline = JSON.parse(
      fs.readFileSync(
        path.join(EXT_ROOT, 'docs', 'migrations', 'oxlint-warning-baseline.json'),
        'utf8',
      ),
    );
    expect(baseline.schema_version).toBe(1);
    expect(baseline.warning_count).toBe(
      (Object.values(baseline.by_rule) as number[]).reduce((total, count) => total + count, 0),
    );
    expect(baseline.warning_count).toBe(
      (Object.values(baseline.by_surface) as number[]).reduce((total, count) => total + count, 0),
    );
    expect(baseline.warning_count).toBe(
      (Object.values(baseline.by_file) as number[]).reduce((total, count) => total + count, 0),
    );
    expect(baseline.warning_count).toBe(
      (Object.values(baseline.by_rule_and_surface) as Record<string, number>[])
        .flatMap((counts) => Object.values(counts))
        .reduce((total, count) => total + count, 0),
    );
    expect(baseline.scope).toEqual(
      expect.arrayContaining(['src/test', 'scripts', 'jest.config.js', 'rolldown.config.mjs']),
    );
  });

  test('warning inventory classifies rules by owned surface', () => {
    const inventory = buildInventory([
      { severity: 'warning', code: 'no-console', filename: 'scripts/build.js' },
      { severity: 'warning', code: 'typescript/no-explicit-any', filename: 'src/main.ts' },
      {
        severity: 'warning',
        code: 'typescript/no-explicit-any',
        filename: 'src/test/main.test.ts',
      },
      { severity: 'error', code: 'eqeqeq', filename: 'src/main.ts' },
    ]);

    expect(inventory.warning_count).toBe(3);
    expect(inventory.by_surface).toEqual({ production: 1, scripts: 1, tests: 1 });
    expect(inventory.by_rule_and_surface['typescript/no-explicit-any']).toEqual({
      production: 1,
      tests: 1,
    });
    expect(surfaceForFile('rolldown.config.mjs')).toBe('build-config');
  });

  test('warning ratchet rejects a new rule or surface bucket', () => {
    const baseline = {
      warning_count: 2,
      by_rule: { 'no-console': 2 },
      by_surface: { scripts: 2 },
      by_rule_and_surface: {},
      by_file: {},
    };
    const current = {
      warning_count: 2,
      by_rule: { 'no-console': 1, 'no-unused-vars': 1 },
      by_surface: { scripts: 1, tests: 1 },
      by_rule_and_surface: {},
      by_file: {},
    };

    expect(compareInventory(current, baseline)).toEqual([
      'rule no-unused-vars: 1 > 0',
      'surface tests: 1 > 0',
    ]);
    expect(compareInventory({ ...current, warning_count: 3 }, baseline)).toContain(
      'total warnings: 3 > 2',
    );
  });

  test('warning ratchet rejects growth in rule-by-surface and file buckets', () => {
    const baseline = {
      warning_count: 2,
      by_rule: { 'no-console': 2 },
      by_surface: { scripts: 2 },
      by_rule_and_surface: { 'no-console': { scripts: 2 } },
      by_file: { 'scripts/build.js': 2 },
    };
    const current = {
      warning_count: 2,
      by_rule: { 'no-console': 2 },
      by_surface: { scripts: 2 },
      by_rule_and_surface: { 'no-console': { scripts: 1, tests: 1 } },
      by_file: { 'scripts/build.js': 1, 'scripts/report.js': 1 },
    };

    expect(compareInventory(current, baseline)).toEqual([
      'rule×surface no-console tests: 1 > 0',
      'file scripts/report.js: 1 > 0',
    ]);
  });

  test('Oxlint errors always produce a failing process status', () => {
    expect(failureExitCode([{ severity: 'error' }], 0)).toBe(1);
    expect(failureExitCode([{ severity: 'error' }], 2)).toBe(2);
    expect(failureExitCode([], 2)).toBe(2);
    expect(failureExitCode([], null)).toBe(1);
  });

  test('canonical config covers tests and JavaScript build scripts', () => {
    const config = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, '.oxlintrc.json'), 'utf8'));
    expect(config.ignorePatterns).not.toContain('src/test/**');
    expect(config.ignorePatterns).not.toContain('*.js');
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
