# Real Perl Editor Trust Implementation Plan

Status: active
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0001](../../docs/specs/PLSP-SPEC-0001-parser-compatibility-bucket-closeout.md)
- [PLSP-SPEC-0002](../../docs/specs/PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0003](../../docs/specs/PLSP-SPEC-0003-real-workspace-editor-baseline.md)
- [PLSP-SPEC-0004](../../docs/specs/PLSP-SPEC-0004-corpus-receipt-freshness.md)
Linked ADRs:
- [PLSP-ADR-0001](../../docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md)
- [PLSP-ADR-0002](../../docs/adr/PLSP-ADR-0002-confidence-before-cutover.md)
Active goal: planned `.perl-lsp/goals/active.toml`

## Current State

- [parser accuracy next](../../docs/project/status/parser_accuracy_next.md)
  reports 0 active failure packets and no measurement gaps.
- Parser capability work routes through
  [parser raw failure buckets](../../docs/project/status/parser.md#raw-failure-buckets).
- Raw bucket counts are point-in-time corpus receipt data. Fixture-only PRs may
  lock source-backed shapes, but only fresh corpus receipts may claim bucket
  movement.
- Provider confidence work routes through
  [provider cutover](../../docs/project/status/provider_cutover.md),
  [semantic scorecard](../../docs/project/status/semantic_scorecard.md),
  [semantic shadow compare](../../docs/project/status/semantic_shadow_compare.md),
  and [UX capability dashboard](../../docs/project/status/ux_capability_dashboard.md).

## Work item: source-of-truth-scaffolding

Status: completed; PR #8801
Linked proposal: n/a
Linked spec: n/a
Linked ADR: n/a
Blocks: proposal, specs, ADRs, implementation plan, active goal manifest
Blocked by: none

Goal

Define where Real Perl Editor Trust artifacts live and what each layer owns.

Production delta

Added source-of-truth READMEs for proposals, specs, ADRs, plans, and goals.

Non-goals

No proposal, behavior spec, parser fixture, provider change, generated status
edit, or implementation plan content.

Acceptance

Layer ownership READMEs exist and link to current generated status sources.

Proof commands

```bash
git diff --check
```

Rollback

Revert PR #8801. This removes the source-of-truth scaffold and should also park
later plan/goal PRs until the scaffold is restored.

## Work item: real-perl-editor-trust-proposal

Status: completed; PR #8804
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: n/a
Linked ADR: n/a
Blocks: specs, ADRs, implementation plan, active goal manifest
Blocked by: source-of-truth-scaffolding

Goal

Record why Real Perl Editor Trust exists and what user trust means for parser,
provider, real-workspace, and control-plane work.

Production delta

Added the lane proposal and claim boundaries.

Non-goals

No behavior contract, PR sequence, parser fixture, provider cutover, or generated
status edit.

Acceptance

Proposal includes problem, users, success criteria, proposed shape, alternatives,
evidence plan, risks, non-goals, and exit criteria.

Proof commands

```bash
git diff --check
```

Rollback

Revert PR #8804. Specs and ADRs should be reviewed for orphaned proposal links.

## Work item: parser-bucket-closeout-spec

Status: completed; PR #8806
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0001](../../docs/specs/PLSP-SPEC-0001-parser-compatibility-bucket-closeout.md)
Linked ADR: [PLSP-ADR-0001](../../docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md)
Blocks: raw-bucket-fixture-lane, linux-corpus-refresh
Blocked by: real-perl-editor-trust-proposal

Goal

Define how `parser_accuracy_next.md` and `parser.md#raw-failure-buckets` route
parser capability lanes.

Production delta

Added the parser bucket closeout contract, valid/invalid PR shapes, acceptance,
proof commands, and claim boundaries.

Non-goals

No parser runtime change, corpus sweep, generated status edit, or provider
behavior.

Acceptance

Spec states that stale buckets route discovery only and fresh corpus receipts
are required for bucket-count claims.

Proof commands

```bash
git diff --check
```

Rollback

Revert PR #8806 and pause parser bucket closeout PRs until a replacement spec
lands.

## Work item: provider-confidence-receipts-spec

