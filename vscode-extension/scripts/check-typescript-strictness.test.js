const assert = require('node:assert/strict');
const { test } = require('node:test');
const {
  compareToBaseline,
  parseDiagnostics,
  summarizeDiagnostics,
  validateCompilerResult,
  getOption,
} = require('./check-typescript-strictness');

void test('parses and buckets TypeScript diagnostics by config, code, and file', () => {
  const diagnostics = parseDiagnostics(
    'src/a.ts(4,9): error TS2532: Object is possibly undefined.\n',
    'tsconfig.json',
  );
  assert.deepEqual(summarizeDiagnostics(diagnostics), {
    total: 1,
    by_config: { 'tsconfig.json': 1 },
    by_code: { TS2532: 1 },
    by_file: { 'tsconfig.json:src/a.ts': 1 },
    diagnostics: [
      {
        config: 'tsconfig.json',
        file: 'src/a.ts',
        line: 4,
        column: 9,
        code: 'TS2532',
        message: 'Object is possibly undefined.',
      },
    ],
  });
});

void test('rejects total and bucket growth without absorbing new debt', () => {
  const actual = summarizeDiagnostics(
    parseDiagnostics(
      [
        'src/a.ts(4,9): error TS2532: Object is possibly undefined.',
        'src/b.ts(8,2): error TS2345: Argument is possibly undefined.',
      ].join('\n'),
      'tsconfig.json',
    ),
  );
  const violations = compareToBaseline(actual, {
    total: 1,
    by_config: { 'tsconfig.json': 1 },
    by_code: { TS2532: 1 },
    by_file: { 'tsconfig.json:src/a.ts': 1 },
  });
  assert.deepEqual(violations, [
    'total grew from 1 to 2',
    'config tsconfig.json grew from 1 to 2',
    'code TS2345 grew from 0 to 1',
    'file tsconfig.json:src/b.ts grew from 0 to 1',
  ]);
});

void test('rejects compiler failures without parseable diagnostics', () => {
  assert.throws(
    () =>
      validateCompilerResult(
        { status: 1, stdout: '', stderr: 'error TS5058: invalid project' },
        [],
        'missing.json',
      ),
    /TypeScript failed for missing\.json without parsed diagnostics/,
  );
});

void test('selects only supported strictness policies', () => {
  assert.equal(getOption(['node', 'check.js']), 'noUncheckedIndexedAccess');
  assert.equal(
    getOption(['node', 'check.js', '--option', 'exactOptionalPropertyTypes']),
    'exactOptionalPropertyTypes',
  );
  assert.throws(
    () => getOption(['node', 'check.js', '--option', 'unknownPolicy']),
    /Unsupported TypeScript strictness option: unknownPolicy/,
  );
});
