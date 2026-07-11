#!/usr/bin/env node

/**
 * Thin wrapper around the Jest CLI that remaps `--runTestsByPath` operands
 * under `src/` to their compiled `out-test/` counterparts before handing off
 * to Jest.
 *
 * jest.config.js runs Jest directly against tsc's compile-ahead output
 * (`roots: ['<rootDir>/out-test/test']`, `transform: {}` — no ts-jest, no
 * Babel/SWC/esbuild). Jest itself never sees a `.ts` path. Without this
 * wrapper, a targeted run such as
 *   npm test -- --runTestsByPath src/test/commands.test.ts
 * silently matches nothing: `--runTestsByPath` compares literal file paths,
 * and the source path was never on Jest's radar.
 *
 * This script only rewrites arguments in `--runTestsByPath` operand
 * position, recognizing every form Jest itself accepts for that flag.
 * `runTestsByPath` is declared `type: 'boolean'` in jest-cli's own option
 * table (node_modules/jest-cli/build/index.js) with no explicit `alias` —
 * `--run-tests-by-path` only works because yargs' built-in camelCase
 * expansion auto-generates a kebab-case alias for every camelCase option.
 * That makes the full set of forms Jest accepts exactly the 2x2 cross
 * product of {camelCase, kebab-case} x {space-separated, `=`-joined}:
 *   --runTestsByPath <path>       --runTestsByPath=<path>
 *   --run-tests-by-path <path>    --run-tests-by-path=<path>
 * plus further bare paths immediately following the flag (Jest allows
 * multiple). It does not rewrite arbitrary `src/`-prefixed tokens: an
 * unrelated option value that happens to start with `src/` (e.g.
 * `--testNamePattern src/test`) passes through untouched, because it never
 * occupies `--runTestsByPath` operand position. So
 * `npm test -- --runTestsByPath src/test/commands.test.ts` keeps working
 * exactly as before the ts-jest removal — now against the emitted JS, with
 * coverage and failure locations still mapped back to the .ts via the inline
 * source maps `tsconfig.test.json` emits.
 */

const path = require('path');

const ROOT = path.join(__dirname, '..');
const SRC_DIR = path.join(ROOT, 'src');
const OUT_TEST_DIR = path.join(ROOT, 'out-test');
const RUN_TESTS_BY_PATH_FLAG = '--runTestsByPath';
const RUN_TESTS_BY_PATH_FLAG_ALIASES = [RUN_TESTS_BY_PATH_FLAG, '--run-tests-by-path'];

function mapSourcePathToCompiled(arg) {
  if (!arg) {
    return arg;
  }

  const absolute = path.isAbsolute(arg) ? arg : path.resolve(ROOT, arg);
  const relativeToSrc = path.relative(SRC_DIR, absolute);

  // Not under src/ — e.g. already an out-test/ path. Leave it alone.
  if (relativeToSrc.startsWith('..') || path.isAbsolute(relativeToSrc)) {
    return arg;
  }

  return path.join(OUT_TEST_DIR, withCompiledExtension(relativeToSrc));
}

function withCompiledExtension(relativePath) {
  if (relativePath.endsWith('.ts')) {
    return `${relativePath.slice(0, -3)}.js`;
  }
  if (relativePath.endsWith('.js')) {
    return relativePath;
  }
  // No recognized extension (e.g. `src/test/commands.test`, matching how
  // some Jest CLI flags accept an extension-less path) — tsc always emits
  // `.js`, so that is the only compiled artifact that could exist.
  return `${relativePath}.js`;
}

// Match a bare or `=`-joined --runTestsByPath flag in any spelling Jest
// accepts. Returns { value } where value is the `=`-joined operand (or null
// for the bare space-separated form), or null if arg isn't this flag at all.
function matchRunTestsByPathFlag(arg) {
  for (const alias of RUN_TESTS_BY_PATH_FLAG_ALIASES) {
    if (arg === alias) {
      return { value: null };
    }
    if (arg.startsWith(`${alias}=`)) {
      return { value: arg.slice(alias.length + 1) };
    }
  }
  return null;
}

