# Implementation Checklist: Visual Regression Testing

Lock down the VS Code extension's TextMate syntax highlighting with scope
snapshots, wired into CI. Additive, non-Rust, scoped to `vscode-extension/**`
plus docs.

## Steps

- [x] **Add tooling.** `vscode-tmgrammar-test ^0.1.3` to `vscode-extension`
  devDependencies; sync `package-lock.json`. Verify: `grep vscode-tmgrammar-test package-lock.json`.
- [x] **Add fixtures.** `test/grammar/fixtures/*.pl` covering comments, variables,
  strings, numbers, keywords/control flow, operators, functions, regex, pod.
- [x] **Generate snapshots.** `npm run test:grammar:update` → committed
  `*.pl.snap`. Verify: `npm run test:grammar` exits 0.
- [x] **Add scripts.** `test:grammar` (verify) and `test:grammar:update` (regen)
  in `package.json`, using `--config package.json` for scope resolution.
- [x] **Prove regression detection.** Perturb a grammar scope → suite exits
  non-zero with a diff; revert → exits 0.
- [x] **Wire CI.** Add a `Grammar snapshot tests` step to the `extension-jest`
  job in `ux-regression-gate.yml`; make `ci/extension-jest` status reflect it.
- [x] **Docs.** `test/grammar/README.md`, section in
  `docs/reference/SNAPSHOT_TESTING.md`, check the E2E strategy box, CHANGELOG
  "Under the hood" entry.
- [x] **Verify no collateral.** `npm run compile`, `npm run lint`, `npm audit`
  all clean; jest suite unaffected (fixtures outside `src/`).

## Verify (full)

```bash
cd vscode-extension
npm ci
npm run lint && npm run compile && npm run test:ci && npm run test:grammar
```
