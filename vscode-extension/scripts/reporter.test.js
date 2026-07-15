'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { createReporter } = require('./reporter');

void test('reporter writes scoped informational and error output to separate streams', () => {
  const output = [];
  const errors = [];
  const reporter = createReporter('fixture', {
    stdout: { write: (chunk) => output.push(chunk) },
    stderr: { write: (chunk) => errors.push(chunk) },
  });

  reporter.info('ready');
  reporter.error('failed');

  assert.deepEqual(output, ['[fixture] ready\n']);
  assert.deepEqual(errors, ['[fixture] failed\n']);
});
