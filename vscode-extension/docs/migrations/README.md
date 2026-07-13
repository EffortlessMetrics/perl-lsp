# VS Code toolchain migration index

Status: the TypeScript 6 → 7 migration train is complete. The receipt files in
this directory preserve the evidence captured at each historical commit; their
old versions, package counts, and “next PREP” language are as-of statements,
not the current toolchain contract.

## Completed train

| Concern                              | Historical receipt                                                             | Current authority                                                               |
| ------------------------------------ | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------- |
| Jest decoupled from the compiler API | [`ts7-prep-1-jest-decouple-receipts.md`](ts7-prep-1-jest-decouple-receipts.md) | [`jest-vs-vitest-decision.md`](jest-vs-vitest-decision.md) and `jest.config.js` |
| ESLint replaced by Oxlint            | [`ts7-prep-2-oxlint-receipts.md`](ts7-prep-2-oxlint-receipts.md)               | `package.json`, `.oxlintrc.json`, and the committed warning baseline            |
| Oxfmt adopted as formatter           | [`ts7-prep-3-oxfmt-receipts.md`](ts7-prep-3-oxfmt-receipts.md)                 | `.oxfmtrc.json` and `fmt:check`                                                 |
| TypeScript compiler swapped to 7     | [`ts7-compiler-swap-receipts.md`](ts7-compiler-swap-receipts.md)               | `package.json`, `package-lock.json`, and the five TypeScript authority configs  |
| Rolldown production bundle adopted   | [`ts7-rolldown-bundle-receipts.md`](ts7-rolldown-bundle-receipts.md)           | `rolldown.config.mjs`, `compile`, and the VSIX inventory gate                   |

## Current follow-through

The active development contract is maintained in
[`../../DEVELOPMENT.md`](../../DEVELOPMENT.md). It now includes:

- npm `10.8.2` and the supported Node floor as enforceable doctor checks;
- source, test, integration, published-smoke, and script TypeScript checks;
- non-growing `noUncheckedIndexedAccess` and `exactOptionalPropertyTypes`
  baselines, with `noImplicitOverride` blocking and clean;
- type-aware Oxlint canary and warning-budget inventory;
- exact-source VSIX/current-server smoke, repeated startup/request receipts,
  and package inventory/size ratchets; and
- the current decisions to retain Jest and defer optional-feature loading until
  feature-attributable receipts exist.

The historical receipts remain useful for provenance and migration rationale.
For commands, versions, package contents, and current acceptance criteria, use
the current authority links above rather than copying measurements from an old
receipt into a new claim.
