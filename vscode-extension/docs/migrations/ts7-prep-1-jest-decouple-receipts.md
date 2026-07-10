# TypeScript 7 migration — PREP-1: decouple Jest from the compiler API

Durable receipts for the first PR of the VS Code extension's TypeScript 6 → stable
TypeScript 7 migration. This PR is **preparatory only** — it does **not** upgrade the
compiler. It removes the one ecosystem coupling that has no forward path (ts-jest),
so a later PR can swap `typescript` 6 → 7 without ts-jest blocking it.

- Repo: `EffortlessMetrics/perl-lsp`
- Base commit (branch cut from `origin/main`): `ac9939f4235714b24b6a95064df03179bbbc035f`
- Scope: `vscode-extension/` only
- Toolchain used to produce these receipts: Node `v25.8.2`, npm `11.8.0`, pnpm `10.11.1`

## Decision: PREP_TRAIN (not LAND_DIRECT)

A direct `typescript` 6 → 7 swap is **impossible today** because two ecosystem
dependencies hard-cap their `typescript` peer below 7, and neither has any published
version that supports TS7:

| Dependency | Installed | `typescript` peer range | Supports TS7? | Newest published | Newest peer range |
|---|---|---|---|---|---|
| `ts-jest` | 29.4.11 | `>=4.3 <7` | **No** | 29.4.11 (`latest`) | `>=4.3 <7` |
| `@typescript-eslint/*` | 8.61.1 | `>=4.8.4 <6.1.0` | **No** | 8.63.0 (`latest`) | `>=4.8.4 <6.1.0` |

`typescript` dist-tags at probe time: `latest = 7.0.2`, `rc = 7.0.1-rc`,
`beta = 6.0.0-beta`. Stable 7 exists (`7.0.2`); `tsc` was probed, not
`@typescript/native-preview`/`tsgo`.

Empirical peer-conflict proof (no lockfile mutation, `--dry-run`):

```
$ npm install typescript@7.0.2 --dry-run
npm warn ERESOLVE overriding peer dependency
  Could not resolve dependency:
  peer typescript@">=4.8.4 <6.1.0" from @typescript-eslint/eslint-plugin@8.61.1
  peer typescript@">=4.8.4 <6.1.0" from @typescript-eslint/parser@8.61.1
  (ts-jest@29.4.11 peer typescript@">=4.3 <7" also excludes 7)
# typescript on disk unchanged: still 6.0.3
```

`ts-jest` is the load-bearing blocker: its newest release (`29.4.11`, also the
`latest` dist-tag — there is no `ts-jest@30`) still declares `typescript` peer
`>=4.3 <7`. No version bump can lift it. It must be **removed**, which is what this
PR does. (`typescript-eslint`'s cap is handled by a **separate follow-on** — it may
be liftable by a future release; it is out of scope for PREP-1.)

## Migration train (this PR is #1)

1. **PREP-1 (this PR):** decouple the Jest unit suite from ts-jest — compile with
   `tsc` and run Jest on the emitted JS, no transformer. Removes `ts-jest`. Green on
   TS6 today.
2. **PREP-2 (follow-on):** resolve the `@typescript-eslint` TS7 cap (compatible
   release when available, or lint adjustment).
3. **TS7 swap (follow-on):** `typescript` 6 → stable 7.0.x, regenerate lockfiles,
   make TS7 the canonical extension CI lane, prove emit/VSIX/startup parity.

## What PREP-1 changes

Jest previously ran through `preset: 'ts-jest'`, transpiling each `.ts` on the fly via
the TypeScript compiler API — the coupling that pins `typescript <7`. PREP-1 replaces
that with a **compile-ahead** pipeline:

- New `tsconfig.test.json`: compiles the extension sources **and** their Jest unit
  tests to CommonJS under `out-test/`, with `inlineSourceMap` + `inlineSources`.
- `jest.config.js`: `transform: {}` (no transformer — **no Babel / SWC / esbuild**),
  runs `out-test/test/**/*.test.js`, `moduleNameMapper` points at the emitted vscode
  mock, `coverageProvider: 'v8'` (needs no transform; honors the inline source maps).
- `package.json`: adds `compile:test` (`tsc -p ./tsconfig.test.json`); `test` and
  `test:ci` now run `compile:test` first. `ts-jest` removed from devDependencies.