Status: completed; PR #8808
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0002](../../docs/specs/PLSP-SPEC-0002-provider-confidence-receipts.md)
Linked ADR: [PLSP-ADR-0002](../../docs/adr/PLSP-ADR-0002-confidence-before-cutover.md)
Blocks: provider-confidence-closeout, support-claim-refresh
Blocked by: real-perl-editor-trust-proposal

Goal

Define provider confidence, freshness, fallback, blocker, and live-comparison
receipt requirements before cutover.

Production delta

Added the provider confidence receipt contract and provider surface list.

Non-goals

No live provider cutover, parser bucket work, real-workspace baseline contract,
or support-tier map.

Acceptance

Spec covers completion, goto, hover, references, symbols, rename, safe delete,
diagnostics, semantic tokens, and DAP module paths.

Proof commands

```bash
git diff --check
```

Rollback

Revert PR #8808 and block provider cutover PRs that depend on its receipt
contract.

## Work item: real-workspace-baseline-spec

Status: completed; PR #8811
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0003](../../docs/specs/PLSP-SPEC-0003-real-workspace-editor-baseline.md)
Linked ADR: [PLSP-ADR-0002](../../docs/adr/PLSP-ADR-0002-confidence-before-cutover.md)
Blocks: real-workspace-baseline-run, provider-confidence-closeout
Blocked by: provider-confidence-receipts-spec

Goal

Define how at least one real CPAN-style workspace baseline bridges fixtures to
user-scale editor trust.

Production delta

Added the real-workspace baseline contract, first-baseline rule, provider
bridge, proof commands, and claim boundaries.

Non-goals

No baseline run, generated status edit, provider behavior change, or all-CPAN
claim.

Acceptance

Spec requires project/source provenance, host/toolchain context, cold start,
indexing, module resolution, provider metrics, confidence/freshness links, and
explicit deferrals.

Proof commands

```bash
git diff --check
```

Rollback

Revert PR #8811 and pause real-workspace baseline promotion until a replacement
contract lands.

## Work item: corpus-receipt-freshness-spec

Status: completed; PR #8813
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0004](../../docs/specs/PLSP-SPEC-0004-corpus-receipt-freshness.md)
Linked ADR: [PLSP-ADR-0001](../../docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md)
Blocks: raw-bucket-fixture-lane, linux-corpus-refresh, support-claim-refresh
Blocked by: parser-bucket-closeout-spec

Goal

Formalize how fresh and stale parser corpus receipts may be used.

Production delta

Added the receipt-state table, lane rules, valid/invalid claims, proof commands,
and claim boundaries.

Non-goals

No corpus sweep implementation, generated status edit, parser runtime behavior
change, or provider confidence rule.

Acceptance

Spec states that stale receipts route fixture discovery only and refreshed
corpus PRs are the only valid source for bucket-count movement.

Proof commands

```bash
git diff --check
```

Rollback

Revert PR #8813 and rely on `PLSP-SPEC-0001` until a replacement freshness
contract lands.

## Work item: generated-status-control-plane-adr

Status: completed; PR #8815
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0001](../../docs/specs/PLSP-SPEC-0001-parser-compatibility-bucket-closeout.md), [PLSP-SPEC-0004](../../docs/specs/PLSP-SPEC-0004-corpus-receipt-freshness.md)
Linked ADR: [PLSP-ADR-0001](../../docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md)
Blocks: implementation plan, active goal manifest, raw-bucket-fixture-lane
Blocked by: parser-bucket-closeout-spec, corpus-receipt-freshness-spec

Goal

Record the durable decision that generated status routes valid parser and
editor-trust work.

Production delta

Added and indexed `PLSP-ADR-0001`.

Non-goals

No generated status edit, behavior change, implementation plan, or active goal
manifest.

Acceptance

ADR states that specs interpret generated status, xtask owns generated content,
and agents must read status before choosing work.

Proof commands

```bash
git diff --check
```

Rollback

Revert PR #8815 and stop treating generated status as a formal control-plane
decision until a replacement ADR lands.

## Work item: confidence-before-cutover-adr

Status: completed; PR #8817
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0002](../../docs/specs/PLSP-SPEC-0002-provider-confidence-receipts.md), [PLSP-SPEC-0003](../../docs/specs/PLSP-SPEC-0003-real-workspace-editor-baseline.md)
Linked ADR: [PLSP-ADR-0002](../../docs/adr/PLSP-ADR-0002-confidence-before-cutover.md)
Blocks: provider-confidence-closeout, support-claim-refresh
Blocked by: provider-confidence-receipts-spec, real-workspace-baseline-spec

