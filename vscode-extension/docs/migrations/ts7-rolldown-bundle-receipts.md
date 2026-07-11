# TS7 migration — Rolldown production bundle receipts (#3662)

The final step of the TypeScript 6 -> stable TypeScript 7 migration train.
Rolldown replaces TypeScript **emission** as the production artifact builder
— it does **not** type-check. TypeScript 7 (`tsc --noEmit`, the new
`typecheck` npm script) remains the sole type-check authority. Target flow:
typecheck -> TS7, lint -> oxlint, format -> oxfmt, bundle -> Rolldown, test
-> jest, package -> VSIX.

## 1. Version + install

```
$ npm view rolldown dist-tags
{ nightly: '1.0.0-beta.13-...', canary: '1.0.0-beta.31-...', latest: '1.1.5' }

$ npm install --save-dev --save-exact rolldown@1.1.5
added 8 packages, and audited 696 packages in 2s
```

No `--force`, no `--legacy-peer-deps` (Rolldown declares no
`peerDependencies`). `rolldown` is `1.1.5` in both `package.json` and the
regenerated `package-lock.json`, exactly pinned (no `^`/`~`, same discipline
as the Oxlint/Oxfmt/typescript pins from earlier PREPs).

## 2. Config (`rolldown.config.mjs`)

```js
export default defineConfig({
  input: 'src/extension.ts',
  tsconfig: './tsconfig.json',
  platform: 'node',
  external: (id) => id === 'vscode' || nodeBuiltins.has(id),
  output: {
    file: 'out/extension.js',
    format: 'cjs',
    sourcemap: true,
    minify: false,
    codeSplitting: false,
  },
});
```

- **Single CJS entry**: `src/extension.ts` -> `out/extension.js` — the exact
  path `package.json`'s `"main"` and the debugger's `"program"` already
  point at. Verified via `rolldown.test.ts`.
- **Node built-ins**: covered via `node:module`'s `builtinModules`, both the
  bare (`fs`) and `node:`-prefixed (`node:fs`) forms.
- **No minification** in this first PR, per the migration charter.
- **`codeSplitting: false`**: without this, Rolldown split a facade chunk
  (`out/commandResults.js`, near-empty) for a type-only
  `import type {...} from './commandResults'` in `extension.ts`, even
  though there is no real dynamic `import()` anywhere in the source
  (verified by grep — zero matches). CJS has no native async chunk-loading
  anyway, so any split there would just become a synchronous `require()` of
  a sibling file. This was caught during development, not assumed away.
- **`tsconfig: './tsconfig.json'`**: points Rolldown's TS transform at the
  same `esModuleInterop`/`allowSyntheticDefaultImports` settings `tsc`
  itself uses, so interop shape matches between the type-check pass and the
  bundle pass.

### `out/` hygiene: a real gap found and closed

`output.cleanDir` was tried first but does **not** apply in single-file
(`output.file`) mode — verified empirically: reproduced the exact failure
by running `tsc -p tsconfig.integration.json` (which also emits into the
shared `out/` directory, for the integration test harness) first, which
left a stray `out/commandResults.js` + `.map` behind as a byproduct of a
type-only cross-reference from a test file. `cleanDir: true` did **not**
remove it on a subsequent `npm run compile`. The actual fix: a `clean:out`
npm script (`node -e "..."`, matching the existing `clean:test` pattern)
that removes everything under `out/` **except** `out/test/**` (which the
separate integration/published-smoke `tsc` builds own) before Rolldown
runs — `"compile": "npm run clean:out && rolldown -c rolldown.config.mjs"`.
Re-reproduced the exact failure scenario after the fix and confirmed the
stray file is now removed while `out/test/**` survives untouched.

## 3. Dependency classification (bundled vs external)

| Dependency                                                                                   | Kind           | Classification           | Basis                                                                                                                                                                                                                       |
| -------------------------------------------------------------------------------------------- | -------------- | ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `vscode`                                                                                     | —              | **External (mandatory)** | Supplied by the extension host at runtime; not a real resolvable package.                                                                                                                                                   |
| Node built-ins (`fs`, `path`, `os`, `http`, `https`, `crypto`, `child_process`, `util`, ...) | —              | **External**             | Node resolves these natively; bundling would be both wrong and pointless.                                                                                                                                                   |
| `adm-zip`                                                                                    | production dep | **Bundled**              | Pure JS, zero `__dirname` usage, zero `.node` native bindings anywhere in its tree (verified via `grep`/`find`).                                                                                                            |
| `tar`                                                                                        | production dep | **Bundled**              | Pure JS, zero `__dirname` usage, zero `.node` native bindings anywhere in its transitive tree (including `mkdirp`, `minipass`, `chownr`, `@isaacs/fs-minipass`, etc. — checked the full subtree, not just the top package). |
| `vscode-languageclient`                                                                      | production dep | **Bundled**              | Pure JS/TS, itself already depends on `vscode` as external.                                                                                                                                                                 |