- `.gitignore` / `.vscodeignore` / `eslint.config.js`: ignore `out-test/`; keep the
  new `tsconfig.test.json` out of the VSIX.

The existing CI lane (`ux-regression-gate.yml` → `extension-jest`) runs `npm ci → lint
→ compile → test:ci → test:grammar` and needs **no workflow edit**: `test:ci` now
compiles the tests itself. Integration (`@vscode/test-electron`) and published-smoke
suites are Mocha, keep their own tsconfig + runner, and are excluded from
`tsconfig.test.json`, so they are never emitted into `out-test/` or picked up by Jest.

## Behavior-preservation proofs (all on TS6, this PR's HEAD)

| Property | Baseline (ts-jest) | PREP-1 (tsc + Jest-on-JS) |
|---|---|---|
| `npm ci` | exit 0 | exit 0 |
| `npm run lint` (eslint) | exit 0 | exit 0 |
| `npm run compile` (prod `tsc -p ./`) | exit 0 | exit 0 |
| `npm run test:ci` | 643 passed, 1 skipped¹ | **643 passed, 1 skipped, exit 0** |
| `npm run test:grammar` | exit 0 | exit 0 |
| Integration/published `tsc` compile | exit 0 | exit 0 (unchanged path) |
| Coverage | collected (ts-jest maps to `.ts`) | collected (v8 maps to `.ts` via inline maps) |
| Failure locations | `.ts:line:col` | `.ts:line:col` (verified²) |
| Production `out/` payload | 17 `.js` (+ `out/test/**`) | identical set |
| `typescript` in tree | single `6.0.3` | single `6.0.3` |

