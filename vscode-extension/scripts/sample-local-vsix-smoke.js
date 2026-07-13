#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const root = path.resolve(__dirname, '..');

function parseRunCount(args = process.argv, env = process.env) {
  const runsIndex = args.indexOf('--runs');
  const value = runsIndex === -1 ? env.PERL_LSP_VSCODE_SAMPLE_RUNS || '3' : args[runsIndex + 1];
  const runs = Number(value);
  if (!Number.isSafeInteger(runs) || runs < 1) {
    throw new Error(
      `Sample count must be a positive integer, received: ${value ?? 'missing value'}`,
    );
  }
  return runs;
}

function sampleDirectory(rootDirectory, index) {
  return path.join(rootDirectory, `sample-${String(index).padStart(2, '0')}`);
}

function hasCompletedReceipt(rootDirectory) {
  const pending = [rootDirectory];
  while (pending.length > 0) {
    const current = pending.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        pending.push(fullPath);
      } else if (entry.name === 'first_hour_vscode_receipt.json') {
        const receipt = JSON.parse(fs.readFileSync(fullPath, 'utf8'));
        if (receipt.outcome === 'completed') {
          return true;
        }
      }
    }
  }
  return false;
}

function runSample(index, rootDirectory, env) {
  const receiptDirectory = sampleDirectory(rootDirectory, index);
  fs.mkdirSync(receiptDirectory, { recursive: true });
  const result = spawnSync(
    process.execPath,
    [path.join(root, 'scripts', 'run-local-vsix-smoke.js')],
    {
      cwd: root,
      env: {
        ...env,
        PERL_LSP_SMOKE_RECEIPTS_DIR: receiptDirectory,
        PERL_LSP_SMOKE_SOURCE_LABEL: `local-current-source-sample-${index}`,
      },
      stdio: 'inherit',
      windowsHide: true,
    },
  );
  const status = result.status ?? 1;
  if (status !== 0 && hasCompletedReceipt(receiptDirectory)) {
    process.stderr.write(
      `Smoke process exited ${status} after writing a completed receipt for sample ${index}; treating the known cleanup warning as non-fatal.\n`,
    );
    return 0;
  }
  return status;
}

function main() {
  const runs = parseRunCount();
  const configuredRoot = process.env.PERL_LSP_VSCODE_SAMPLES_DIR;
  const rootDirectory = configuredRoot
    ? path.resolve(configuredRoot)
    : path.join(root, 'target', 'receipts', 'vscode-samples', `run-${Date.now()}`);
  fs.mkdirSync(rootDirectory, { recursive: true });

  let failures = 0;
  let successfulSamples = 0;
  for (let index = 1; index <= runs; index += 1) {
    process.stdout.write(`\n=== VS Code current-source sample ${index}/${runs} ===\n`);
    if (runSample(index, rootDirectory, process.env) === 0) {
      successfulSamples += 1;
    } else {
      failures += 1;
    }
  }

  const summaryPath =
    process.env.PERL_LSP_VSCODE_SAMPLE_SUMMARY || path.join(rootDirectory, 'summary.json');
  const summaryResult = spawnSync(
    process.execPath,
    [path.join(root, 'scripts', 'summarize-vscode-receipts.js'), rootDirectory, summaryPath],
    { cwd: root, env: process.env, stdio: 'inherit', windowsHide: true },
  );
  if (summaryResult.status !== 0) {
    failures += 1;
  }

  process.stdout.write(
    `\nCollected ${successfulSamples} successful sample run(s) under ${rootDirectory}.\n`,
  );
  if (failures > 0) {
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

module.exports = { parseRunCount, sampleDirectory };