Goal

Record the durable decision that confidence/freshness receipts must exist before
compiler-backed provider facts authorize broader live behavior.

Production delta

Added and indexed `PLSP-ADR-0002`.

Non-goals

No provider behavior change, generated status edit, implementation plan, or
active goal manifest.

Acceptance

ADR states cutover rules for stale, low-confidence, generated, and dynamic facts
and requires fallback/blocker/live-comparison proof.

Proof commands

```bash
git diff --check
```

Rollback

Revert PR #8817 and block provider cutover PRs until a replacement cutover ADR
lands.

## Work item: raw-bucket-fixture-lane

Status: active
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0001](../../docs/specs/PLSP-SPEC-0001-parser-compatibility-bucket-closeout.md), [PLSP-SPEC-0004](../../docs/specs/PLSP-SPEC-0004-corpus-receipt-freshness.md)
Linked ADR: [PLSP-ADR-0001](../../docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md)
Blocks: linux-corpus-refresh, support-claim-refresh
Blocked by: generated-status-control-plane-adr

Goal

Continue source-backed fixture or narrow parser-fix work from
`parser.md#raw-failure-buckets`.

Production delta

Each PR locks one real-Perl parser shape or fixes one narrow parser behavior
with focused tests.

Non-goals

No bucket-count reduction claim without a refreshed corpus receipt. No parser
runtime change in fixture-only PRs.

Acceptance

Each PR names the generated status pointer, states receipt freshness, keeps the
scope PR-sized, and states allowed and unproven claims.

Proof commands

```bash
cargo test -p perl-parser-core --test <bucket-test> --profile agent --locked -- --nocapture
cargo xtask metrics parser-accuracy --check
cargo xtask update-status --only parser --check
cargo xtask metrics ratchet-check parser_accuracy
cargo xtask fmt --check
git diff --check
```

Rollback

Revert the focused fixture/fix PR. If a parser behavior fix regresses corpus
status, revert behavior first and leave fixture evidence for follow-up.

## Work item: linux-corpus-refresh

Status: ready
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0004](../../docs/specs/PLSP-SPEC-0004-corpus-receipt-freshness.md)
Linked ADR: [PLSP-ADR-0001](../../docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md)
Blocks: support-claim-refresh, lane-closeout
Blocked by: raw-bucket-fixture-lane or explicit decision to refresh now

Goal

Refresh the Linux system-Perl corpus receipt so raw bucket movement can be
claimed or explicitly deferred.

Production delta

Generated parser status reflects a current corpus sweep.

Non-goals

No parser runtime behavior change, fixture addition, provider change, or support
claim promotion in the refresh PR.

Acceptance

Corpus sweep completes on Linux, generated parser status is updated through
tooling, and the PR states bucket-count claims limited to that receipt.

Proof commands

```bash
cargo xtask parser-corpus-sweep --baseline .ci/parser-corpus-baseline.json --enforce --receipt
cargo xtask update-status --only parser --write
cargo xtask update-status --only parser --check
cargo xtask metrics ratchet-check parser_accuracy
git diff --check
```

Rollback

Revert generated receipt/status updates. If Linux roots are unavailable, close
with an explicit deferral note and keep fixture-only work in scope.

## Work item: real-workspace-baseline-run

Status: ready
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0003](../../docs/specs/PLSP-SPEC-0003-real-workspace-editor-baseline.md)
Linked ADR: [PLSP-ADR-0002](../../docs/adr/PLSP-ADR-0002-confidence-before-cutover.md)
Blocks: provider-confidence-closeout, support-claim-refresh
Blocked by: real-workspace-baseline-spec

Goal

Record at least one real-workspace baseline that proves cold start, indexing,
module resolution, provider behavior, and confidence boundaries.

Production delta

Adds a receipt or forensic/status link for the selected CPAN-style project.

Non-goals

No all-CPAN claim, no hidden network dependency for ordinary PRs, and no live
provider cutover from one baseline.

Acceptance

Baseline names the project/source, host/toolchain context, provider surfaces
covered or deferred, confidence/freshness links, and claim boundary.

