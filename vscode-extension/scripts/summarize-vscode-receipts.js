#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

/** @typedef {{count:number,min:number|null,max:number|null,p50:number|null,p95:number|null}} MetricSummary */

/** @type {Record<string, (receipt: any) => number|undefined>} */
const METRIC_READERS = {
  activation_ms: (receipt) => receipt.startup?.extension_activation_ms,
  extension_load_ms: (receipt) => receipt.startup?.language_client?.milestones?.extension_load,
  activate_entered_ms: (receipt) => receipt.startup?.language_client?.milestones?.activate_entered,
  commands_registered_ms: (receipt) =>
    receipt.startup?.language_client?.milestones?.commands_registered,
  activate_returned_ms: (receipt) =>
    receipt.startup?.language_client?.milestones?.activate_returned,
  binary_resolution_started_ms: (receipt) =>
    receipt.startup?.language_client?.milestones?.binary_resolution_started,
  binary_resolution_completed_ms: (receipt) =>
    receipt.startup?.language_client?.milestones?.binary_resolution_completed,
  process_started_ms: (receipt) => receipt.startup?.language_client?.milestones?.process_started,
  initialize_completed_ms: (receipt) =>
    receipt.startup?.language_client?.milestones?.initialize_completed,
  workspace_ready_ms: (receipt) => receipt.startup?.language_client?.milestones?.workspace_ready,
  first_useful_request_ms: (receipt) =>
    receipt.startup?.language_client?.milestones?.first_useful_request,
  warm_request_ms: (receipt) => receipt.startup?.language_client?.milestones?.warm_request,
  restart_milestone_ms: (receipt) =>
    receipt.lifecycle?.restart?.language_client?.milestones?.restart ??
    receipt.startup?.language_client?.milestones?.restart,
  shutdown_milestone_ms: (receipt) =>
    receipt.lifecycle?.shutdown?.language_client?.milestones?.shutdown ??
    receipt.startup?.language_client?.milestones?.shutdown,
  binary_resolution_ms: (receipt) => receipt.startup?.language_client?.binary_resolution_ms,
  server_start_ms: (receipt) => receipt.startup?.language_client?.server_start_ms,
  initialize_ms: (receipt) => receipt.startup?.language_client?.initialize_ms,
  immediate_completion_ms: (receipt) => receipt.moments?.immediate?.completion?.duration_ms,
  warm_completion_ms: (receipt) =>
    receipt.moments?.after_30_seconds?.classification === 'warm'
      ? receipt.moments.after_30_seconds.completion?.duration_ms
      : undefined,
  restart_ms: (receipt) => receipt.lifecycle?.restart?.duration_ms,
  shutdown_ms: (receipt) => receipt.lifecycle?.shutdown?.duration_ms,
};

function percentile(values, fraction) {
  if (values.length === 0) {
    return null;
  }
  const sorted = [...values].sort((left, right) => left - right);
  const position = (sorted.length - 1) * fraction;
  const lower = Math.floor(position);
  const upper = Math.ceil(position);
  if (lower === upper) {
    return sorted[lower];
  }
  const weight = position - lower;
  return sorted[lower] + (sorted[upper] - sorted[lower]) * weight;
}

function collectReceiptPaths(root) {
  if (!fs.existsSync(root)) {
    return [];
  }
  const results = [];
  const pending = [root];
  while (pending.length > 0) {
    const current = pending.pop();
    if (!current) {
      continue;
    }
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        pending.push(fullPath);
      } else if (entry.name === 'first_hour_vscode_receipt.json') {
        results.push(fullPath);
      }
    }
  }
  return results.sort();
}

function readCompletedReceipts(paths) {
  return paths.flatMap((receiptPath) => {
    const receipt = JSON.parse(fs.readFileSync(receiptPath, 'utf8'));
    return receipt.outcome === 'completed' ? [{ receipt, receiptPath }] : [];
  });
}

/** @param {Array<{receipt:any}>} receipts */
function summarizeReceipts(receipts) {
  const metrics = {};
  for (const [name, readMetric] of Object.entries(METRIC_READERS)) {
    /** @type {number[]} */
    const values = [];
    for (const { receipt } of receipts) {
      const value = readMetric(receipt);
      if (typeof value === 'number' && Number.isFinite(value)) {
        values.push(value);
      }
    }
    metrics[name] = {
      count: values.length,
      min: values.length > 0 ? Math.min(...values) : null,
      max: values.length > 0 ? Math.max(...values) : null,
      p50: percentile(values, 0.5),
      p95: percentile(values, 0.95),
    };
  }

  const binaryResolutionSources = {};
  const runClassifications = {};
  for (const { receipt } of receipts) {
    const source = receipt.startup?.language_client?.binary_resolution_source;
    if (typeof source === 'string') {
      binaryResolutionSources[source] = (binaryResolutionSources[source] ?? 0) + 1;
    }
    const classification = receipt.performance?.run_classification;
    if (typeof classification === 'string') {
      runClassifications[classification] = (runClassifications[classification] ?? 0) + 1;
    }
  }

  return {
    schema_version: 1,
    generated_at: new Date().toISOString(),
    sample_count: receipts.length,
    binary_resolution_sources: binaryResolutionSources,
    run_classifications: runClassifications,
    metrics,
  };
}

function main() {
  const root = path.resolve(
    process.argv[2] ?? path.resolve(__dirname, '..', 'target', 'receipts', 'vscode-smoke'),
  );
  const outputPath = path.resolve(
    process.argv[3] ??
      process.env.PERL_LSP_VSCODE_RECEIPT_SUMMARY ??
      path.join(root, 'summary.json'),
  );
  const receiptPaths = collectReceiptPaths(root);
  const receipts = readCompletedReceipts(receiptPaths);
  if (receipts.length === 0) {
    throw new Error(`No completed first-hour VS Code receipts found under ${root}`);
  }
  const summary = summarizeReceipts(receipts);
  summary.receipts = receipts.map(({ receiptPath }) => path.relative(root, receiptPath));
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(summary, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}

module.exports = { percentile, summarizeReceipts };
