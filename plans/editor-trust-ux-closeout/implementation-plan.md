# Editor Trust UX Closeout Implementation Plan

Status: active
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Active goal: [active.toml](../../.perl-lsp/goals/active.toml)
Previous goal archive: [2026-05-18-real-perl-editor-trust.toml](../../.perl-lsp/goals/archive/2026-05-18-real-perl-editor-trust.toml)

## Current State

The original Real Perl Editor Trust control-plane lane is archived as complete.
Its proposal, initial specs, ADRs, generated parser status, provider confidence
matrix, support tiers, and real-workspace receipts remain the source of truth
for what can be claimed.

This plan owns the next active lane: turning that control plane into a
user-facing editor-trust product. It does not reopen broad parser bucket work
while generated parser status lists no current nonzero raw bucket, and it does
not promote provider behavior from docs or receipt-only PRs.

Current merged contract anchors:

- [PLSP-SPEC-0002](../../docs/specs/PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0003](../../docs/specs/PLSP-SPEC-0003-real-workspace-editor-baseline.md)
- [PLSP-SPEC-0005](../../docs/specs/PLSP-SPEC-0005-receiver-expression-facts.md)
- [PLSP-SPEC-0007](../../docs/specs/PLSP-SPEC-0007-receiver-fact-completion.md)
- [PLSP-SPEC-0008](../../docs/specs/PLSP-SPEC-0008-edit-producing-provider-safety.md)
- [PLSP-SPEC-0009](../../docs/specs/PLSP-SPEC-0009-workspace-trust-report.md)
- [PLSP-SPEC-0010](../../docs/specs/PLSP-SPEC-0010-support-claim-map.md)
- [PLSP-SPEC-0011](../../docs/specs/PLSP-SPEC-0011-trust-lane-ci-routing.md)
- [PLSP-SPEC-0012](../../docs/specs/PLSP-SPEC-0012-user-facing-trust-surfaces.md)
- [PLSP-ADR-0001](../../docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md)
- [PLSP-ADR-0002](../../docs/adr/PLSP-ADR-0002-confidence-before-cutover.md)
- [PLSP-ADR-0003](../../docs/adr/PLSP-ADR-0003-preview-before-edit.md)

Status owners:

- [Real Perl Editor Trust dashboard](../../docs/project/status/real_perl_editor_trust_v1.md)
- [support tiers](../../docs/project/status/SUPPORT_TIERS.md)
- [provider confidence matrix](../../docs/project/status/provider_confidence_matrix.md)
- [provider cutover](../../docs/project/status/provider_cutover.md)
- [semantic scorecard](../../docs/project/status/semantic_scorecard.md)
- [semantic shadow compare](../../docs/project/status/semantic_shadow_compare.md)
- [parser accuracy next](../../docs/project/status/parser_accuracy_next.md)
- [parser status](../../docs/project/status/parser.md)

## Work item: editor-trust-user-facing-contracts

Status: completed
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs: [PLSP-SPEC-0011](../../docs/specs/PLSP-SPEC-0011-trust-lane-ci-routing.md), [PLSP-SPEC-0012](../../docs/specs/PLSP-SPEC-0012-user-facing-trust-surfaces.md)
Blocks: user-facing trust docs, command documentation, setup troubleshooting
Blocked by: none

Goal

Encode the remaining user-facing trust contracts in durable specs so
agents do not need chat context to know what explanations, receipts, previews,
workspace trust, support claims, and CI routing may claim.

Production delta

Spec and ADR changes only. No provider behavior, support-tier promotion, parser
runtime change, workspace scan, DAP launch, perldoc execution, or CI routing
implementation unless the PR explicitly says it is a validator/policy PR.

Non-goals

No broad roadmap. No generated status content. No current PR queue ordering.
No behavior cutover from docs-only PRs.

Acceptance

Each spec states contract, non-goals, valid PR shapes, invalid PR shapes, proof
commands, claim boundaries, and status docs that own current evidence.

Proof commands

```bash
cargo xtask check-support-claims
cargo xtask check-provider-confidence-matrix
git diff --check
```

Rollback

Revert the affected spec PR. If a contract is wrong, leave the active
goal in place and mark the specific work item blocked until a replacement
contract lands.

## Work item: editor-trust-user-docs

Status: completed
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0012](../../docs/specs/PLSP-SPEC-0012-user-facing-trust-surfaces.md)
Blocks: README claim refresh, VS Code command docs
Blocked by: none
Receipt: [Editor Trust](../../docs/how-to/EDITOR_TRUST.md)
Supporting receipts:
[Perl Setup Troubleshooting](../../docs/how-to/PERL_SETUP_TROUBLESHOOTING.md),
[VS Code README](../../vscode-extension/README.md),
[Commands reference](../../docs/reference/COMMANDS_REFERENCE.md)

