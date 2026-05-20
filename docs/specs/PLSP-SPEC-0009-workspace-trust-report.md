# PLSP-SPEC-0009: Workspace trust report

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0003](PLSP-SPEC-0003-real-workspace-editor-baseline.md)
- [PLSP-SPEC-0022](PLSP-SPEC-0022-module-path-authority.md)
- [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: support tiers, provider confidence matrix, real editor trust
dashboard, VS Code command documentation, setup troubleshooting docs
Schema: [workspace_trust_report.v1.schema.json](../../schemas/workspace_trust_report.v1.schema.json)

## Current implementation status

This spec is accepted as the workspace trust report contract. The current
report and VS Code output surface are implemented as read-only views over
existing server and client state, with schema coverage tracked from
`lsp_execute_command_tests` and support evidence owned by
[SUPPORT_TIERS.md](../project/status/SUPPORT_TIERS.md) and
[real_perl_editor_trust_v1.md](../project/status/real_perl_editor_trust_v1.md).

Current setup hints, perldoc/DAP state, launch-configuration classes, and
module-path boundaries remain report-only. The payload shape is locked by
`workspace_trust_report.v1`; neither the report nor the schema may probe Perl,
run `perldoc`, launch DAP, scan the workspace, or promote setup-health support
tiers.

## Contract

The workspace trust report aggregates existing server and client state. It must
not run probes, scan files, launch DAP, run `perldoc`, resolve Perl, or promote
support tiers.

The report is a user-facing setup and trust surface. It explains what the
server already knows about workspace roots, Perl configuration, include paths,
provider support tiers, dynamic-boundary policy, and client-supplied runtime
state. It is not a discovery command and must not create new facts while
rendering the report.

This spec governs:

- `perl.workspaceTrustReport`
- VS Code **Perl LSP: Show Workspace Trust Report**
- [Perl setup troubleshooting](../how-to/PERL_SETUP_TROUBLESHOOTING.md)
- output-channel text derived from the workspace trust report payload

## Required Report Fields

The structured report must include these fields or explicit unavailable states
when the current server/client context does not have them:

```text
workspace roots
Perl binary/config state when known
effective include paths / @INC state
PERL5LIB policy
perldoc oracle contract state
DAP client/runtime state when supplied
launch configuration counts/classes
provider support tiers
dynamic-boundary caveats
copyable payload
known limitations
```

Fields may be grouped or renamed in implementation as long as snapshots and
documentation preserve the same user-facing meaning.

## Report-Only Boundary

The report must stay read-only while it is being generated:

- no workspace scan
- no parser corpus refresh
- no file-system crawl beyond already-held workspace/config state
- no Perl binary probing
- no `perl -V` invocation
- no `perldoc` execution
- no DAP launch
- no debug-session inspection
- no module-resolution behavior change
- no support-tier promotion
- no telemetry

Setup hints must be derived from existing configuration, existing runtime
counts, known client-supplied fields, or already-held server state. A setup hint
may recommend another command or configuration change, but it must not perform
that action.

## Sensitive Data Rules

The report is intended for issue reports and support handoff, so it must avoid
unnecessary sensitive local data.

Acceptance rules:

- do not expose raw sensitive paths unless they are already normal repo/config
  values required to understand the setup
- use path class, count, root class, or hash where raw paths are not necessary
- never include secrets, tokens, private environment values, or production
  credentials
- caller-supplied client state must be sanitized to known fields before
  rendering
- output-channel text and JSON/copyable payload must agree on the same trust
  state
- no automatic telemetry or upload is allowed

## Valid PR Shapes

Valid PRs under this spec include:

- schema snapshot PRs that lock the workspace trust report payload shape
- output-channel rendering PRs that present the existing payload without adding
  probing
- setup-hint PRs derived only from existing configuration or client-supplied
  state
- docs PRs that explain what the report does and does not probe
- DAP/perldoc metadata PRs that report supplied state without launching DAP or
  running `perldoc`
- support-map PRs that review claim wording after proof already landed

Each PR must state whether it changes schema, rendering, docs, setup hints, or
support wording.

## Invalid PR Shapes

Invalid PRs include:

- running Perl, `perldoc`, DAP, or module probes while producing the report
- scanning the workspace to fill missing report fields
- treating DAP `includePaths` metadata as module-path authority without a
  separate behavior proof
- exposing raw unredacted client paths where class/count/hash is sufficient
- changing diagnostics, completion, hover, module resolution, or DAP behavior
  from a report-only PR
- promoting a support tier from setup hints or report rendering alone
- adding telemetry or automatic report upload
- using the report as parser bucket, corpus freshness, or provider cutover
  proof

## Acceptance

A workspace-trust-report PR satisfies this spec when:

- the report is generated from existing server/client state only
- unavailable state is explicit instead of silently omitted when user-facing
  trust would be ambiguous
- path and runtime state are sanitized according to the sensitive-data rules
- output-channel text and structured payload agree
- docs explain what is and is not probed
- provider tiers link to support-tier status rather than duplicating claim
  rows
- setup hints do not change resolver, DAP, perldoc, diagnostic, or provider
  behavior
- proof commands cover the changed surface

## Proof Commands

Report schema and command proof:

```bash
cargo test -p perl-lsp-rs --test lsp_execute_command_tests test_execute_command_workspace_trust_report --profile agent --locked -- --nocapture --test-threads=1
cargo test -p perl-lsp-rs --test lsp_execute_command_tests test_execute_command_workspace_trust_report_schema_snapshot --profile agent --locked -- --nocapture --test-threads=1
```

VS Code output-channel proof, when the extension surface changes:

```bash
npm --prefix vscode-extension test -- --runTestsByPath src/test/commands.test.ts src/test/extensionUx.test.ts
npm --prefix vscode-extension run compile
npm --prefix vscode-extension run lint
```

Support and claim proof:

```bash
cargo xtask check-support-claims
cargo xtask check-provider-confidence-matrix
git diff --check
```

Docs-only PRs for this spec may use:

```bash
cargo xtask check-support-claims
cargo xtask check-provider-confidence-matrix
git diff --check
```

## Non-goals

- No workspace scanning or probing.
- No Perl binary resolution change.
- No `@INC` resolver behavior change.
- No `perldoc` execution.
- No DAP launch or debug-session inspection.
- No support-tier promotion from report output alone.
- No provider cutover.
- No parser bucket or corpus freshness claim.
- No telemetry or automatic upload.

## Claim Boundaries

The workspace trust report may claim that it shows the current server/client
trust state for the active workspace. It may not claim setup health is proven
beyond the fields already supplied or already known by the server.

The report may expose DAP launch configuration counts, path classes, and
client-supplied runtime state. It may not claim native DAP module-path behavior
has been promoted unless a separate DAP behavior receipt proves that support
claim.

The report may expose perldoc configuration and oracle contract state. It may
not claim `perldoc` is available unless that state was already known outside the
report-generation path.

The report may link support tiers and provider receipts. It must not duplicate
their current rows or convert status wording into a stronger public claim.

## Current Evidence Owners

Current state and evidence live outside this spec:

- [Support tiers](../project/status/SUPPORT_TIERS.md)
- [Provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [Real Perl Editor Trust dashboard](../project/status/real_perl_editor_trust_v1.md)
- [Module resolution status](../project/status/module_resolution.md)
- [Command reference](../reference/COMMANDS_REFERENCE.md)
- [Editor trust user guide](../how-to/EDITOR_TRUST.md)
- [Perl setup troubleshooting](../how-to/PERL_SETUP_TROUBLESHOOTING.md)
- [VS Code extension README](../../vscode-extension/README.md)

Do not copy current report snapshots, dashboard rows, support claim rows,
generated parser state, active PR order, local branch names, or one-off CI
failures into this spec.
