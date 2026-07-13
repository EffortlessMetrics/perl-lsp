const assert = require('node:assert/strict');
const path = require('node:path');
const { test } = require('node:test');
const { parseRunCount, sampleDirectory } = require('./sample-local-vsix-smoke');

void test('defaults to three samples and accepts explicit counts', () => {
  assert.equal(parseRunCount(['node', 'sample.js'], {}), 3);
  assert.equal(parseRunCount(['node', 'sample.js'], { PERL_LSP_VSCODE_SAMPLE_RUNS: '5' }), 5);
  assert.equal(parseRunCount(['node', 'sample.js', '--runs', '7'], {}), 7);
});

void test('rejects invalid sample counts and keeps receipt runs separate', () => {
  assert.throws(
    () => parseRunCount(['node', 'sample.js', '--runs', '0'], {}),
    /Sample count must be a positive integer/,
  );
  assert.equal(sampleDirectory('receipts', 3), path.join('receipts', 'sample-03'));
});