Goal

Add user-facing docs that explain measured Perl editor trust without requiring
users to read provider matrices or dashboards.

Production delta

Add or update docs for editor trust, setup troubleshooting, command discovery,
and bug-report receipts.

Non-goals

No support-tier promotion. No broad CPAN/static/refactor claims. No duplicated
support matrix.

Acceptance

Docs explain partial-live-with-fallback, fallback reasons, safe-edit previews,
dynamic Perl boundaries, `@INC`/module resolution, workspace trust report,
provider explanations, diagnostic explanations, and copyable receipts in plain
language while linking to status/support truth sources.

Current implementation status

The user-facing guide, setup troubleshooting guide, VS Code trust-claim docs,
and command reference now cover the trust and explanation surfaces. This work
item is documentation only; it does not promote support tiers or broaden
provider behavior.

Proof commands

```bash
cargo xtask check-support-claims
cargo xtask check-provider-confidence-matrix
git diff --check
```

Rollback

Revert the docs PR. If wording overclaims, narrow the docs before any support
claim changes.

## Work item: workspace-trust-schema-snapshots

Status: completed
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0009](../../docs/specs/PLSP-SPEC-0009-workspace-trust-report.md)
Blocks: setup troubleshooting docs, support-front-door docs
Receipt: `crates/perl-lsp-rs/tests/lsp_execute_command_tests.rs::test_execute_command_workspace_trust_report_schema_snapshot`

Goal

Lock the user-facing workspace trust report shape enough that setup support and
bug reports do not churn.

Production delta

Add focused snapshots or tests for the report schema and output-channel
presentation. The report remains read-only over existing server/client state.

Non-goals

No Perl probing. No perldoc execution. No DAP launch. No workspace scan. No
support-tier promotion.

Acceptance

Snapshots cover workspace roots, include path state, `PERL5LIB` policy, perldoc
contract state, DAP/perldoc runtime state when supplied, launch config
counts/classes, provider tiers, dynamic caveats, and copyable payload fields.

Proof commands

```bash
cargo test -p perl-lsp-rs --test lsp_execute_command_tests test_execute_command_workspace_trust_report --profile agent --locked -- --nocapture --test-threads=1
cargo xtask check-support-claims
git diff --check
```

Rollback

Revert snapshots or output changes. If the report shape is unstable, mark the
work item blocked and keep docs from promising a stable payload.

## Work item: receiver-fact-completion

Status: completed
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs: [PLSP-SPEC-0005](../../docs/specs/PLSP-SPEC-0005-receiver-expression-facts.md), [PLSP-SPEC-0007](../../docs/specs/PLSP-SPEC-0007-receiver-fact-completion.md)
Blocks: broader receiver-form completion expansion
Blocked by: none
Receipt: [receiver facts status](../../docs/project/status/receiver_facts.md)
Supporting receipts:
[support tiers](../../docs/project/status/SUPPORT_TIERS.md),
[provider cutover](../../docs/project/status/provider_cutover.md),
[provider confidence matrix](../../docs/project/status/provider_confidence_matrix.md),
`crates/perl-lsp-rs-core/src/providers/completion/completion/tests.rs`

Goal

Use receiver facts to make method completion useful for source-backed object and
package shapes while preserving fallback for unknown and dynamic receivers.

Production delta

First add receiver fact extraction or receipts. Enable a narrow live completion
pilot only after source-backed, fresh, high-confidence receiver facts and
fallback preservation are proven.

Non-goals

No completion behavior change from facts-only PRs. No generated/no-source
promotion. No dynamic hash key exactness. No suppression of legacy candidates
for unknown receivers.

Acceptance

Facts expose receiver kind, inferred package, shape fact, confidence, evidence,
freshness, dynamic boundary, source range, and fallback state. Completion
receipt proves source-backed `$self`/object/package/hashref known-slot ranking,
dynamic fallback, and unknown fallback before any pilot.

Current implementation status

Receiver facts, expression-fact substrate, completion ranking receipts, the
narrow source-backed receiver completion pilot, and the support review have
landed. Completion remains partial-live-with-fallback: only fresh,
high-confidence, source-backed receiver evidence may contribute exact method
candidates in the narrow pilot, while unknown, dynamic, generated/no-source,
stale, low-confidence, and broader workspace-wide method shapes remain
fallback, shadowed, or blocked.

