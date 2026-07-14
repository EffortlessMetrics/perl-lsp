/**
 * Regression tests for #3676: Jest verbosity used to be hardcoded via
 * `--verbose` on the base `npm test` script, forcing a per-test-case listing
 * on every local run. These tests pin:
 *   - the base `test` script no longer hardcodes `--verbose` (quiet locally)
 *   - the `test:ci` script (what CI invokes) still runs Jest verbosely
 *   - scripts/run-jest.js's `JEST_VERBOSE` env toggle lets a caller opt back
 *     into verbose output from the plain `npm test` path, without forcing it
 *     on everyone
 */

import * as fs from 'fs';
import * as path from 'path';

const EXT_ROOT = path.resolve(__dirname, '..', '..');

// scripts/run-jest.js is a plain Node script, not compiled from src/ — this
// resolves relative to the *compiled* location of this test file
// (out-test/test/jestVerbosity.test.js), which lands back at
// vscode-extension/scripts/run-jest.js, exactly like the downloader tests'
// `require('vscode')` pattern requires a module outside src/.
const runJest = require('../../scripts/run-jest.js');

describe('package.json test script verbosity wiring', () => {
  test('base "test" script does not hardcode --verbose', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).not.toContain('--verbose');
  });

  test('"test:ci" script (what CI runs) still requests --verbose', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    expect(pkg.scripts['test:ci']).toContain('--verbose');
  });
});

describe('scripts/run-jest.js JEST_VERBOSE env toggle', () => {
  test('does not add --verbose when JEST_VERBOSE is unset', () => {
    expect(runJest.buildJestArgs(['--ci', '--coverage'], {})).toEqual(['--ci', '--coverage']);
  });

  test('does not add --verbose when JEST_VERBOSE is falsy', () => {
    expect(runJest.buildJestArgs(['--ci'], { JEST_VERBOSE: '0' })).toEqual(['--ci']);
    expect(runJest.buildJestArgs(['--ci'], { JEST_VERBOSE: 'false' })).toEqual(['--ci']);
  });

  test('adds --verbose when JEST_VERBOSE opts in', () => {
    expect(runJest.buildJestArgs(['--ci'], { JEST_VERBOSE: '1' })).toEqual(['--ci', '--verbose']);
    expect(runJest.buildJestArgs([], { JEST_VERBOSE: 'true' })).toEqual(['--verbose']);
    expect(runJest.buildJestArgs([], { JEST_VERBOSE: 'YES' })).toEqual(['--verbose']);
  });

  test('does not duplicate --verbose when it is already explicit on the command line', () => {
    expect(runJest.buildJestArgs(['--verbose'], { JEST_VERBOSE: '1' })).toEqual(['--verbose']);
  });

  test('does not add --verbose when the caller explicitly opted out', () => {
    expect(runJest.buildJestArgs(['--silent'], { JEST_VERBOSE: '1' })).toEqual(['--silent']);
    expect(runJest.buildJestArgs(['--verbose=false'], { JEST_VERBOSE: '1' })).toEqual([
      '--verbose=false',
    ]);
  });

  test('preserves --runTestsByPath remapping alongside the verbosity toggle', () => {
    const result = runJest.buildJestArgs(['--runTestsByPath', 'src/test/foo.test.ts'], {
      JEST_VERBOSE: '1',
    });
    expect(result).toEqual([
      '--runTestsByPath',
      path.join(EXT_ROOT, 'out-test', 'test', 'foo.test.js'),
      '--verbose',
    ]);
  });
});
