# VS Code client/toolchain modernization lane closeout

Status: implementation hardening and history-preserving cross-repository sync
are complete on `main`. The earlier synchronized code cut is recorded in
target PR [#10001](https://github.com/EffortlessMetrics/perl-lsp/pull/10001),
and the final hardening cut is ready for the next target ledger entry.

This closeout records the reviewable PR trail, the evidence boundary, and the
completed handoff for the modernization lane. It does not publish, tag, or
release either repository.

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
| #4125 | `noUncheckedIndexedAccess` cleanup baseline          | Merged |
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
| #4249 | Runtime metadata closeout reconciliation             | Merged |
| #4250 | Closeout formatting correction                       | Merged |
| #4265 | Workspace guidance positive-path coverage            | Merged |
| #4266 | Lifecycle failure-edge coverage                      | Merged |
| #4270 | Blocking `exactOptionalPropertyTypes` contract       | Merged |
| #4271 | Strictness hardening closeout update                 | Merged |
| #4272 | Downloader indexed-access hardening                  | Merged |
| #4274 | Gherkin indexed-access hardening                     | Merged |
| #4275 | POD preview indexed-access hardening                 | Merged |
| #4276 | Test Adapter indexed-access hardening                | Merged |
| #4278 | Debug Adapter indexed-access hardening               | Merged |
| #4279 | Extension entrypoint indexed-access hardening        | Merged |
| #4280 | Utility indexed-access hardening                     | Merged |
| #4281 | Lifecycle test indexed-access hardening              | Merged |
| #4282 | Packaged smoke indexed-access hardening              | Merged |
| #4284 | Blocking `noUncheckedIndexedAccess` promotion        | Merged |
| #4285 | Complete Oxlint inventory enforcement                | Merged |
| #4291 | Typed command manifest contract tests                | Merged |
| #4293 | Script output reporter seam                          | Merged |
| #4294 | Typed arrow-completion tests                         | Merged |
| #4296 | Typed walkthrough and What's New contract tests      | Merged |
| #4298 | Typed small client test seams                        | Merged |
| #4299 | Typed formatting-error test fixture                  | Merged |
| #4301 | Typed onboarding test seams                          | Merged |
| #4302 | Typed POD preview contract                           | Merged |
| #4333 | VS Code host-floor compatibility proof               | Merged |
| #4335 | Workspace topology capability receipts               | Merged |
| #4375 | Workspace capability host-mode proof                 | Merged |
| #4336 | Bundle source-map evidence archive                   | Merged |
| #4338 | Feature activation timing attribution                | Merged |
| #4339 | Server command composition                           | Merged |
| #4341 | Critic command composition                           | Merged |
| #4349 | Test command composition                             | Merged |
| #4352 | Document feature composition                         | Merged |
| #4354 | Onboarding command composition                       | Merged |
| #4356 | Navigation command composition                       | Merged |
| #4358 | Centralized VS Code toolchain setup                  | Merged |
| #4362 | Provider-diagnostics command composition             | Merged |
| #4363 | Document command composition                         | Merged |
| #4365 | Refactoring command composition                      | Merged |
| #4366 | Final VSIX inventory baseline refresh                | Merged |
| #4368 | Support/report issue command composition             | Merged |
| #4384 | Workspace runtime-state ownership                    | Merged |
| #4389 | Diagnostic command boundary and focused tests        | Merged |

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
  authority configurations and the doctor check are explicit. `exactOptionalPropertyTypes`,
  `noImplicitOverride`, and `noUncheckedIndexedAccess` are blocking across every
  TypeScript authority configuration.
- Oxlint warning debt is recorded and enforced by rule, surface, rule-by-surface,
  and file; new errors or warning growth are rejected rather than silently
  baselined. The current inventory is 0 errors and 0 warnings after the typed
  VS Code mock, command manifest, script reporter, arrow-completion,
  walkthrough, What's New, health-widget, test-at-cursor, streaming-completion,
  formatting-error, onboarding, and command-composition cleanups.
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

The focused downloader run passed 113 tests; the latest full extension gate on
the current hardening head passed 797 tests with one documented packaged-server
skip. The focused command-composition slices add isolated delegation and
disposal proof for the server, critic, test, onboarding, navigation,
diagnostic, document, refactoring, and support groups. The runtime-state and
diagnostic command boundaries add direct tests for injected lifecycle state,
output, unavailable-state messaging, and request failures. The final package
evidence contains 28 inventoried files, 1,474,277 bytes total, and a
1,316,660-byte bundle; the inventory and source-map checks pass. These checks
prove the exercised behavior and packaging paths; they do not establish a
performance budget or prove every platform-specific release condition.

The workspace capability proof also records exact-source Windows host receipts
for trusted multi-root workspaces (two folders) and genuinely untrusted
single-root workspaces (one folder), including initialization, provider,
restart, and shutdown. Receipts retain separate Git source identity and server
artifact SHA-256 fields.

PR #4042's required routed Rust result was merged through the maintainer
override path because both routed attempts failed in the unchanged
`.github/workflows/em-ci-routed-rust.yml` shell wrapper at the existing nested
`bash -c`/`awk` quoting, after the Rust checks themselves reached the scorecard
command. PR #4070 subsequently repaired that quoting in the workflow. The
downloader PR changed only its implementation and focused tests; the repair
was kept as a separate workflow PR.

## Handoff and release boundary

The reviewed implementation cut `bd3eb11b221e18e9914c326326ee1515620bfae2`
was pinned, tested, and synchronized into `perl-lsp` through target PR #10001.
Its two-parent target merge is `90d6fb5614841a621a5950e0f9b92044406320a8`,
with the target-owned `.claude/`, swarm-only cleanup scripts, and sync ledgers
as the only approved tree differences. The exact ancestry and tree audit are
recorded in `perl-lsp/docs/swarm/source-syncs/2026-07-15-workspace-capabilities-bd3eb11b2.md`.
The final target ledger update merged as target PR #10002.

The post-sync hardening slices #4384 and #4389 are merged on `main` and are
included in the final stable cut used for the next history-preserving target
sync. That cut retains the complete swarm commit trail; its target ledger will
record the exact SHA, two-parent merge, approved exclusions, and target-side
proofs.

Issue #4120 was closed by merged PR #4121; that historical issue state does not
change the current Node 26/npm 11 authority.

No publish, tag, or release operation has been performed. Historical receipts
retain the toolchain versions they actually tested; current development and
publishing authority is Node 26.x with npm 11.18.0.
