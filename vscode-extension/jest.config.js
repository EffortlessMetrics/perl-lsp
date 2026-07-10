/**
 * Jest configuration — runs the extension's unit suite against the JS emitted by
 * `tsc -p tsconfig.test.json` (into out-test/), with NO transformer.
 *
 * Previously this config used `preset: 'ts-jest'`, which transpiled each .ts on
 * the fly through the TypeScript compiler API. ts-jest pins a `typescript` peer
 * of `>=4.3 <7`, so that coupling blocked upgrading the extension to TypeScript
 * 7. Compiling ahead of time with tsc and running the emitted CommonJS directly
 * removes the ts-jest dependency entirely — the same tests, discovery, mocks,
 * coverage, and source-mapped failure locations, now decoupled from the compiler
 * version. No Babel / SWC / esbuild is introduced: `transform: {}` disables all
 * transformation and Jest executes the tsc output as-is.
 *
 * Discovery parity: ts-jest discovered live .ts sources, so a renamed/deleted
 * test vanished from the run immediately. Here Jest discovers emitted .js in
 * out-test/, and tsc never prunes stale outputs — so `compile:test` runs
 * `clean:test` (rm -rf out-test) before every emit. That keeps discovery keyed
 * to the current sources: a removed/renamed .ts leaves no orphan .js ghost suite
 * behind for Jest to keep running locally. (CI is already fresh — out-test/ is
 * gitignored and rebuilt from a clean `npm ci` checkout.)
 *
 * Source maps: tsconfig.test.json emits `inlineSourceMap` + `inlineSources`, so
 * stack traces and coverage map back to the original .ts (verified: a failing
 * assertion reports `src/test/<file>.ts:line:col`).
 *
 * Coverage: the v8 provider is used because it needs no transform step and honors
 * the emitted inline source maps to attribute coverage to the .ts sources.
 *
 * Integration (@vscode/test-electron) and published-smoke suites are Mocha and
 * are excluded from tsconfig.test.json, so they are never emitted into out-test
 * and never picked up here — they run via their own tsconfig + runner.
 *
 * Targeted runs: `roots`/`testMatch` only see out-test/, so a source-path
 * filter like `--runTestsByPath src/test/commands.test.ts` would otherwise
 * match nothing. The `test`/`test:ci` scripts invoke Jest through
 * scripts/run-jest.js, which remaps any `.ts` argument under `src/` to the
 * corresponding `.js` path under `out-test/` before forwarding to Jest — so
 * source-path targeted runs still work.
 *
 * @type {import('jest').Config}
 */
module.exports = {
  testEnvironment: 'node',
  roots: ['<rootDir>/out-test/test'],
  testMatch: ['**/*.test.js'],
  moduleNameMapper: {
    '^vscode$': '<rootDir>/out-test/test/__mocks__/vscode.js',
  },
  transform: {},
  coverageProvider: 'v8',
};
