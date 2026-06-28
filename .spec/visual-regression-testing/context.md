# Context: Visual Regression Testing for Syntax Highlighting

## Origin

`crates/perl-parser/tests/E2E_TEST_STRATEGY.md` listed an unchecked planned
improvement: *"Visual regression testing for UI features."* No issue, PR, or
prior implementation existed. This spec records the chosen interpretation and the
implementation that closes it.

## Problem

The VS Code extension's static syntax highlighting is produced by the TextMate
grammar `vscode-extension/syntaxes/perl.tmLanguage.json` (310 lines, repository
keys: comments, pod, strings, interpolation, numbers, variables, keywords,
operators, functions, regex, swig). Before this change, **nothing** tested that
grammar. A scope rename, a broken regex, or a reordered pattern could silently
break highlighting for users, and no CI gate would notice — the only signal would
be a user bug report against a published extension.

The Rust side already has strong snapshot discipline (`insta` for AST and LSP
capabilities — see `docs/reference/SNAPSHOT_TESTING.md`). The grammar had no
equivalent.

## Scope decision

"Visual regression testing" has several possible meanings. The chosen surface is
**TextMate grammar scope snapshots** (confirmed with the maintainer):

- Deterministic, offline, CI-friendly — matches the repo's existing snapshot
  philosophy.
- Tests the exact artifact that drives highlighting in the editor.

Explicitly **out of scope** (separate future work):

- LSP semantic-token snapshots (the half-baked `xtask` `test_syntax_highlighting`
  stub remains untouched to avoid scope drift).
- Screenshot / pixel-diff visual testing (too heavy and flaky for this repo).

## Tooling

[`vscode-tmgrammar-test`](https://github.com/PanAeon/vscode-tmgrammar-test)
`^0.1.3`, snapshot mode (`vscode-tmgrammar-snap`). It uses the same
`vscode-textmate` + `vscode-oniguruma` engine VS Code ships, runs fully offline
(no network/display/WASM download), and resolves the grammar + `source.perl`
scope from the extension's own `package.json` contributes via `--config
package.json`.

## CI integration

Wired into the existing `extension-jest` job of
`.github/workflows/ux-regression-gate.yml`, which already runs `npm ci`, lint,
typecheck, and jest on every PR touching `vscode-extension/**`. The new step runs
`npm run test:grammar`; the `ci/extension-jest` commit status reflects it.