// Rewrite `--runTestsByPath` operands only (any accepted spelling) —
// everything else (flags, other option values, already-compiled paths)
// passes through untouched. This is deliberately narrower than "any
// src/-prefixed argument": an option value that happens to start with
// `src/` but isn't a --runTestsByPath operand (e.g. `--testNamePattern
// src/test`) must not be treated as a file path.
//
// The `=<path>` forms are split into the two-token `--runTestsByPath
// <path>` form rather than reassembled as `<flag>=<mapped>`: Jest's own
// CLI (yargs, `runTestsByPath` is boolean-typed) does not treat the
// `=value` form as a path operand for either spelling — verified directly
// against `node_modules/jest/bin/jest.js` on this HEAD, where
// `--runTestsByPath=out-test/test/commands.test.js` (and the kebab-case
// equivalent) silently runs the entire suite ("Ran all test suites"),
// while `--runTestsByPath out-test/test/commands.test.js` correctly
// scopes to that one file ("Ran all test suites within paths ...").
// Splitting to the two-token form (always emitted in canonical camelCase)
// is what makes the `=` syntax actually work end to end, for both
// spellings.
function remapRunTestsByPathArgs(argv) {
  const result = [];
  let collectingPaths = false;

  for (const arg of argv) {
    const match = matchRunTestsByPathFlag(arg);
    if (match) {
      if (match.value === null) {
        result.push(arg);
      } else {
        result.push(RUN_TESTS_BY_PATH_FLAG, mapSourcePathToCompiled(match.value));
      }
      collectingPaths = true;
      continue;
    }

    if (collectingPaths && !arg.startsWith('-')) {
      result.push(mapSourcePathToCompiled(arg));
      continue;
    }

    // Any other flag ends path collection — its own value (if any) is not a
    // --runTestsByPath operand and must not be remapped.
    if (arg.startsWith('-')) {
      collectingPaths = false;
    }

    result.push(arg);
  }

  return result;
}

// `--verbose` used to be hardcoded onto the base `npm test` script, which
// made every local run print a per-test-case listing regardless of whether
// anyone wanted it. It is opt-in now: `npm run test:ci` (and CI, which
// invokes that script) passes `--verbose` explicitly on the command line, and
// `JEST_VERBOSE` is available as an env toggle for anyone who wants verbose
// output from the plain `npm test` path locally without editing the script.
const VERBOSE_FLAG = '--verbose';
const JEST_VERBOSE_ENV_VAR = 'JEST_VERBOSE';
const TRUTHY_ENV_VALUES = new Set(['1', 'true', 'yes']);

function isVerboseEnvEnabled(env) {
  const raw = (env[JEST_VERBOSE_ENV_VAR] || '').trim().toLowerCase();
  return TRUTHY_ENV_VALUES.has(raw);
}

// Appends `--verbose` when the env toggle opts in, unless the caller already
// passed an explicit verbosity flag on the command line (in which case the
// explicit flag wins and the env toggle is not applied on top of it).
function applyVerboseEnvToggle(argv, env) {
  const hasExplicitVerboseFlag = argv.some(
    (arg) => arg === VERBOSE_FLAG || arg === '--verbose=false' || arg === '--silent',
  );
  if (hasExplicitVerboseFlag || !isVerboseEnvEnabled(env)) {
    return argv;
  }
  return [...argv, VERBOSE_FLAG];
}

function buildJestArgs(rawArgv, env) {
  return applyVerboseEnvToggle(remapRunTestsByPathArgs(rawArgv), env);
}

module.exports = {
  remapRunTestsByPathArgs,
  mapSourcePathToCompiled,
  isVerboseEnvEnabled,
  applyVerboseEnvToggle,
  buildJestArgs,
};

// Only run Jest when this file is executed directly (`node scripts/run-jest.js`),
// not when it is `require()`d — e.g. by tests that exercise the helpers above
// — which would otherwise recursively spawn a full Jest run in-process.
if (require.main === module) {
  const jestArgs = buildJestArgs(process.argv.slice(2), process.env);

  // Run Jest's own CLI entry point in-process (require it directly) rather
  // than spawning it as a child process. Two reasons:
  //   - `npx`/`npx.cmd` needs `shell: true` on Windows just to reach the
  //     .cmd shim — a quoting hazard avoided entirely by not shelling out.
  //   - `spawnSync` would add an extra process layer between this wrapper
  //     and Jest's own worker pool (jest-worker forks its own child
  //     processes for parallel test execution). That extra nesting caused
  //     worker forks to fail intermittently in CI ("Jest worker encountered
  //     N child process exceptions, exceeding retry limit") — a regression
  //     introduced by wrapping Jest in a subprocess at all. Requiring
  //     Jest's CLI entry directly makes this script's process the same
  //     top-level process Jest's workers fork from, exactly as when `jest`
  //     is invoked directly (no wrapper).
  const jestBin = require.resolve('jest/bin/jest', { paths: [ROOT] });
  process.chdir(ROOT);
  process.argv = [process.argv[0], jestBin, ...jestArgs];
  require(jestBin);
}
