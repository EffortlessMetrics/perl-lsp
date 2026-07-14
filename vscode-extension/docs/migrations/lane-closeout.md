# VS Code client/toolchain modernization lane closeout

Status: complete on `main`; this closeout is merged after the final runtime
slice and records the handoff boundary for the resulting history.

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
| #4121 | Node 26/npm 11 toolchain authority                   | Merged |
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
| #4070 | Routed Rust shell quoting repair                     | Merged |
| #4142 | Exact-source editor-host lifecycle contract          | Merged |
| #4144 | VSIX package inventory baseline refresh              | Merged |
| #4232 | Toolchain and extension-host receipt metadata        | Merged |
| #4247 | Published-smoke runtime metadata propagation         | Merged |

Each slice was refreshed from current `origin/main` when necessary and kept
to its owned production seam, direct proof, and required generated artifacts.
The rescue checkpoint was not merged wholesale.

## Current ownership and policy

- The lifecycle controller is authoritative for client start, restart, stop,
  generation invalidation, and disposal. Activation remains non-blocking.
- Configuration changes are classified as live, reconstruct, restart, or
  unrelated. Include paths and critic settings use canonical LSP payloads;
  deprecated critic settings remain compatibility inputs only.
- npm and `package-lock.json` are the package authority. Node 26.x and npm
  11.18.0 are the supported toolchain; CI pins Node 26.5.0. All TypeScript
  authority configurations and the doctor check are explicit.
- Oxlint warning debt is recorded by rule, surface, rule-by-surface, and file;
  new errors or warning growth are rejected rather than silently baselined.
- Workspace guidance and the `perllsp --health` process boundary have separate
  owners and focused tests.
- The exact-source VSIX/current-server harness is reused by hosted Linux smoke.
  Startup, initialization, provider, restart, and shutdown receipts include
  source identity, monotonic milestones, and separate toolchain versus
  extension-host runtime metadata. The published-smoke path now supplies the
  same toolchain metadata when it bypasses the integration runner.
- The package inventory and size ratchet protect the published artifact.
- Jest remains the test runner. Optional feature loading remains deferred until
  repeated receipts attribute measurable cold-path cost to a specific feature.

## Evidence boundary

The following checks and receipts are present on the current merged history:

```text
npm run fmt:check
npm run typecheck:all
npm run lint
npm test -- --runInBand --runTestsByPath src/test/downloader.test.ts
npm run test:ci
```

The focused downloader run passed 113 tests; the full CI suite passed 735
tests with one existing skip. The current-source Linux smoke passed on the
package-baseline refresh and exercised the exact-source editor-host contract:
activation, initialization, provider request, restart, shutdown, invalid-path
health guidance, source identity, and VSIX identity. The package inventory
check passed with 28 inventoried files and no allowlist violations. These
checks prove the exercised behavior and packaging paths; they do not establish
a performance budget or prove every platform-specific release condition.

PR #4042's required routed Rust result was merged through the maintainer
override path because both routed attempts failed in the unchanged
`.github/workflows/em-ci-routed-rust.yml` shell wrapper at the existing nested
`bash -c`/`awk` quoting, after the Rust checks themselves reached the scorecard
command. PR #4070 subsequently repaired that quoting in the workflow. The
downloader PR changed only its implementation and focused tests; the repair
was kept as a separate workflow PR.

## Remaining work and sync boundary

The modernization lane is complete and ready for a history-preserving sync.
Issue #4120 was closed by merged PR #4121. No publish, tag, release, or
`perl-lsp` synchronization has been performed. The target checkout is
available locally, and the next authorized operation is to pin and verify a
swarm cut before merging this logical PR/commit history into it without
replacing the trail with an opaque snapshot.