Proof commands

```bash
cargo test -p perl-semantic-analyzer --lib receiver_fact --profile agent --locked -- --nocapture
cargo test -p perl-lsp-rs-core --lib completion --profile agent --locked -- --nocapture
cargo xtask check-support-claims
cargo xtask check-provider-confidence-matrix
git diff --check
```

Rollback

Revert the facts, receipt, or pilot PR. If fallback changes unexpectedly, revert
the pilot first and keep facts-only evidence for follow-up.

## Work item: edit-provider-safety-refresh

Status: planned
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0008](../../docs/specs/PLSP-SPEC-0008-edit-producing-provider-safety.md)
Blocks: rename/safe-delete support review
Blocked by: provider fact or workspace-index changes that require refreshed edit-safety proof

Trigger condition

Run this work item only when rename facts, safe-delete facts, workspace-index
behavior, or edit-producing provider code changes. Docs-only changes,
unrelated refactors, receiver-completion work, parser fixtures, workspace-trust
reporting, or CI routing do not require a new edit-safety receipt by default.

Goal

Keep rename and safe-delete false-allow, edit-freshness, blocker, and rollback
receipts current as facts and workspace indexing evolve.

Production delta

Receipt/test updates only unless a separate live-cutover PR satisfies the
edit-producing provider safety contract.

Non-goals

No broader package rename. No broader safe-delete. No generated, dynamic,
imported/exported, stale, low-confidence, ambiguous, non-source-backed,
non-subroutine, or package-wide edit authorization.

Acceptance

Generated/no-source, dynamic, referenced, imported/exported, stale,
low-confidence, ambiguous, non-subroutine, and package-wide cases return no
edit or fallback with explicit reasons and copyable receipts.

Current implementation status

Current rename and safe-delete receipts have already been reviewed in the
provider confidence matrix, support tiers, provider cutover notes, and Real Perl
Editor Trust dashboard. Rename remains limited to same-file lexical rename and
the narrow package-local pilot; safe-delete remains limited to the exact
unreferenced source-backed subroutine pilot with current-source, workspace
reference, workspace identity, and rollback guards. This work item stays planned
as a trigger-based maintenance lane, not as the next unconditional provider
cutover.

Proof commands

```bash
cargo test -p perl-lsp-rs --lib refactor_runtime_blocker_ux_safe_delete --profile agent --locked -- --nocapture --test-threads=1
cargo test -p perl-lsp-rs-core --lib safe_delete_shadow --profile agent --locked -- --nocapture
cargo xtask check-support-claims
cargo xtask check-provider-confidence-matrix
git diff --check
```

Rollback

Revert the receipt or behavior PR. If an edit-producing live path is unsafe,
revert live behavior before changing docs.

## Work item: trust-lane-ci-routing

Status: completed
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0011](../../docs/specs/PLSP-SPEC-0011-trust-lane-ci-routing.md)
Blocks: cheap parser fixture lane, hosted CI exposure estimate
Blocked by: none
Receipt: [PR Plan trust-lane summary](../../docs/ci/pr-plan.md)
Supporting receipts:
[trust-lane policy](../../policy/trust-lanes.toml),
[PR Plan workflow](../../.github/workflows/pr-plan.yml),
[PR Plan classifier](../../scripts/ci/pr_plan.py),
[trust-lane validator](../../scripts/ci/validate_trust_lanes.py)

Goal

Classify PRs by trust lane so CI proof follows the claim boundary instead of
defaulting to broad proof by habit.

Production delta

Add policy or PR summary support only after the routing contract lands. The
first implementation should be advisory and receipt-producing.

Non-goals

No broad full-CI default. No provider behavior proof from CI routing alone. No
support-tier promotion from routing alone.

Acceptance

PR summary or receipt names the trust-lane class, changed surface, required
proof, skipped-by-policy checks, hosted-CI estimate, and widening triggers.

Current implementation status

The advisory PR Plan summary reads `policy/trust-lanes.toml`, classifies
changed files, emits a `trust_lanes` block in `ci-plan.json`, and renders the
trust-lane class, required proof, skipped-by-policy checks, support claim
impact, and widening triggers in the step summary. This is receipt-producing CI
metadata only; it does not skip branch protection, prove provider behavior, or
promote support tiers.

Proof commands

```bash
python scripts/ci/validate_trust_lanes.py --strict
python scripts/ci/validate_risk_packs.py --strict
cargo xtask check-support-claims
git diff --check
```

Rollback

Revert the routing policy or summary PR. If classification is wrong, keep the
spec and disable only the classifier output until fixed.
