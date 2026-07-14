# VS Code extension test runner decision

Status: retain Jest as the unit-test authority until a parity harness shows a
material operational improvement from another runner.

## Evidence captured

The probe ran from the current extension tree on 2026-07-13 with Node
`v26.5.0` and npm `10.8.2` (the repository's npm authority). The current Jest
pipeline completed these checks:

| Check                   | Result                             | Wall time |
| ----------------------- | ---------------------------------- | --------: |
| `npm run compile:test`  | pass                               |    2.05 s |
| `npm run test:ci`       | pass: 732 tests, 1 documented skip |   16.52 s |
| `npm run typecheck:all` | pass                               |    7.17 s |
| `npm run lint`          | pass: 446/446 warning budget       |    2.46 s |

The repository has no `vitest` dependency or configuration. A bounded
compatibility probe using `npx --yes vitest@4.1.10` resolved
`vitest/4.1.10 win32-x64 node-v26.5.0`; no package or lockfile was changed.
The probe took 4.36 s to fail, but that elapsed time is only the duration of
the compatibility probe, not a runner benchmark. It was not a comparable test
run: Vitest discovered both TypeScript test files under `src/test` (for
example, `src/test/**/*.test.ts`) and emitted Jest files under `out-test/test`
(for example, `out-test/test/**/*.test.js`), then failed 71 suites because the
current contract depends on Jest globals, `@jest/globals`, the Jest `vscode`
module mapper, and Jest-specific mocks. The probe therefore establishes
integration work, not a performance result.

## Decision

Keep Jest as the canonical unit runner. The current compile-ahead design is
already decoupled from `ts-jest`, works with TypeScript 7, preserves source
maps, and has a green full-suite proof. Introducing Vitest now would add a
runner migration, mock-environment conversion, discovery rules, and coverage
parity work without evidence of a material benefit.

This decision does not prohibit a future comparison. A migration proposal must
first provide a separate parity harness that:

1. runs the same unit-test surface without mixing source and emitted files;
2. preserves source-path targeted runs such as `--runTestsByPath
src/test/commands.test.ts` and clean-output discovery semantics;
3. preserves VS Code module mapping, Jest-style fake timers, `jest.mock(...)`
   calls, and source-mapped failures;
4. compares clean-install size, full-suite and focused-test time, watch
   latency, coverage parity, Windows behavior, failure output, and open-handle
   detection; and
5. records the commands, versions, environment, and limitations before
   recommending a switch.

Until that evidence exists, Jest remains the lower-risk and reproducible
authority. This record does not make a startup-performance claim and does not
change runtime extension behavior.