Proof commands

```bash
just real-workspace-baseline mojolicious
cargo test -p perl-lsp-rs --test real_project_latency mojolicious -- --include-ignored --nocapture
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare --check
git diff --check
```

Rollback

Revert the receipt/status link. If the baseline exposes a failure, keep the
failure as a blocker issue and do not promote the claim.

## Work item: provider-confidence-closeout

Status: ready
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0002](../../docs/specs/PLSP-SPEC-0002-provider-confidence-receipts.md), [PLSP-SPEC-0003](../../docs/specs/PLSP-SPEC-0003-real-workspace-editor-baseline.md)
Linked ADR: [PLSP-ADR-0002](../../docs/adr/PLSP-ADR-0002-confidence-before-cutover.md)
Blocks: support-claim-refresh, lane-closeout
Blocked by: confidence-before-cutover-adr, real-workspace-baseline-run when project-scale proof is required

Goal

Close provider confidence gaps by ensuring provider surfaces have source,
provenance, confidence, freshness, fallback, blocker, and live-comparison
receipts before broader cutover.

Production delta

Provider status surfaces explain why each provider acted, fell back, blocked, or
remained shadowed.

Non-goals

No broad live provider cutover without the cutover requirements. No parser
bucket or corpus refresh work.

Acceptance

Provider confidence matrix or existing status surface records provider, fact
source, confidence, freshness, fallback, runtime receipt, live cutover state,
and next proof.

Proof commands

```bash
cargo test -p perl-lsp-rs-core --lib rename_shadow safe_delete_shadow -- --nocapture
cargo test -p perl-lsp-rs --lib refactor_runtime_blocker -- --nocapture
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare --check
git diff --check
```

Rollback

Revert the provider receipt/status PR. If a provider proof is unsafe, leave the
provider shadowed or blocked and file a narrow follow-up.

## Work item: support-claim-refresh

Status: ready
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0002](../../docs/specs/PLSP-SPEC-0002-provider-confidence-receipts.md), [PLSP-SPEC-0003](../../docs/specs/PLSP-SPEC-0003-real-workspace-editor-baseline.md), [PLSP-SPEC-0004](../../docs/specs/PLSP-SPEC-0004-corpus-receipt-freshness.md)
Linked ADR: [PLSP-ADR-0001](../../docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md), [PLSP-ADR-0002](../../docs/adr/PLSP-ADR-0002-confidence-before-cutover.md)
Blocks: lane-closeout
Blocked by: linux-corpus-refresh when parser claims change; provider-confidence-closeout when provider claims change; real-workspace-baseline-run when workspace claims change

Goal

Map user-facing LSP capability claims to proof commands, status docs, known
limitations, and next promotion proof.

Production delta

Users and release reviewers can see which parser/provider claims are supported
and which remain bounded.

Non-goals

No new parser or provider behavior. No unsupported full-CPAN or broad live
cutover claim.

Acceptance

Support/status rows link claims to proof commands and status docs, with known
limitations and next promotion proof.

Proof commands

```bash
cargo xtask update-status --only parser --check
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare --check
git diff --check
```

Rollback

Revert the support/status claim PR. If proof is stale, demote the claim and keep
the limitation visible.

## Work item: lane-closeout

Status: ready
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: all Real Perl Editor Trust specs
Linked ADR: all Real Perl Editor Trust ADRs
Blocks: none
Blocked by: active goal manifest, support-claim-refresh, provider-confidence-closeout, linux-corpus-refresh or explicit deferral

Goal

Close the lane when repo artifacts let agents choose the next parser, provider,
real-workspace, and support-claim work without chat history.

Production delta

The repo has proposal, specs, ADRs, plan, active goal manifest, generated status
pointers, and proof receipts aligned.

Non-goals

No new behavior, parser rewrite, full CPAN-clean claim, or live provider cutover
without its own proof.

Acceptance

Active manifest points to this plan and current status docs; implementation plan
has no missing required fields; status/support surfaces link claims to proof;
deferred items name successor work.

Proof commands

```bash
cargo xtask update-status --only parser --check
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare --check
git diff --check
```

Rollback

Reopen the lane by changing the manifest status back to active and adding a
specific ready work item with proof commands and claim boundaries.
