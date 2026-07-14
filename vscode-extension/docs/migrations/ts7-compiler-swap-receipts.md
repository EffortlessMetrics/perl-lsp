# TS7 migration — compiler swap receipts (#3662)

The payoff step of the TypeScript 6 -> 7 migration train. Swaps the
`typescript` devDependency from `^6.0.3` to stable `^7.0.2` — the real
compiler, no alias, no shim, no override. This was blocked until PREP-2
(#3690) removed `@typescript-eslint`'s `typescript >=4.8.4 <6.1.0` peer cap
(the only remaining coupling to the TS6 compiler API); with that gone,
TypeScript 7 installs like any other dependency bump.

## 1. Version installed + no override needed

```
$ npm view typescript dist-tags
{ ..., latest: '7.0.2', next: '7.1.0-dev.20260710.1', rc: '7.0.1-rc' }

$ npm install --save-dev typescript@7.0.2
added 1 package, changed 1 package, and audited 688 packages in 3s
```

**No `--force`, no `--legacy-peer-deps`, no ERESOLVE conflict of any kind.**
`package.json`'s `typescript` entry is now `^7.0.2` (same caret-range
convention the file already used for `^6.0.3` — this repo doesn't pin the
compiler itself exactly, only the alpha/beta dev-tooling around it per the
PREP-2/PREP-3 discipline). Confirmed the resolved binary:

```
$ ./node_modules/.bin/tsc --version
Version 7.0.2
```

## 2. `ignoreDeprecations: "6.0"` removed

Removed from `tsconfig.json` (the only tsconfig that had it):

```diff
-    "ignoreDeprecations": "6.0",
     "allowSyntheticDefaultImports": true,
```

Nuance for the record: TS 7.0.2 does **not** hard-error if this option is
left in (a scratch probe with `"ignoreDeprecations": "6.0"` still extended
still compiled clean, exit 0) — it's tolerated, not rejected. It is
nonetheless a TS6-era escape hatch for deprecation warnings that no longer
apply once you're actually on TS7, so removing it is the correct, deliberate
cleanup regardless of whether leaving it in would have technically failed.

## 3. All four tsconfigs compiled under real TS7 — zero diagnostics

Re-verified on the current merged tree (post PREP-2 Oxlint + PREP-3 Oxfmt),
not just the earlier pre-merge investigation:

```
$ tsc -p ./tsconfig.json --noEmit                    exit 0
$ tsc -p ./tsconfig.test.json --noEmit                exit 0
$ tsc -p ./tsconfig.integration.json --noEmit         exit 0
$ tsc -p ./tsconfig.published-smoke.json --noEmit     exit 0
```

**Zero new diagnostics, zero source fixes needed.** The earlier
compile-parity investigation (before PREP-2/3 landed) already proved this
config was TS7-clean; Oxlint and Oxfmt landing since didn't introduce
anything TS7-incompatible.

### Emit parity re-verified (not just type-check)

Installed a scratch `typescript@6.0.3` alongside the real `typescript@7.0.2`
and emitted the main build with both, into separate directories:

```
TS6 emit exit: 0    (17 .js files)
TS7 emit exit: 0    (17 .js files)
diff -rq --exclude="*.map" <ts6-out> <ts7-out>   -> no output (identical)
```

Byte-identical `.js` output between TS6 and TS7 on the real, current tree.

## 4. Watch mode verified separately

```
$ tsc --watch -p ./tsconfig.json
Starting compilation in watch mode...
Found 0 errors. Watching for file changes.
```

Confirmed watch mode is not just reporting a stale "0 errors" — two live
probes, each reverted immediately after:

- **Incremental recompile on a benign new file**: added
  `src/__ts7_watch_probe.ts` (a trivial `export const`) — watcher detected
  the change, recompiled, reported "Found 0 errors" again. Removed the
  file — watcher recompiled again, still clean. (File never committed;
  `git status` confirms.)
