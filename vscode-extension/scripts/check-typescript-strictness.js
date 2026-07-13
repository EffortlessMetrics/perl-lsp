#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const EXTENSION_ROOT = path.resolve(__dirname, '..');
const TYPESCRIPT_CLI = path.join(EXTENSION_ROOT, 'node_modules', 'typescript', 'bin', 'tsc');
const BASELINE_PATH = path.join(EXTENSION_ROOT, 'scripts', 'typescript-strictness-baseline.json');
const CONFIGS = [
  'tsconfig.json',
  'tsconfig.test.json',
  'tsconfig.integration.json',
  'tsconfig.published-smoke.json',
  'tsconfig.scripts.json',
];

function parseDiagnostics(output, config) {
  const diagnostics = [];
  const pattern = /^(.*)\((\d+),(\d+)\): error (TS\d+): (.*)$/gm;
  for (const match of output.matchAll(pattern)) {
    diagnostics.push({
      config,
      file: match[1].replaceAll('\\', '/'),
      line: Number(match[2]),
      column: Number(match[3]),
      code: match[4],
      message: match[5],
    });
  }
  return diagnostics;
}

function increment(bucket, key) {
  bucket[key] = (bucket[key] ?? 0) + 1;
}

function summarizeDiagnostics(diagnostics) {
  const byConfig = {};
  const byCode = {};
  const byFile = {};
  for (const diagnostic of diagnostics) {
    increment(byConfig, diagnostic.config);
    increment(byCode, diagnostic.code);
    increment(byFile, `${diagnostic.config}:${diagnostic.file}`);
  }
  return {
    total: diagnostics.length,
    by_config: byConfig,
    by_code: byCode,
    by_file: byFile,
    diagnostics,
  };
}

function compareBuckets(actual, baseline, label) {
  const violations = [];
  for (const [key, count] of Object.entries(actual)) {
    const baselineCount = baseline[key] ?? 0;
    if (count > baselineCount) {
      violations.push(`${label} ${key} grew from ${baselineCount} to ${count}`);
    }
  }
  return violations;
}

function compareToBaseline(actual, baseline) {
  const violations = [];
  if (actual.total > baseline.total) {
    violations.push(`total grew from ${baseline.total} to ${actual.total}`);
  }
  violations.push(...compareBuckets(actual.by_config, baseline.by_config, 'config'));
  violations.push(...compareBuckets(actual.by_code, baseline.by_code, 'code'));
  violations.push(...compareBuckets(actual.by_file, baseline.by_file, 'file'));
  return violations;
}

function validateCompilerResult(result, diagnostics, config) {
  if (result.status !== 0 && diagnostics.length === 0) {
    const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
    throw new Error(
      `TypeScript failed for ${config} without parsed diagnostics (status ${result.status ?? 'unknown'}): ${output.trim() || 'no output'}`,
    );
  }
}

function runConfig(config) {
  const result = spawnSync(
    process.execPath,
    [
      TYPESCRIPT_CLI,
      '--noEmit',
      '--project',
      config,
      '--noUncheckedIndexedAccess',
      'true',
      '--pretty',
      'false',
    ],
    { cwd: EXTENSION_ROOT, encoding: 'utf8', windowsHide: true },
  );
  if (result.error) {
    throw result.error;
  }
  const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
  const diagnostics = parseDiagnostics(output, config);
  validateCompilerResult(result, diagnostics, config);
  return diagnostics;
}

function main() {
  const baseline = JSON.parse(fs.readFileSync(BASELINE_PATH, 'utf8'));
  const diagnostics = CONFIGS.flatMap(runConfig);
  const actual = summarizeDiagnostics(diagnostics);
  if (process.argv.includes('--update-baseline')) {
    fs.writeFileSync(
      BASELINE_PATH,
      `${JSON.stringify({ schema_version: 1, option: 'noUncheckedIndexedAccess', ...actual }, null, 2)}\n`,
    );
    process.stdout.write(`Updated ${BASELINE_PATH}\n`);
    return;
  }
  const violations = compareToBaseline(actual, baseline);
  const { diagnostics: detailedDiagnostics, ...summary } = actual;
  const report = {
    schema_version: 1,
    option: 'noUncheckedIndexedAccess',
    baseline: BASELINE_PATH,
    ...summary,
    ...(process.argv.includes('--verbose') ? { diagnostics: detailedDiagnostics } : {}),
    status: violations.length === 0 ? 'advisory_within_baseline' : 'growth_detected',
    violations,
  };
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (violations.length > 0) {
    process.exitCode = 1;
  }
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}

module.exports = {
  compareToBaseline,
  parseDiagnostics,
  summarizeDiagnostics,
  validateCompilerResult,
};
