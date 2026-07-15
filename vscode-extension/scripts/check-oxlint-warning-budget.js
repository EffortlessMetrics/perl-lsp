#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..');
const BASELINE_PATH = path.join(ROOT, 'docs', 'migrations', 'oxlint-warning-baseline.json');
const OXLINT_ENTRY = path.join(ROOT, 'node_modules', 'oxlint', 'bin', 'oxlint');
const OXLINT_PATHS = ['src', 'src/test', 'scripts', 'jest.config.js', 'rolldown.config.mjs'];

/** @typedef {Record<string, number>} CountMap */
/** @typedef {{severity?: string, code?: string, filename?: string, message?: string, labels?: Array<{span?: {line?: number, column?: number}}>}} OxlintDiagnostic */
/** @typedef {{warning_count: number, by_rule: CountMap, by_surface: CountMap, by_rule_and_surface: Record<string, CountMap>, by_file: CountMap}} Inventory */

/**
 * @param {string} filename
 * @returns {string}
 */
function normalizeFilename(filename) {
  return path.relative(ROOT, path.resolve(ROOT, filename)).replaceAll('\\', '/');
}

/**
 * @param {string} filename
 * @returns {string}
 */
function surfaceForFile(filename) {
  const normalized = normalizeFilename(filename);
  if (normalized.startsWith('src/test/')) return 'tests';
  if (normalized.startsWith('src/')) return 'production';
  if (normalized.startsWith('scripts/')) return 'scripts';
  return 'build-config';
}

/** @param {CountMap} counts @param {string} key */
function increment(counts, key) {
  counts[key] = (counts[key] ?? 0) + 1;
}

/** @param {CountMap} counts @returns {CountMap} */
function sortedCounts(counts) {
  return Object.fromEntries(
    Object.entries(counts).sort(([left], [right]) => left.localeCompare(right)),
  );
}

/**
 * @param {OxlintDiagnostic[]} diagnostics
 * @returns {Inventory}
 */
function buildInventory(diagnostics) {
  /** @type {CountMap} */
  const byRule = {};
  /** @type {CountMap} */
  const bySurface = {};
  /** @type {Record<string, CountMap>} */
  const byRuleAndSurface = {};
  /** @type {CountMap} */
  const byFile = {};
  let warningCount = 0;

  for (const diagnostic of diagnostics) {
    if (diagnostic.severity !== 'warning') continue;
    warningCount += 1;
    const rule = diagnostic.code || 'unknown';
    const filename = normalizeFilename(diagnostic.filename || 'unknown');
    const surface = surfaceForFile(filename);
    increment(byRule, rule);
    increment(bySurface, surface);
    byRuleAndSurface[rule] ??= {};
    increment(byRuleAndSurface[rule], surface);
    increment(byFile, filename);
  }

  return {
    warning_count: warningCount,
    by_rule: sortedCounts(byRule),
    by_surface: sortedCounts(bySurface),
    by_rule_and_surface: Object.fromEntries(
      Object.entries(byRuleAndSurface)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([rule, counts]) => [rule, sortedCounts(counts)]),
    ),
    by_file: sortedCounts(byFile),
  };
}

function runOxlint() {
  return spawnSync(
    process.execPath,
    [OXLINT_ENTRY, ...OXLINT_PATHS, '--type-aware', '--format', 'json'],
    { cwd: ROOT, encoding: 'utf8', windowsHide: true, maxBuffer: 10 * 1024 * 1024 },
  );
}

/** @param {string} output @returns {OxlintDiagnostic[]} */
function readDiagnostics(output) {
  let parsed;
  try {
    parsed = JSON.parse(output);
  } catch (error) {
    throw new Error(
      `Oxlint did not return JSON diagnostics: ${error instanceof Error ? error.message : String(error)}\n${output.slice(0, 2000)}`,
    );
  }
  if (Array.isArray(parsed)) return parsed;
  if (Array.isArray(parsed.diagnostics)) return parsed.diagnostics;
  throw new Error('Oxlint JSON output did not contain a diagnostics array');
}

/** @param {CountMap} current @param {CountMap} baseline @param {string} label */
function countExceeds(current, baseline, label) {
  const failures = [];
  const keys = new Set([...Object.keys(current), ...Object.keys(baseline)]);
  for (const key of [...keys].sort()) {
    const currentCount = current[key] ?? 0;
    const baselineCount = baseline[key] ?? 0;
    if (currentCount > baselineCount)
      failures.push(`${label} ${key}: ${currentCount} > ${baselineCount}`);
  }
  return failures;
}