- **Real error detection**: added `src/__ts7_watch_error_probe.ts` with a
  genuine type mismatch (`const x: number = 'not a number'`) — watcher
  reported `error TS2322: Type 'string' is not assignable to type
'number'.` and "Found 1 error." Removed the file — watcher recompiled and
  cleared back to "Found 0 errors." Confirms the watcher genuinely
  type-checks under TS7, not a stale pass-through. (File never committed.)

## 5. No TS6 remnant — proven, not assumed

```
$ node -e "console.log(require('./node_modules/typescript/package.json').version)"
7.0.2

$ find node_modules -maxdepth 1 -iname "typescript*" -o -maxdepth 1 -iname "@typescript*"
node_modules/@typescript
node_modules/typescript

$ grep -n "typescript6\|@typescript/native\|npm:typescript" package-lock.json
(no matches)
```

The one `@typescript`-scoped entry present,
`node_modules/@typescript/typescript-win32-x64`, is **TypeScript 7's own
native platform binding** (part of `typescript@7.0.2`'s own
`optionalDependencies` — 20 platform/arch variants, same pattern as
Oxlint's/Oxfmt's `@oxlint/binding-*`/`@oxfmt/binding-*` packages), correctly
version-matched at `7.0.2`. Confirmed by reading
`node_modules/typescript/package.json`'s own `optionalDependencies` field —
it lists exactly this package at exactly this version.

`package-lock.json`'s diff is exactly the old `typescript@6.0.3` entry's
fields replaced by the new `7.0.2` entry (version, resolved URL, integrity,
engines range) plus TS7's platform-binding subtree — nothing else in the
lockfile changed (`git diff` shows 8 removed lines, all from the old 6.0.3
entry; the rest is purely additive). One notable difference in the `bin`
field itself, not just its path: TS6's entry declared `bin: {tsc, tsserver}`;
TS7's declares `bin: {tsc}` only — **`tsserver` was removed, not moved**.
Confirmed against the live installed package
(`require('typescript/package.json').bin` -> `{ tsc: './bin/tsc' }`) and
both lockfile entries directly. This extension doesn't invoke `tsserver`
(it uses `tsc` for compile and its own Perl LSP for everything else), so
it's not functionally impactful here — but worth recording accurately since
it's a real behavioral difference in the package, not mere bin-path churn.

## 6. Full toolchain confirmed green post-swap

```
npm run lint         -> canary PASS + oxlint --type-aware clean, exit 0
npm run fmt:check    -> All matched files use the correct format, exit 0
npm run compile      -> tsc -p ./ under REAL typescript@7.0.2, exit 0
npm run test:ci      -> Test Suites: 1 skipped, 24 passed, 24 of 25 total
                         Tests: 1 skipped, 654 passed, 655 total (unchanged)
npm run test:grammar -> 9/9 fixtures pass, exit 0
```

`oxlint-tsgolint`'s type-aware backend was never coupled to the `typescript`
npm package version in the first place (it's backed by `typescript-go`
directly) — the canary passing here is expected continuity, not a new
result, but it's re-verified anyway as part of "confirm the whole toolchain
still green."

VSIX packaging re-checked: `vsce ls` shows no `typescript`-related entries
at all (the compiler is a devDependency and ships no `.rc.json`-equivalent
config file the way Oxlint/Oxfmt do), and the existing
`oxfmt.test.ts` VSIX-inventory regression test (added in PREP-3, extended
after review) still passes — confirming no oxfmt/oxlint/prettier binary
leak continues to hold after this change.

## Scope boundary

This PR does not touch: the Jest pipeline (PREP-1), the Oxlint config
(PREP-2), the Oxfmt config (PREP-3) — all already merged and unmodified
here. No alias, no peer override, no `--force`/`--legacy-peer-deps` was used
anywhere in this change. The final step in the train is Rolldown (the
production bundle for `out/extension.js`).