Before deciding, grepped the entire `src/` tree for the actual risk
patterns the migration charter called out:

```
__dirname usage:                zero (outside src/test/**)
dynamic import():                zero anywhere
dynamic require(computed path):  zero — the one require() call is
                                  require('fs') (a Node built-in, static
                                  string literal) inside debugAdapter.ts
process.platform / process.arch: used only for string construction (spawn
                                  target paths for the downloaded LSP
                                  binary), never for dynamic module require
child_process usage:              downloader.ts, extension.ts, onboarding.ts,
                                  startupDiagnosis.ts, testAdapter.ts — all
                                  spawn EXTERNAL processes (perl, perltidy,
                                  perlcritic, the downloaded perllsp binary)
                                  via path strings, never require() other
                                  application modules
worker_threads:                   not used
.json imports as modules:         zero (resolveJsonModule is declared but
                                  never actually exercised by application code)
```

This confirms the "naive bundle breaks `__dirname`/dynamic-require" risk the
charter warns about doesn't actually apply to this codebase's _own_ source —
the real risk surface was entirely in whether the three bundled
dependencies (adm-zip/tar/vscode-languageclient) behave correctly once
inlined, which is what the parity proof below directly tests (not just
greps for).

## 4. Assets — never imported, so never bundled

Grammars (`syntaxes/*.tmLanguage.json`), walkthrough media
(`media/walkthrough/*.svg`), snippets (`snippets/*.json`), configuration
files (`language-configuration.json`, `gherkin-language-configuration.json`)
are never `import`ed or `require()`d as JS/JSON modules anywhere in
`src/**` (verified: zero `.json` imports in application code). They are
physical files in the extension root, packaged by `vsce package` via
`.vscodeignore`'s inclusion rules — a process entirely independent of
whatever builds `out/extension.js`. Switching from `tsc` to Rolldown changes
nothing about how these assets reach the VSIX; they were never part of the
compile step's job in the first place. Platform LSP/DAP binaries are
downloaded at runtime into `context.globalStorageUri` (the "managed binary"
flow) — never bundled or imported; this flow itself is directly exercised
end-to-end in the parity proof below.

## 5. Parity proof

### Existing real end-to-end test harnesses used as the proof mechanism

This repo already has two Electron-based, real-VS-Code-host integration
suites (`@vscode/test-electron`) that were not previously used for a
bundler-swap proof but turned out to be exactly the right tool:

