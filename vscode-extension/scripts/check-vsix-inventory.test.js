const assert = require('node:assert/strict');
const { test } = require('node:test');
const { compareInventory, summarizeInventory } = require('./check-vsix-inventory');

void test('summarizes packaged file sizes', () => {
  assert.deepEqual(
    summarizeInventory([
      { file: 'a.txt', bytes: 2 },
      { file: 'b.js', bytes: 3 },
    ]),
    {
      schema_version: 1,
      total_files: 2,
      total_bytes: 5,
      files: { 'a.txt': 2, 'b.js': 3 },
    },
  );
});

void test('rejects package growth and inventory drift', () => {
  const violations = compareInventory(
    { total_files: 2, total_bytes: 8, files: { 'a.txt': 5, 'new.js': 3 } },
    { total_files: 2, total_bytes: 5, files: { 'a.txt': 2, 'old.js': 3 } },
  );
  assert.deepEqual(violations, [
    'total bytes grew from 5 to 8',
    'file a.txt grew from 2 to 5 bytes',
    'new packaged file: new.js',
    'baseline packaged file is missing: old.js',
  ]);
});