/**
 * @param {Record<string, CountMap>} current
 * @param {Record<string, CountMap>} baseline
 * @param {string} label
 */
function nestedCountsExceed(current, baseline, label) {
  const failures = [];
  const keys = new Set([...Object.keys(current ?? {}), ...Object.keys(baseline ?? {})]);
  for (const key of [...keys].sort()) {
    failures.push(...countExceeds(current?.[key] ?? {}, baseline?.[key] ?? {}, `${label} ${key}`));
  }
  return failures;
}

/** @param {Inventory} current @param {Inventory} baseline @returns {string[]} */
function compareInventory(current, baseline) {
  return [
    ...(current.warning_count > baseline.warning_count
      ? [`total warnings: ${current.warning_count} > ${baseline.warning_count}`]
      : []),
    ...countExceeds(current.by_rule, baseline.by_rule, 'rule'),
    ...countExceeds(current.by_surface, baseline.by_surface, 'surface'),
    ...nestedCountsExceed(
      current.by_rule_and_surface,
      baseline.by_rule_and_surface,
      'rule×surface',
    ),
    ...countExceeds(current.by_file, baseline.by_file, 'file'),
  ];
}

/** @param {OxlintDiagnostic[]} errors @param {number | null} status @returns {number} */
function failureExitCode(errors, status) {
  if (errors.length > 0) return status && status !== 0 ? status : 1;
  return status ?? 1;
}

/** @param {OxlintDiagnostic} diagnostic @returns {string} */
function formatDiagnostic(diagnostic) {
  const span = diagnostic.labels?.[0]?.span;
  const location = span
    ? `${diagnostic.filename}:${span.line}:${span.column}`
    : diagnostic.filename || '<unknown>';
  return `${location}: ${diagnostic.code || 'oxlint'}: ${diagnostic.message || ''}`;
}

/** @param {Inventory} inventory @param {Inventory} baseline */
function printSummary(inventory, baseline) {
  process.stdout.write(
    `Oxlint warning inventory: ${inventory.warning_count}/${baseline.warning_count}\n`,
  );
  process.stdout.write(
    `  surfaces: ${Object.entries(inventory.by_surface)
      .map(([surface, count]) => `${surface}=${count}`)
      .join(', ')}\n`,
  );
  for (const [rule, count] of Object.entries(inventory.by_rule)) {
    process.stdout.write(`  ${rule}: ${count}\n`);
  }
}

/** @returns {number} */
function main() {
  let baseline;
  try {
    baseline = JSON.parse(fs.readFileSync(BASELINE_PATH, 'utf8'));
  } catch (error) {
    process.stderr.write(
      `Unable to read Oxlint baseline: ${error instanceof Error ? error.message : String(error)}\n`,
    );
    return 1;
  }

  const result = runOxlint();
  if (result.error) {
    process.stderr.write(`Unable to run Oxlint: ${result.error.message}\n`);
    return 1;
  }

  let diagnostics;
  try {
    diagnostics = readDiagnostics(result.stdout ?? '');
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    if (result.stderr) process.stderr.write(`Oxlint stderr:\n${result.stderr}\n`);
    return 1;
  }

  const errors = diagnostics.filter((diagnostic) => diagnostic.severity === 'error');
  if (errors.length > 0 || result.status !== 0) {
    if (errors.length > 0) {
      for (const diagnostic of errors.slice(0, 20))
        process.stderr.write(`${formatDiagnostic(diagnostic)}\n`);
      if (errors.length > 20)
        process.stderr.write(`... ${errors.length - 20} more Oxlint errors\n`);
    } else if (result.stderr) {
      process.stderr.write(`Oxlint stderr:\n${result.stderr}\n`);
    }
    return failureExitCode(errors, result.status);
  }

  const inventory = buildInventory(diagnostics);
  printSummary(inventory, baseline);
  const failures = compareInventory(inventory, baseline);
  if (failures.length > 0) {
    process.stderr.write('Oxlint warning ratchet exceeded:\n');
    for (const failure of failures) process.stderr.write(`  ${failure}\n`);
    return 1;
  }
  return 0;
}

if (require.main === module) process.exitCode = main();

module.exports = { buildInventory, compareInventory, failureExitCode, surfaceForFile };