- **`npm run test:integration`** — loads the actual unpacked extension
  (`out/extension.js` via `package.json`'s `"main"`) into a real, downloaded
  VS Code instance and exercises the `Managed binary smoke` suite:
  activation, `perl-lsp.reinstall` (auto-download + SHA256SUMS + archive
  extraction via the bundled `adm-zip`/`tar`), spawning the downloaded
  `perllsp` binary to hold a file lock, a second reinstall under lock,
  health checks. **This is the single most load-bearing test for this
  PR** — it directly exercises every dependency this PR reclassified as
  "bundled."
- **`npm run test:published`** with `PERL_LSP_PUBLISHED_EXTENSION_SOURCE=vsix`
  - `PERL_LSP_PUBLISHED_VSIX_PATH=<path>` — installs the **real packaged
    `.vsix`** into a clean `--extensions-dir` (no dev-mode loading, no
    `node_modules/` on disk for the extension to reach for) and runs the same
    class of managed-binary command flow against the genuinely-installed
    extension. This is the strongest available proof: it's the artifact an
    end user would actually receive.

  (The default `test:published` invocation, with no env vars, installs from
  the **marketplace** — an unrelated already-published version. Discovered
  this while investigating: an unqualified first run "passed" but was
  silently testing nothing relevant to this PR. Re-ran with the `vsix`
  source explicitly pointed at the freshly-packaged local VSIX for a
  meaningful result.)

### Each parity item, proven

| Item                                                    | Result                                                                                                                                                                                                                                                                                                |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Activation succeeds                                     | ✅ `extension.activate()` resolves within 30s in both `test:integration` and `test:published` (vsix source)                                                                                                                                                                                           |
| Every registered command exists                         | ✅ `waitForCommand('perl-lsp.reinstall', ...)` and `vscode.commands.executeCommand('perl-lsp.reinstall'/'perl-lsp.runHealthCheck')` succeed                                                                                                                                                           |
| Native Rust LSP launches                                | ✅ the downloaded `perllsp` binary is spawned and holds a file lock across a second reinstall (proves the process starts and stays alive, i.e. isn't crashing on startup)                                                                                                                             |
| Native DAP launches/resolves configs                    | Not separately exercised by the existing smoke suites (out of scope of what they test) — DAP registration code itself is bundled identically to every other `src/*.ts` module and is exercised by the full `npm run test:ci` jest suite (`debugAdapter.test.ts`), which passed unchanged (661/1 skip) |
| Auto-download + archive extraction work                 | ✅ `reinstall1`/`reinstall2` both succeed, `checksumVerified: true`, `fs.existsSync(serverPath)` true — this is `adm-zip`+`tar` (both bundled) doing real extraction against a real downloaded archive                                                                                                |
| Extension context/resource paths resolve                | ✅ `context.globalStorageUri`-based binary paths resolve correctly (the reinstall flow depends on this); `context.extensionPath`-based demo-project/CHANGELOG paths are exercised by `packageManifest.test.ts` / `whatsNew.test.ts` in the jest suite, unchanged                                      |
| Grammars/snippets/walkthroughs/config survive packaging | ✅ confirmed present in the packaged VSIX (`vsce ls`): `syntaxes/*.tmLanguage.json`, `snippets/*.json`, `media/walkthrough/*.svg`, `language-configuration.json`, `gherkin-language-configuration.json` — unaffected by the Rolldown change since they were never part of the compile step            |
| Published-smoke tests pass                              | ✅ against the real packaged VSIX (source=vsix), not just marketplace — see above                                                                                                                                                                                                                     |
| VSIX inventory + size inspected                         | ✅ see section 6                                                                                                                                                                                                                                                                                      |

## 6. VSIX inventory + size delta

```
BEFORE (tsc, unbundled, current main):  458 files, 1.25 MB
AFTER  (Rolldown, bundled):              33 files, 291 KB
```

**~93% fewer files, ~77% smaller package.** The entire delta is
`node_modules/**` (408 files, 5.41 MB uncompressed) now being excluded —
every runtime dependency Rolldown bundles into `out/extension.js` no longer
needs to ship as raw files. `out/` itself went from 17 separate `.js` files
(326 KB) to exactly 1 file (`out/extension.js`, 1.22 MB unminified — flagged
by `vsce` as "large," expected and accepted per the migration charter's
explicit no-minification scope for this PR; minification is the natural
next lever if size becomes a concern later).

```
$ vsce ls | grep -iE "rolldown|oxlint|oxfmt|node_modules"
.oxlintrc.json
.oxfmtrc.json
```

Zero `node_modules/**` entries of any kind (not just dev-tooling — the
production dependencies too, since they're now bundled). Zero Rolldown
artifacts (`rolldown.config.mjs` excluded via `.vscodeignore`). The two
`.rc.json` config files are the same harmless, already-documented inclusion
from PREP-2/PREP-3.

### Regression test hardened, not just manually checked

`src/test/rolldown.test.ts` (new) asserts against the real `vsce ls`
manifest — not config intent — that: zero `node_modules/**` entries exist,
`out/` contains exactly one file (`out/extension.js`, not a facade-chunk
split), and no `rolldown.config.mjs`/stray `.rc.json` leaks. It builds
`out/extension.js` itself in a `beforeAll` (via direct `node` invocation of
Rolldown's own CLI entry, not `npm.cmd`/`npx` — see the Windows spawn note
below) so the suite is self-sufficient regardless of what ran before it,
rather than silently passing a vacuous "zero files" check if `out/` doesn't
exist yet. This exact gap was caught during development: an early version
of this test assumed `out/extension.js` already existed from a prior
manual build step and would have silently "passed" (checking an empty
array against an empty array) in a fresh CI checkout where nothing had
built it yet.

**Windows spawn note**: the `beforeAll` and the moved VSIX-inventory test
both invoke `node_modules/rolldown/bin/cli.mjs` and
`node_modules/@vscode/vsce/vsce` directly via `execFileSync(process.execPath,
[entry, ...args])` rather than `npm.cmd`/`npx` (which `EINVAL`s under
`spawnSync` without `shell: true`, and `shell: true` string-concatenates
args instead of escaping them) — same class of fix already applied in
`scripts/lint-canary.js` during PREP-2.

## 7. Pre-existing bug fixed forward (blocked VSIX packaging entirely)

`vsce package` failed outright before any of this PR's changes could even
be tested: `@types/vscode` was `~1.125.0` while `package.json`'s
`engines.vscode` was still `^1.120.0` — a mismatch left by an unrelated
dependabot PR (`6fa71cdeb`, bumped `@types/vscode` without updating
`engines.vscode` in lockstep). Confirmed via `git log`/`git show` this
predates this PR entirely. Bumped `engines.vscode` to `^1.125.0` to match —
a one-line, minimal, clearly-scoped fix, bundled into this PR only because
VSIX packaging is a hard prerequisite for the parity proof this PR is
required to deliver.

## 8. Full toolchain green (clean `npm ci`, every gate)

```
npm run lint         -> canary PASS + oxlint --type-aware clean, exit 0
npm run fmt:check    -> All matched files use the correct format, exit 0
npm run typecheck    -> tsc --noEmit -p ./tsconfig.json, exit 0 (TS7)
npm run compile      -> rolldown -c rolldown.config.mjs, exit 0 (single file)
npm run test:ci      -> Test Suites: 1 skipped, 25 passed, 25 of 26 total
                         Tests: 1 skipped, 661 passed, 662 total
npm run test:grammar -> 9/9 fixtures pass, exit 0
npm run test:integration -> 1 passing (real VS Code host, dev-mode load)
npm run test:published (source=vsix) -> 1 passing (real installed VSIX)
```

## 9. CI wiring

`.github/workflows/ux-regression-gate.yml`'s existing `extension-jest` job
(`ubuntu-24.04`, no new job, no Windows runner): the old dual-purpose
"Typecheck (tsc)" step (which ran `npm run compile`, doing both type-check
and build at once) is split into two clearly-named, sequential steps —
`Typecheck (tsc --noEmit, TS7)` running `npm run typecheck`, then
`Build (rolldown)` running `npm run compile` — matching the target flow
(typecheck -> TS7, bundle -> Rolldown) and making a Rolldown-specific build
failure surface distinctly from a type error.

## 10. Review finding: `watch` broke the documented dev-loop contract

chatgpt-codex-connector caught a real regression on PR #3755: the original
version of this change kept `"watch": "tsc -watch --noEmit -p ./tsconfig.json"`
— type-check-only. `DEVELOPMENT.md` documents `npm run watch` as "Rebuild
on every file change (use during active development)," and `package.json`'s
`"main"` loads `./out/extension.js` as the actual runtime entry. With the
original change, editing `src/extension.ts` while `npm run watch` ran would
report type errors correctly but silently leave the stale Rolldown bundle
in place — VS Code's "Run Extension" (F5) would launch old code with no
indication anything was wrong.

Fixed: `"watch": "npm run clean:out && rolldown -c rolldown.config.mjs --watch"`
— Rolldown's own watch mode, which rebuilds `out/extension.js` on real code
changes (matching the documented contract). Added a separate
`"watch:types": "tsc -watch --noEmit -p ./tsconfig.json"` script for anyone
who wants a terminal-based live type-check loop outside VS Code's own
built-in TypeScript language service (which already gives live
in-editor diagnostics independent of any script, for anyone editing inside
VS Code itself).

Verified Rolldown's watch mode is genuinely tracking the right files, not
a stale pass-through: editing `src/commandResults.ts` (imported only via
`import type`, i.e. zero runtime footprint) does **not** trigger a rebuild
— correct, since a type-only file's content cannot change the emitted
bundle. Editing `src/extension.ts` itself (always-loaded, real code) does
trigger a rebuild, confirmed via `out/extension.js`'s rebuild count and
Rolldown's own "Rebuilt out in Nms." log line. `DEVELOPMENT.md` updated to
document `typecheck`/`watch`/`watch:types` accurately.

## Scope boundary

This PR does not: touch the Jest pipeline (PREP-1), the Oxlint config
(PREP-2), the Oxfmt config (PREP-3), or the TS7 compiler swap itself — all
already merged and unmodified here beyond the one pre-existing
`engines.vscode` fix documented in section 7. This completes the Oxc/TS7
migration train: Jest decoupled -> ESLint replaced by Oxlint -> Prettier-free
Oxfmt adoption -> TypeScript 7 -> Rolldown production bundle.
