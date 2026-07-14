const assert = require('node:assert/strict');
const { test } = require('node:test');
const { summarizeReceipts } = require('./summarize-vscode-receipts');

function receipt(activation, completion, classification = 'cold_start_with_warm_request') {
  return {
    outcome: 'completed',
    performance: { run_classification: classification },
    startup: {
      extension_activation_ms: activation,
      language_client: {
        binary_resolution_source: 'configured',
        binary_resolution_ms: 2,
        server_start_ms: activation + 5,
        initialize_ms: activation + 10,
        milestones: {
          extension_load: 0,
          activate_entered: 1,
          commands_registered: activation - 2,
          activate_returned: activation,
          first_useful_request: activation + 20,
          warm_request: activation + 100,
        },
      },
    },
    moments: {
      immediate: { classification: 'cold', completion: { duration_ms: completion } },
      after_30_seconds: {
        classification: classification === 'cold_start_with_restart' ? 'post_restart' : 'warm',
        completion: { duration_ms: completion / 2 },
      },
    },
    lifecycle: { restart: { duration_ms: 20 }, shutdown: { duration_ms: 3 } },
  };
}

void test('summarizes phase metrics with deterministic percentiles', () => {
  const summary = summarizeReceipts([
    { receipt: receipt(10, 100) },
    { receipt: receipt(20, 200) },
    { receipt: receipt(30, 300) },
  ]);

  assert.equal(summary.sample_count, 3);
  assert.deepEqual(summary.binary_resolution_sources, { configured: 3 });
  assert.deepEqual(summary.run_classifications, { cold_start_with_warm_request: 3 });
  assert.deepEqual(summary.metrics.activation_ms, {
    count: 3,
    min: 10,
    max: 30,
    p50: 20,
    p95: 29,
  });
  assert.equal(summary.metrics.warm_completion_ms.p50, 100);
  assert.equal(summary.metrics.warm_completion_ms.p95, 145);
  assert.equal(summary.metrics.first_useful_request_ms.p50, 40);
});

void test('does not count post-restart probes as warm samples', () => {
  const summary = summarizeReceipts([{ receipt: receipt(10, 100, 'cold_start_with_restart') }]);
  assert.equal(summary.metrics.warm_completion_ms.count, 0);
});

void test('omits unavailable phase values without inventing samples', () => {
  const summary = summarizeReceipts([{ receipt: { outcome: 'completed' } }]);
  assert.deepEqual(summary.metrics.initialize_ms, {
    count: 0,
    min: null,
    max: null,
    p50: null,
    p95: null,
  });
});
