# TS7 migration — PREP-3 receipts: adopt Oxfmt (#3662)

PREP-3 of the TypeScript 6 -> 7 migration train. Preparatory only: does
**not** upgrade the `typescript` compiler (stays `^6.0.3`). Adopts
[Oxfmt](https://oxc.rs) as the sole formatter for `vscode-extension/**` —
no Prettier, no other formatter. Scope is the extension only; a repo-wide
Markdown/JSON reformat is explicitly a separate, deliberately-reviewed
campaign, not part of this train.

## 1. Registry facts

```
$ npm view oxfmt versions   -> ... 0.56.0, 0.57.0, 0.58.0
$ npm view oxfmt dist-tags  -> { latest: '0.58.0' }
$ npm view oxfmt bin        -> { oxfmt: 'bin/oxfmt' }
$ npm view oxfmt peerDependencies      -> { svelte: '^5.0.0', 'vite-plus': '*' }
$ npm view oxfmt peerDependenciesMeta  -> { svelte: {optional:true}, 'vite-plus': {optional:true} }
```

Both peers are optional and unused here (no Svelte, no Vite). `npm install
--save-dev --save-exact oxfmt@0.58.0` resolved cleanly — no
`--force`/`--legacy-peer-deps`. Oxfmt 0.58.0 is beta; that is accepted for a
formatter gate (lower blast radius than a compiler or type-checker), same
posture as Oxlint's alpha type-aware linting in PREP-2 — exact version
pinned for the same reason.

## 2. Config decisions (`.oxfmtrc.json`)

```json
{
  "$schema": "./node_modules/oxfmt/configuration_schema.json",
  "ignorePatterns": ["syntaxes/**", "test/grammar/fixtures/**", "package-lock.json"],
  "sortPackageJson": false,
  "singleQuote": true
}
```

### `sortPackageJson` — inspected before enabling, per the migration charter

Oxfmt defaults `sortPackageJson` to `true`. Before accepting the default,
built a scratch copy of `package.json` and ran Oxfmt with
`sortPackageJson: true` against it in isolation:

- Top-level key order (`name`, `displayName`, `description`, ... `scripts`,
  `dependencies`, `devDependencies`) turned out to be **already identical**
  to the sorted output — this file's curated order happens to match Oxfmt's
  package.json convention at the top level.
- However, the _full_ diff was **not** a no-op: `sortPackageJson: true`
  applies a different, more compact printer to package.json specifically —
  arrays like `categories` and `keywords` collapsed onto single lines, and
  nested objects reflowed under different wrapping rules. Measured diff on
  the scratch copy: **1069 insertions / 1160 deletions / 158 modified
  lines** (`diff` tool's own summary).

Given the charter's goal ("first format application as close to a no-op as
feasible"), `sortPackageJson: false` was the deliberate choice.
**Verified**: with this setting, `oxfmt --check package.json` on the real
file reports "All matched files use the correct format" — package.json is
a genuine, byte-identical no-op. The PREP-3 formatting commit does not
touch `package.json` at all (confirmed via `git diff --stat`).

### `singleQuote` — matched to the existing convention

Oxfmt's own default is double quotes (`singleQuote: false`). A first
`--write` pass with defaults rewrote every string literal in every `.ts`
file from single to double quotes — e.g. `src/downloader.ts` alone went
from a 2016-line diff to fewer once corrected. Checked the actual
convention in the codebase: `src/extension.ts` has 25 `from '...'` single-
quoted imports and 0 double-quoted ones. Set `singleQuote: true` to match;
re-ran, confirmed the resulting diff no longer touches quote style at all
(only indentation/wrapping differences remain — see section 4).

### `ignorePatterns` — preserving generated/vendored/fixture surfaces

- `syntaxes/**` — the two TextMate grammar files
  (`perl.tmLanguage.json`, `gherkin.tmLanguage.json`) are large,
  externally-structured JSON; reformatting them would produce a huge,
  meaningless diff.
- `test/grammar/fixtures/**` — the `.pl`/`.pl.snap` grammar snapshot
  fixtures (unsupported languages/extension for Oxfmt anyway, but listed
  explicitly per the migration charter's instruction to preserve snapshot
  fixtures via ignores, not by accident of extension support).
- `package-lock.json` — npm's own generated lockfile format; must never be
  touched by a JS/JSON formatter. Verified: the lockfile diff in this PR is
  386 insertions / 0 deletions — purely additive (the new `oxfmt`
  dependency tree), zero reformatting of existing entries.

`node_modules/`, `out/`, `out-test/`, `.vscode-test/`, `coverage/`, `bin/`
are already covered by `.gitignore`, which Oxfmt respects by default (its
`--ignore-path` defaults to `.gitignore` + `.prettierignore` in the current
directory) — no explicit duplication needed in `ignorePatterns`.

## 3. Scope: `vscode-extension/**` only

Every touched file in both commits of this PR is under `vscode-extension/`.
No Rust source, no repo-root Markdown, no other package was reformatted.
`git diff --stat` against the base commit confirms this — the diff's path
list contains only `vscode-extension/*` and the two CI/dependabot config
files that wire the new gate (also legitimately in scope: they configure
how `vscode-extension/**` is checked, they don't reformat anything
themselves).

## 4. Two-commit structure: tooling, then pure format

Per the migration charter ("the first format application is ONE
formatting-only commit/PR — no semantic edits mixed in"), this PR is split
into exactly two commits:

1. **`build(vscode-extension): adopt Oxfmt tooling (no reformat yet)`** —
   `.oxfmtrc.json`, the `oxfmt` devDependency + lockfile update, the `fmt`/
   `fmt:check` npm scripts, `src/test/oxfmt.test.ts` (contract test), the
   CI step, and the dependabot group update. Zero existing files are
   reformatted in this commit — `npm run fmt:check` still fails here
   (expected: the tree isn't Oxfmt-conformant yet).

2. **`style(vscode-extension): apply Oxfmt formatting`** — `oxfmt --write .`
   applied, committed with nothing else mixed in. This commit's diff should
   read as pure whitespace/quote/wrap normalization, nothing else.

### Formatting-only proof for commit 2

- **All 6 touched JSON/JSONC files** (`gherkin-language-configuration.json`,
  `language-configuration.json`, `snippets/launch.json`, `snippets/perl.json`,
  `tsconfig.json`, `tsconfig.test.json`) were parsed before (`git show
HEAD~1:<path>`) and after with a JSONC-tolerant parser
  (`jsonc-parser`, already a transitive devDependency) and deep-compared:

  ```
  gherkin-language-configuration.json -> semantically identical: true
  language-configuration.json         -> semantically identical: true
  snippets/launch.json                -> semantically identical: true
  snippets/perl.json                  -> semantically identical: true
  tsconfig.json                       -> semantically identical: true
  tsconfig.test.json                  -> semantically identical: true
  ```

  Every touched value, key, and array element is identical — only
  whitespace/line-wrap changed.

- **`.ts`/`.js` source files**: Oxfmt (like Oxlint) is built on the `oxc`
  parser — a pretty-printer is definitionally AST-preserving. As a
  behavioral cross-check (not just a definitional argument), the full
  verification suite was re-run from a clean `npm ci` after both commits:

  ```
  npm run fmt:check   -> "All matched files use the correct format.", exit 0
  npm run lint        -> canary PASS + oxlint --type-aware clean, exit 0
  npm run compile     -> tsc -p ./ (still 6.0.3), exit 0
  npm run test:ci     -> Test Suites: 1 skipped, 24 passed, 24 of 25 total
                          Tests: 1 skipped, 653 passed, 654 total
                          (was 645/1 skip before oxfmt.test.ts's +8 tests)
  npm run test:grammar -> 9/9 fixtures pass, exit 0
  ```

  Identical pass/fail outcome before and after the formatting commit
  (module the +8 new tests contributed by the tooling commit's own
  `oxfmt.test.ts`, which is not part of the formatting diff).

- Most of the diff is **indentation normalization**: several source files
  mixed 2-space and 4-space indentation _within the same file_
  (pre-existing inconsistency — e.g. `src/downloader.ts`'s `interface`
  declarations used 4-space while its function bodies used 2-space) and
  are now uniformly 2-space, matching both the dominant existing
  convention and Oxfmt's default `tabWidth: 2`. This is inherent,
  unavoidable churn for a first-time formatter adoption on an
  inconsistently-hand-formatted tree — the config choices above minimize
  _avoidable_ churn (quote style, package.json layout) but cannot make an
  internally-inconsistent tree format to a literal no-op.

## 5. VSIX does not ship Oxfmt's native binary

```
$ npx @vscode/vsce ls | grep -iE "oxfmt|oxlint"
.oxlintrc.json
.oxfmtrc.json
```

`vsce ls` (the packaging tool's own dry-run manifest) lists only the two
tiny `.rc.json` config files — no `node_modules/oxfmt/**`,
`node_modules/@oxfmt/**`, or any native binary. `vsce` includes
`node_modules/adm-zip/**` and `node_modules/tar/**` (both real runtime
`dependencies`) but correctly excludes `devDependencies`-only packages like
`oxfmt`, `oxlint`, `oxlint-tsgolint`, and `typescript`. Total packaged file
count: 371 (was 370 in the PREP-1 baseline + 1 for `.oxfmtrc.json`;
`.oxlintrc.json` was already counted from PREP-2). The two `.rc.json` files
shipping is consistent with existing PREP-2 precedent and is harmless (not
code, not a binary, doesn't affect runtime) — not something introduced or
newly decided by this PR.

## 6. CI wiring

`.github/workflows/ux-regression-gate.yml`'s existing `extension-jest` job
(same `ubuntu-24.04` runner, no new job, no Windows/non-Ubuntu runner
added) gets one new blocking step, `Format check (oxfmt)`, running
`npm run fmt:check`, placed after `Lint` and before `Typecheck`.

`.github/dependabot.yml`'s existing `typescript` dependency group (already
covering `typescript`, `oxlint`, `oxlint-tsgolint` from PREP-2) now also
covers `oxfmt`.

## Scope boundary

This PR does not: bump `typescript`, touch `tsconfig*.json`'s compiler
options (only whitespace, per section 4's semantic-equality proof), or
change the Jest pipeline (PREP-1) or Oxlint config (PREP-2, both already
merged). No Prettier or Prettier-compatible package was introduced anywhere
in this PR (verified via `oxfmt.test.ts`'s explicit negative-dependency
check). The TS7 compiler swap (bump `typescript` 6 -> 7, stable CLI, no
alias/override, remove `ignoreDeprecations: "6.0"`) is the next PREP in the
train; Rolldown after that.