¹ Baseline `test:ci` also reports 3 FAILED suites **on Windows only**: the
`testPathIgnorePatterns` entries (`<rootDir>/src/test/integration/` etc.) use forward
slashes that do not match Windows `\` paths, so Jest mis-runs the Mocha
integration/published files (`suite is not defined`). On the Linux CI runner those
patterns match and the suites are correctly ignored — the gate is green. Confirmed by
re-running with a cross-platform ignore regex: `23 passed, 1 skipped, 643 tests,
exit 0`. PREP-1 removes this fragility entirely: those files are no longer compiled
into `out-test/`, so Jest never sees them regardless of OS.

² Source-map failure-location check: a throwaway failing assertion produced
`at Object.<anonymous> (src/test/_smoke_srcmap.test.ts:4:19)` — the stack maps to the
original `.ts` line/column, not the emitted `.js`.

## VSIX inventory parity — IDENTICAL

`npx @vscode/vsce ls` before vs after: **370 files both**, `diff` empty. No
`out-test/`, no `tsconfig.test.json`, no `ts-jest`/tsc/test-tooling added to the
package. (Pre-existing note, out of scope: the baseline VSIX already ships some dev
config — `jest.config.js`, `eslint.config.js`, `tsconfig.integration.json`,
`tsconfig.published-smoke.json`; PREP-1 does not add to or remove from that set. A
`.vscodeignore` cleanup is a reasonable separate follow-up.)

## Lockfile graph — npm (authoritative), surgical

`package-lock.json` regenerated by `npm install` against the edited manifest:

- **Removed (8):** `ts-jest`, `bs-logger`, `handlebars`, `lodash.memoize`,
  `make-error`, `neo-async`, `uglify-js`, `wordwrap` (ts-jest + its exclusive
  transitive closure).
- **Added: 0. Version-changed: 0.** npm preserved every existing pin — no general
  dependency refresh.
- `npm ci` (frozen contract) against the new lockfile: exit 0. `ts-jest` references in
  `package-lock.json`: 0.

## Package-manager policy — npm is the sole supported contract; broken pnpm-lock removed

Primary-artifact findings:

- **Every** extension CI/release workflow uses `npm ci` + `package-lock.json`
  (`ux-regression-gate.yml`, `vscode-managed-binary-smoke.yml`,
  `vscode-published-extension-smoke.yml`, `publish-extension.yml`). **No** workflow,
  script, or `packageManager` field references pnpm.
- The committed `pnpm-lock.yaml` is **broken independent of this change**:

  ```
  $ pnpm install --frozen-lockfile        # pristine HEAD, unmodified
  ERR_PNPM_BROKEN_LOCKFILE  The lockfile ... is broken: duplicated mapping key (2863:3)
  ```

  It contains duplicated `packages:` entries (e.g. `vscode-oniguruma@1.7.0`,
  `vscode-textmate@7.0.4`, `vscode-tmgrammar-test@0.1.3` each appear 4× consecutively).
  No `pnpm install --frozen-lockfile` could ever have passed on it.

Decision for PREP-1: **remove `pnpm-lock.yaml` and the dormant `pnpm.overrides` block
in favor of npm-only.** The lockfile was already broken (`ERR_PNPM_BROKEN_LOCKFILE`,
duplicate package entries), consumed by no install/script path (no `packageManager`
field; dependabot registers only the `npm` ecosystem for `/vscode-extension`), and
excluded from the VSIX (`.vscodeignore`). The `pnpm.overrides` block was package-level
config that only pnpm reads — never applied on any real install (all install/CI paths
are npm; the pnpm lock could never install) — so removing it changes **zero** installed
versions: `npm ci` and `package-lock.json` are byte-identical before and after (npm does
not read the `pnpm` key). It is also not a live safety control here, in either
direction: the npm-resolved dev-only dependencies it named remain at `diff@7.0.0` and
`serialize-javascript@6.0.2`, and per `npm audit` (run on this HEAD) both currently
carry **active** advisories — `diff@7.0.0`: low-severity DoS in `parsePatch`/
`applyPatch` ([GHSA-73rr-hh4g-fpgx](https://github.com/advisories/GHSA-73rr-hh4g-fpgx),
fixed in 8.0.3); `serialize-javascript@6.0.2`: high-severity RCE via `RegExp.flags`/
`Date.prototype.toISOString()` ([GHSA-5c6j-r48x-rmvq](https://github.com/advisories/GHSA-5c6j-r48x-rmvq),
fixed in 7.0.3) plus a moderate CPU-exhaustion DoS
([GHSA-qj8w-gfj5-8c6v](https://github.com/advisories/GHSA-qj8w-gfj5-8c6v), fixed in
7.0.5). Both are transitive via `mocha` (dev-only; used by the unrelated
`@vscode/test-electron` integration/published-smoke suites, not by this PR's Jest
change), pre-date PREP-1, and `npm audit`'s only fix path is a semver-major
`mocha@11.3.0+` bump — out of scope here. Removing the pnpm override does not change
any of this: npm never read it, so the installed tree — and its advisory exposure —
is identical before and after this PR. Keeping a half-configuration (pnpm overrides
but no pnpm lockfile) would have left a trap where a future `pnpm install` resolves a
tree that diverges from the authoritative npm one; removing both leaves a single,
fully-consistent npm-only contract.
After this PR removed `ts-jest` from `package.json`/`package-lock.json`, the stale lock
also disagreed with the manifest. Deleting the lock (and the orphaned overrides) is the
minimal, **validatable**, contract-safe resolution of that disagreement:

- It introduces **zero** dependency version changes (unlike a clean `pnpm install`, which
  performs a forbidden ~128-pkg in-range refresh: `eslint` 10.5.0→10.6.0,
  `@typescript-eslint` 8.61.1→8.63.0, `@types/node`, `tar`, `vscode-languageclient`,
  `@babel/*`, …).
- It needs no `--frozen-lockfile` validation (a hand-prune could not be validated because
  the pre-existing duplicate keys make `--frozen-lockfile` abort regardless).
- npm remains the single authoritative, fully-consistent contract (`npm ci` exit 0; VSIX
  inventory unchanged — the lock was never in the VSIX).

**Follow-up (package-manager-policy):** if the project later wants pnpm support, add a
pnpm CI lane and regenerate a fresh `pnpm-lock.yaml` from scratch at that time (the
deleted lock had zero forward value — a future lane would regenerate it anyway).

## Commands (reproduce)

```bash
cd vscode-extension
npm ci
npm run lint
npm run compile          # production tsc -p ./  → out/
npm run test:ci          # compile:test (tsc -p tsconfig.test.json → out-test/) + jest --ci --coverage
npm run test:grammar
npx @vscode/vsce ls      # 370 files, no out-test/ or tsconfig.test.json
```
