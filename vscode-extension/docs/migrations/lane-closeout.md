# VS Code client/toolchain modernization lane closeout

Status: complete on `main` at `fbc5a95738151a7e645a80548e4d84d4122607f6`.

This closeout records the reviewable PR trail and the evidence boundary for
the modernization lane. It does not publish, tag, release, or synchronize
this repository into `perl-lsp`.

## Delivered PR trail

| PR    | Concern                                              | Result |
| ----- | ---------------------------------------------------- | ------ |
| #4043 | Pure language-client lifecycle controller            | Merged |
| #4054 | Extension lifecycle integration                      | Merged |
| #4060 | Configuration routing and synchronization            | Merged |
| #4065 | Node, npm, and TypeScript authority                  | Merged |
| #4068 | Oxlint warning inventory and ratchet                 | Merged |
| #4071 | Workspace guidance ownership                         | Merged |
| #4072 | Language-server health boundary                      | Merged |
| #4086 | Startup milestone receipts                           | Merged |
| #4124 | Declaration compatibility and `skipLibCheck` removal | Merged |
| #4125 | `noUncheckedIndexedAccess` advisory baseline         | Merged |
| #4128 | Blocking `noImplicitOverride` check                  | Merged |
| #4129 | `exactOptionalPropertyTypes` advisory baseline       | Merged |
| #4132 | Repeated exact-source startup/request sampling       | Merged |
| #4133 | VSIX inventory and size ratchet                      | Merged |
| #4134 | Jest versus Vitest evidence decision                 | Merged |
| #4136 | Optional-feature deferral decision                   | Merged |
| #4138 | Active documentation and changelog reconciliation    | Merged |
| #4042 | Downloader stream lifecycle race                     | Merged |

Each slice was refreshed from current `origin/main` when necessary and kept
to its owned production seam, direct proof, and required generated artifacts.
The rescue checkpoint was not merged wholesale.

## Current ownership and policy

- The lifecycle controller is authoritative for client start, restart, stop,
  generation invalidation, and disposal. Activation remains non-blocking.
- Configuration changes are classified as live, reconstruct, restart, or
  unrelated. Include paths and critic settings use canonical LSP payloads;
  deprecated critic settings remain compatibility inputs only.
- npm and `package-lock.json` are the package authority. The supported Node
  floor, npm version, all TypeScript authority configurations, and the doctor
  check are explicit.
- Oxlint warning debt is recorded by rule, surface, rule-by-surface, and file;
  new errors or warning growth are rejected rather than silently baselined.
- Workspace guidance and the `perllsp --health` process boundary have separate
  owners and focused tests.
- The exact-source VSIX/current-server harness is reused by hosted Linux smoke.
  Startup, initialization, provider, restart, and shutdown receipts include
  source identity and monotonic milestones.
- The package inventory and size ratchet protect the published artifact.
- Jest remains the test runner. Optional feature loading remains deferred until
  repeated receipts attribute measurable cold-path cost to a specific feature.

## Evidence boundary

The following extension checks passed on the merged downloader head before
merge:

```text
npm run fmt:check
npm run typecheck:all
npm run lint
npm test -- --runInBand --runTestsByPath src/test/downloader.test.ts
npm run test:ci
```

The focused downloader run passed 113 tests; the full CI suite passed 735
tests with one existing skip. The hosted current-source Linux smoke and the
exact-source VSIX smoke are part of the merged harness history. These checks
prove the exercised behavior and packaging paths; they do not establish a
performance budget or prove every platform-specific release condition.

PR #4042's required routed Rust result was merged through the maintainer
override path. Both routed attempts failed in the unchanged
`.github/workflows/em-ci-routed-rust.yml` shell wrapper at the existing nested
`bash -c`/`awk` quoting, after the Rust checks themselves reached the scorecard
command. The PR changed only the downloader implementation and its focused
tests; no unrelated Rust or workflow files were modified.

## Remaining work and sync boundary

The modernization lane is complete and ready for a history-preserving sync.
The following open work is intentionally outside this lane and was not closed
or modified:

- PR #4121: separate Node 26 toolchain policy proposal.
- PR #4080: Rust startup-readiness receipts.
- PR #4073: Dependabot `tar` update.

No publish, tag, release, or `perl-lsp` synchronization has been performed.
The target `perl-lsp` checkout is not present in this workspace, so the next
authorized operation is to obtain the target checkout and merge this logical
PR/commit history into it without replacing the trail with an opaque snapshot.
