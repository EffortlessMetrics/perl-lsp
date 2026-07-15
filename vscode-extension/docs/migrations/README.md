# VS Code toolchain migration index

Status: the TypeScript 6 → 7 migration train is complete. The receipt files in
this directory preserve the evidence captured at each historical commit; their
old versions, package counts, and “next PREP” language are as-of statements,
not the current toolchain contract.

For the complete client/toolchain modernization lane, merged PR trail, and
bounded sync handoff, see [`lane-closeout.md`](lane-closeout.md).

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

- Node 26.x, npm `11.18.0`, and the CI pin Node `26.5.0` as enforceable doctor
  and workflow authority;
- source, test, integration, published-smoke, and script TypeScript checks;
- blocking `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, and
  `noImplicitOverride` checks across every TypeScript authority configuration;
- type-aware Oxlint canary and warning-budget inventory;
- exact-source VSIX/current-server smoke, repeated startup/request receipts,
  and package inventory/size ratchets; and
- exact `@vscode/vsce` 3.9.2 and `ovsx` 1.0.2 publisher tooling resolved
  offline from `package-lock.json` through the shared workflow setup action; and
- the exact-source Linux host matrix requests the declared VS Code 1.125.0
  floor and current stable, with receipts recording requested and actual host
  versions separately; and
- the workspace capability classifier records topology, trust, URI schemes,
  host kind, capability status, and limitations in the
  [workspace capability matrix](workspace-capability-matrix.md); and
- the current decisions to retain Jest and defer optional-feature loading until
  feature-attributable receipts exist.

The historical receipts remain useful for provenance and migration rationale.
For commands, versions, package contents, and current acceptance criteria, use
the current authority links above rather than copying measurements from an old
receipt into a new claim.
