#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const EXTENSION_ROOT = path.resolve(__dirname, '..');
const BASELINE_PATH = path.join(__dirname, 'vsix-inventory-baseline.json');
const VSCE_ENTRY = path.join(EXTENSION_ROOT, 'node_modules', '@vscode', 'vsce', 'vsce');

function collectPackagedFiles() {
  const result = spawnSync(process.execPath, [VSCE_ENTRY, 'ls'], {
    cwd: EXTENSION_ROOT,
    encoding: 'utf8',
    windowsHide: true,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`vsce ls failed: ${(result.stderr || result.stdout || '').trim()}`);
  }
  return result.stdout
    .split(/\r?\n/)
    .map((line) => line.trim().replaceAll('\\', '/'))
    .filter((file) => file.length > 0)
    .map((file) => ({ file, bytes: fs.statSync(path.join(EXTENSION_ROOT, file)).size }));
}

function summarizeInventory(entries) {
  const files = Object.fromEntries(entries.map(({ file, bytes }) => [file, bytes]));
  return {
    schema_version: 1,
    total_files: entries.length,
    total_bytes: entries.reduce((total, { bytes }) => total + bytes, 0),
    files,
  };
}

function compareInventory(actual, baseline) {
  const violations = [];
  if (actual.total_files > baseline.total_files) {
    violations.push(`file count grew from ${baseline.total_files} to ${actual.total_files}`);
  }
  if (actual.total_bytes > baseline.total_bytes) {
    violations.push(`total bytes grew from ${baseline.total_bytes} to ${actual.total_bytes}`);
  }
  for (const [file, bytes] of Object.entries(actual.files)) {
    if (!Object.hasOwn(baseline.files, file)) {
      violations.push(`new packaged file: ${file}`);
    } else if (bytes > baseline.files[file]) {
      violations.push(`file ${file} grew from ${baseline.files[file]} to ${bytes} bytes`);
    }
  }
  for (const file of Object.keys(baseline.files)) {
    if (!Object.hasOwn(actual.files, file)) {
      violations.push(`baseline packaged file is missing: ${file}`);
    }
  }
  return violations;
}

function main() {
  const updateBaseline = process.argv.includes('--update-baseline');
  const baseline =
    updateBaseline && !fs.existsSync(BASELINE_PATH)
      ? null
      : JSON.parse(fs.readFileSync(BASELINE_PATH, 'utf8'));
  const actual = summarizeInventory(collectPackagedFiles());
  if (updateBaseline) {
    fs.writeFileSync(BASELINE_PATH, `${JSON.stringify(actual, null, 2)}\n`);
    process.stdout.write(`Updated ${BASELINE_PATH}\n`);
    return;
  }
  const violations = compareInventory(actual, baseline);
  process.stdout.write(
    `${JSON.stringify({ ...actual, baseline: BASELINE_PATH, violations }, null, 2)}\n`,
  );
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

module.exports = { compareInventory, summarizeInventory };
