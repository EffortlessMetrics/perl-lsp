# PLSP-SPEC-0012: User-facing trust surfaces

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0008](PLSP-SPEC-0008-edit-producing-provider-safety.md)
- [PLSP-SPEC-0009](PLSP-SPEC-0009-workspace-trust-report.md)
- [PLSP-SPEC-0016](PLSP-SPEC-0016-provider-decision-receipt-v1.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
- [PLSP-ADR-0003](../adr/PLSP-ADR-0003-preview-before-edit.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: provider confidence matrix, provider cutover, support tiers,
workspace trust report, diagnostic explanations, VS Code command surfaces

## Current implementation status

This spec is accepted as the user-facing trust surface contract. Provider
decision explanations, copyable provider receipts, diagnostic explanations,
explain-diagnostic actions, missing-module lookup explanations, workspace trust
report output, and VS Code command-palette presentation now exist as bounded
trust surfaces.

Current evidence and public claim boundaries live in
[provider_confidence_matrix.md](../project/status/provider_confidence_matrix.md),
[provider_cutover.md](../project/status/provider_cutover.md),
[SUPPORT_TIERS.md](../project/status/SUPPORT_TIERS.md), and
[real_perl_editor_trust_v1.md](../project/status/real_perl_editor_trust_v1.md).
Future explanation or command-surface PRs must preserve this spec's rule that
presentation explains existing evidence without creating facts or broadening
provider behavior.

## Contract

User-facing trust surfaces explain existing evidence. They must not create new
facts, broaden support tiers, scan the workspace, run `perldoc`, launch DAP,
probe Perl, or promote provider behavior.

These surfaces translate provider receipts, diagnostics, module-resolution
facts, workspace trust state, and preview/no-edit decisions into plain language
and copyable structured payloads. The structured payload remains canonical.
`user_message` is additive presentation text and must not be the source of
truth for provider, diagnostic, setup, or edit-safety decisions.

This spec governs:

- `perl.explainProviderDecision`
- copyable provider receipts
- diagnostic explanation payloads
- explain-diagnostic code actions
- `perl.explainMissingModuleLookup`
- `perl.workspaceTrustReport`
- VS Code output-channel presentation for trust and explanation commands

## Required Schema Fields

Structured trust payloads must include these fields when applicable, or an
explicit unavailable/unknown state when the evidence does not exist:

```text
schema_version
provider / surface
decision
reason
fact_source
confidence
freshness
fallback
dynamic_boundary
source_backed
request position when supplied
support-tier link
redacted workspace-root class/hash
user_message
copyable_payload
```

Field names may differ between Rust structs and JSON wire shapes when existing
API compatibility requires it, but snapshots and docs must preserve the same
user-facing meaning.

## Canonical Payload Rules

The structured payload is canonical. Presentation layers may summarize it, but
must not add stronger claims than the payload supports.

Acceptance rules:

- `user_message` is additive, not source of truth.
- `copyable_payload` preserves the structured decision boundary.
- local copy/export is user initiated only; no telemetry or automatic upload.
- unknown providers return conservative low-confidence fallback payloads.
- caller-supplied `request_receipt` takes precedence over reconstructed state.
- snapshots lock user-facing schema for commands and copyable reports.
- output-channel text and JSON payloads agree on decision, fallback, and
  blocker state.
- sensitive paths use redacted workspace-root class, count, or hash where raw
  paths are not required for the support question.

## Surface Requirements

### Provider Decision Explanations

Provider decision explanations must explain why the provider acted, fell back,
blocked, or deferred. They must preserve the provider confidence receipt fields
defined by [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md).
The stable v1 receipt shape is defined by
[PLSP-SPEC-0016](PLSP-SPEC-0016-provider-decision-receipt-v1.md) and
[schemas/provider_decision.v1.schema.json](../../schemas/provider_decision.v1.schema.json).

If the request supplies a receipt, that receipt is the source of truth. If no
receipt is available, the command may return a conservative fallback
explanation, but it must not synthesize high-confidence facts.

### Diagnostic Explanations

Diagnostic explanations must connect diagnostics to the evidence that kept,
suppressed, or contextualized them.

For missing-module diagnostics, explanations should expose module name,
expected relative path, include path policy, `PERL5LIB` policy, candidate
visibility, and the workspace trust report pointer when available.

For scope or semantic diagnostics, explanations should expose confidence,
freshness, dynamic-boundary state, and why conservative behavior remains.

### Missing-Module Lookup

Missing-module lookup explanations must describe the module-resolution evidence
already available to the server. They must not run Perl, scan the workspace, or
change resolver behavior while explaining the lookup.

### Workspace Trust Report

The workspace trust report requirements are defined by
[PLSP-SPEC-0009](PLSP-SPEC-0009-workspace-trust-report.md). User-facing trust
surfaces may link to or render report state, but they must preserve the
report-only boundary: no probes, scans, DAP launch, `perldoc` execution, or
support-tier promotion.

### VS Code Presentation

VS Code command-palette and code-action surfaces may render payloads in the
Perl LSP output channel and offer copy commands. They must keep the structured
payload copyable, avoid telemetry, and avoid webview-only state that cannot be
pasted into an issue.

## Valid PR Shapes

Valid PRs under this spec include:

- schema snapshot PRs for provider, diagnostic, missing-module, or trust report
  explanation payloads
- output-channel rendering PRs that present existing payloads without changing
  provider behavior
- command wiring PRs that expose explanation/copy actions for existing payloads
- docs PRs that explain how to paste receipts into issues
- validator PRs that enforce schema fields, claim language, or link integrity
- fallback-message PRs that make unknown or low-confidence states clearer

Every PR must state whether it changes schema, rendering, command wiring, docs,
or validation.

## Invalid PR Shapes

Invalid PRs include:

- changing completion, goto, hover, references, diagnostics, rename,
  safe-delete, module resolution, DAP, or workspace-index behavior from an
  explanation-only PR
- promoting support tiers from explanation payloads alone
- running Perl, `perldoc`, DAP, or workspace scans while explaining a decision
- replacing structured payloads with prose-only messages
- treating `user_message` as canonical decision state
- exposing raw sensitive paths or environment values in copyable payloads
- adding telemetry or automatic report upload
- converting stale, low-confidence, generated, or dynamic facts into exact
  source-backed claims through presentation text
- using current dashboard rows, generated parser counts, or one-off PR state as
  durable spec content

## Acceptance

A user-facing trust surface PR satisfies this spec when:

- the changed surface explains existing evidence only
- structured payloads include the required schema fields or explicit unknown
  states
- `user_message` and output-channel text agree with the canonical payload
- copyable payloads preserve provider, decision, fallback, support-tier, and
  blocker boundaries
- unknown, stale, generated, dynamic, low-confidence, or no-source states remain
  fallback, labeled, blocked, or deferred
- local copy is user initiated and no telemetry is introduced
- snapshots or focused tests cover the changed schema or command surface
- support-claim wording remains bounded by current status docs

## Proof Commands

Provider decision formatting and command proof:

```bash
cargo test -p perl-lsp-rs-core --lib provider_decision_format --profile agent --locked -- --nocapture
cargo test -p perl-lsp-rs --test lsp_execute_command_tests test_execute_command_explain_provider_decision --profile agent --locked -- --nocapture --test-threads=1
```

Missing-module and workspace-trust command proof:

```bash
cargo test -p perl-lsp-rs --test lsp_execute_command_tests test_execute_command_explain_missing_module_lookup --profile agent --locked -- --nocapture --test-threads=1
cargo test -p perl-lsp-rs --test lsp_execute_command_tests test_execute_command_workspace_trust_report --profile agent --locked -- --nocapture --test-threads=1
```

VS Code command-surface proof, when extension wiring changes:

```bash
npm --prefix vscode-extension test -- --runTestsByPath src/test/commands.test.ts src/test/extensionUx.test.ts
npm --prefix vscode-extension run compile
npm --prefix vscode-extension run lint
```

Support and docs proof:

```bash
cargo xtask check-support-claims
cargo xtask check-provider-confidence-matrix
git diff --check
```

Docs-only PRs for this spec may run the support and docs proof only, as long as
they do not change schema, command wiring, provider behavior, or VS Code code.

## Non-goals

- No provider behavior change.
- No support-tier promotion.
- No broad completion, navigation, diagnostic, rename, safe-delete, DAP, or
  module-resolution cutover.
- No workspace scans, Perl probes, `perldoc` execution, or DAP launch.
- No telemetry or automatic issue upload.
- No replacement for provider confidence receipts in
  [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md).
- No replacement for edit safety in
  [PLSP-SPEC-0008](PLSP-SPEC-0008-edit-producing-provider-safety.md).
- No replacement for workspace trust report boundaries in
  [PLSP-SPEC-0009](PLSP-SPEC-0009-workspace-trust-report.md).

## Claim Boundaries

Trust surfaces may claim that `perl-lsp` can explain the evidence it already
has. They may not claim the explanation command proves the underlying provider
is more capable than the current support tier says.

Provider explanations may claim act/fallback/block/defer decisions for the
specific request and receipt. They may not claim broad provider cutover.

Diagnostic and missing-module explanations may claim why the current diagnostic
or lookup behaved as it did. They may not claim setup health beyond existing
server/client state.

Copyable receipts may claim the payload is suitable for issue triage. They may
not claim private paths, environment, or support context are complete unless the
payload explicitly contains that state.

## Current Evidence Owners

Current state and evidence live outside this spec:

- [Provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [Provider cutover](../project/status/provider_cutover.md)
- [Support tiers](../project/status/SUPPORT_TIERS.md)
- [Real Perl Editor Trust dashboard](../project/status/real_perl_editor_trust_v1.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)
- [Semantic scorecard](../project/status/semantic_scorecard.md)
- [Semantic shadow compare](../project/status/semantic_shadow_compare.md)
- [Editor trust user guide](../how-to/EDITOR_TRUST.md)
- [Perl setup troubleshooting](../how-to/PERL_SETUP_TROUBLESHOOTING.md)
- [Command reference](../reference/COMMANDS_REFERENCE.md)
- [VS Code extension README](../../vscode-extension/README.md)

Do not copy current receipt rows, generated parser state, active PR order,
branch names, dashboard routing, or one-off CI failures into this spec.
