# TextMate Grammar Visual Regression Tests

This directory contains **visual regression tests** for the Perl TextMate
grammar (`../../syntaxes/perl.tmLanguage.json`). They lock down exactly which
scopes the grammar assigns to every token in a set of representative Perl
fixtures, so any unintended change to syntax highlighting surfaces as an
explicit diff in code review — the same philosophy as the Rust `insta`
snapshots described in [`docs/reference/SNAPSHOT_TESTING.md`](../../../docs/reference/SNAPSHOT_TESTING.md).

## How it works

We use [`vscode-tmgrammar-test`](https://github.com/PanAeon/vscode-tmgrammar-test)
in **snapshot mode** (`vscode-tmgrammar-snap`). The tool tokenizes each fixture
with the exact same `vscode-textmate` + `vscode-oniguruma` engine VS Code uses,
then records the scope of every character span in a `.snap` file checked in next
to the fixture.

The grammar and its scope (`source.perl`) are resolved from the extension's own
`package.json` `contributes.grammars`/`contributes.languages` declarations
(`--config package.json`), so the tests exercise the precise grammar-to-language
mapping that ships to users. No network, display, or WASM download is required —
the engine runs fully offline.

## Layout

```
fixtures/
  comments.pl          # comment.line scopes
  variables.pl         # scalars, arrays, hashes, slices, special vars
  strings.pl           # single/double quotes, q/qq/qw, escapes
  numbers.pl           # int, float, hex, octal, binary, underscored, exponent
  keywords_control.pl  # use/package/sub/my/if/foreach/while keywords
  operators.pl         # arithmetic, comparison, logical, ternary, range
  functions.pl         # builtin function scopes
  regex.pl             # m//, s///, tr///, qr// and modifiers
  pod.pl               # =pod ... =cut documentation blocks
  *.pl.snap            # generated scope snapshots (committed)
```

Each fixture targets a distinct entry in the grammar's pattern repository
(`comments`, `pod`, `strings`, `interpolation`, `numbers`, `variables`,
`keywords`, `operators`, `functions`, `regex`). The grammar also contains a
`swig` repository key for SWIG interface-file keywords (`keyword.other.perl.swig`);
it is intentionally not covered here because SWIG `.i` files are rarely mixed
with Perl test content, and the scope uses a distinct `.swig` suffix that
isolates it from scope renames affecting the main Perl keyword rules.

## Running

```bash
cd vscode-extension

# Verify the grammar still produces the recorded scopes (used in CI):
npm run test:grammar

# Regenerate snapshots after an intentional grammar change, then review the diff:
npm run test:grammar:update
```

A regression produces a non-zero exit and a per-line diff showing the old scope
vs the new one. CI runs `npm run test:grammar` in the **Extension Jest** job of
the [UX Regression Gate](../../../.github/workflows/ux-regression-gate.yml)
on every PR that touches `vscode-extension/**`.

## Updating snapshots

When you intentionally change `syntaxes/perl.tmLanguage.json`:

1. Run `npm run test:grammar:update`.
2. Inspect the `git diff` of the `.snap` files — every changed scope is visible.
3. Confirm the new scopes are correct, then commit the fixtures and snapshots
   together with the grammar change.

To extend coverage, add a new `.pl` fixture under `fixtures/` and run
`npm run test:grammar:update` to generate its snapshot.

## Known grammar bugs captured by the current baseline

These snapshots record what the grammar **actually** emits today, including
several pre-existing highlighting bugs. That is intentional: a regression
baseline captures current behaviour so that *any* change — including a fix —
shows up as an explicit `.snap` diff. When the grammar is corrected, regenerate
the affected snapshots (`npm run test:grammar:update`) and the diff will show the
scope improving.

The `.snap` files themselves are the complete record of current behaviour; the
list below is **representative, not exhaustive**. The full catalogue of known
defects (and the grammar-fix work) is tracked in
[#1958](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1958). Examples:

- **`qq{...}` / `qw(...)`** (`strings.pl`): the brace/`qw` begin rule scopes
  leading tokens (`my $qq = `) as `string.quoted.q.perl`; the paren form
  `q(...)` is correct.
- **`=~ /pattern/`** (`operators.pl`): the bare match regex after `=~` is
  tokenized as arithmetic division (`/` → `keyword.operator.arithmetic.perl`);
  `m//`, `s///`, `tr///`, `qr//` are correct.
- **`length` / barewords** (`functions.pl`): the `le`/`gt` string-comparison
  operator patterns match *inside* identifiers, shredding `length` into
  `le` + `gt` scopes.
- **`<STDIN>`** (`keywords_control.pl`): the readline diamond is mis-scoped as
  `keyword.operator.comparison.perl` (the `<`/`>` comparison rule).
- **`->` / `=>`** (`variables.pl`): arrow and fat-comma are split by the
  comparison/operator rules rather than scoped as single operators.
- **`0o755`** (`numbers.pl`): modern octal prefix gets no
  `constant.numeric.octal.perl` scope.
- **`1_000_000`** (`numbers.pl`): underscore-separated integers get no
  `constant.numeric.integer.perl` scope (plain `42` is correct).

Fixing these belongs in a grammar PR against `syntaxes/perl.tmLanguage.json`,
not here — this harness is the guard that keeps them fixed once they are.

## Tooling note: accepted transitive dependency

`vscode-tmgrammar-test` (dev-only) transitively depends on `glob@^7`
(`glob@7.2.3`), which npm marks deprecated. This is **knowingly accepted**:

- It is a **dev/test** dependency, not shipped in the extension bundle
  (`dependencies` ships only `adm-zip`, `tar`, `vscode-languageclient`).
- `npm audit` reports **0 vulnerabilities** for the committed lockfile; the
  deprecation is a maintenance notice on the glob v7 line.
- A `package.json` `overrides` forcing glob v9/v10 is **not** applied: the tool
  is written against the glob v7 API and would break. The transitive resolves
  upward naturally when `vscode-tmgrammar-test` bumps its own dependency.
