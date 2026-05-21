# Real Perl Editor Trust Implementation Plan

Status: completed
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
Goal manifest: [active.toml](../../.perl-lsp/goals/active.toml)

## Current State

- [parser accuracy next](../../docs/project/status/parser_accuracy_next.md)
  reports 0 active failure packets and no measurement gaps.
- Parser bucket capability work routes through
  [parser raw failure buckets](../../docs/project/status/parser.md#raw-failure-buckets)
  only when generated parser status lists a nonzero raw bucket or a current
  source-backed fixture fails against the parser. When parser status lists
  `none`, do not start raw-bucket work from stale context.
- Raw bucket counts are point-in-time corpus receipt data. Fixture-only PRs may
  lock source-backed shapes, but only fresh corpus receipts may claim bucket
  movement.
- Provider confidence work routes through
  [provider confidence matrix](../../docs/project/status/provider_confidence_matrix.md),
  [provider cutover](../../docs/project/status/provider_cutover.md),
  [semantic scorecard](../../docs/project/status/semantic_scorecard.md),
  [semantic shadow compare](../../docs/project/status/semantic_shadow_compare.md),
  and [UX capability dashboard](../../docs/project/status/ux_capability_dashboard.md).
- User-facing support claims route through
  [SUPPORT_TIERS.md](../../docs/project/status/SUPPORT_TIERS.md), which maps
  claims to proof commands, status docs, limitations, and next promotion proof.

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

Status: deferred while generated parser status lists no nonzero raw bucket
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0001](../../docs/specs/PLSP-SPEC-0001-parser-compatibility-bucket-closeout.md), [PLSP-SPEC-0004](../../docs/specs/PLSP-SPEC-0004-corpus-receipt-freshness.md)
Linked ADR: [PLSP-ADR-0001](../../docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md)
Blocks: none until a current nonzero raw bucket or failing fixture reopens it
Blocked by: generated parser status listing no nonzero raw bucket

Goal

Do not start raw-bucket work while generated parser status lists `none`. Resume
source-backed fixture or narrow parser-fix work only from a fresh Linux corpus
receipt, a generated nonzero raw bucket, or a focused source-backed fixture that
fails against the current parser.

Historical shape analysis:
[unclosed_paren_identifier shape analysis](../../docs/project/status/parser_unclosed_paren_identifier_shapes.md).
AST boundary receipts:
`../../crates/perl-parser-core/tests/list_operator_boundary_receipts.rs`.

Next parser runtime work should start only from current failing evidence: a
fresh Linux receipt or a focused source-backed fixture that fails against the
current parser. Otherwise the deferred Linux corpus refresh owns bucket-count
movement.

Production delta

When this lane is reopened, each PR locks one real-Perl parser shape or fixes
one narrow parser behavior with focused tests.

Non-goals

No bucket-count reduction claim without a refreshed corpus receipt. No parser
runtime change in fixture-only PRs.

Acceptance

When this lane is reopened, each PR names the generated status pointer, states
receipt freshness, keeps the scope PR-sized, and states allowed and unproven
claims. If generated status still lists `none`, the PR must identify a current
failing source-backed fixture instead of a stale bucket name.

Proof commands

```bash
cargo test -p perl-parser-core --test <bucket-test> --profile agent --locked -- --nocapture
cargo test -p perl-parser-core --test list_operator_boundary_receipts --profile agent --locked -- --nocapture
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

Status: deferred on Windows; successor Linux corpus refresh
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0004](../../docs/specs/PLSP-SPEC-0004-corpus-receipt-freshness.md)
Linked ADR: [PLSP-ADR-0001](../../docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md)
Blocks: none for this closeout; successor Linux corpus refresh owns future
bucket-count claims
Blocked by: Linux system-Perl roots unavailable on this Windows host

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

Deferral note

The current Windows worktree does not have the Linux system roots named by the
stale receipt (`/usr/share/perl`, `/usr/lib/x86_64-linux-gnu/perl`, and
`/usr/share/perl5`). The refresh is deferred to a dedicated Linux corpus
refresh lane and must run on a Linux host before any parser bucket-count
movement is claimed.

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

Status: completed; receipt [2026-05-13 Mojolicious Windows baseline](../../docs/forensics/2026-05-13-real-workspace-baseline-mojolicious.md)
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0003](../../docs/specs/PLSP-SPEC-0003-real-workspace-editor-baseline.md)
Linked ADR: [PLSP-ADR-0002](../../docs/adr/PLSP-ADR-0002-confidence-before-cutover.md)
Blocks: provider-confidence-closeout, support-claim-refresh
Blocked by: real-workspace-baseline-spec

Goal

Record at least one real-workspace baseline that proves cold start, indexing,
module resolution, provider behavior, and confidence boundaries.

Production delta

Recorded a current Mojolicious Windows editor-latency receipt, refreshed the
raw latency JSON for that fixture, and updated the report generator/template to
state provider coverage, explicit deferrals, confidence/status links, outlier
interpretation, and claim boundaries.

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

Status: completed; PR #8852
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

Added [provider confidence matrix](../../docs/project/status/provider_confidence_matrix.md),
a row-per-provider status surface that records fact source/provenance,
confidence/freshness boundaries, fallback/blocker behavior, runtime/live
comparison receipts, real-workspace links, current live state, and next proof.

Non-goals

No broad live provider cutover without the cutover requirements. No parser
bucket or corpus refresh work.

Acceptance

Provider confidence matrix records provider, fact source, confidence, freshness,
fallback, runtime receipt, live cutover state, real-workspace link, and next
proof.

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

Status: completed; PR #8855
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: [PLSP-SPEC-0002](../../docs/specs/PLSP-SPEC-0002-provider-confidence-receipts.md), [PLSP-SPEC-0003](../../docs/specs/PLSP-SPEC-0003-real-workspace-editor-baseline.md), [PLSP-SPEC-0004](../../docs/specs/PLSP-SPEC-0004-corpus-receipt-freshness.md)
Linked ADR: [PLSP-ADR-0001](../../docs/adr/PLSP-ADR-0001-generated-status-is-control-plane.md), [PLSP-ADR-0002](../../docs/adr/PLSP-ADR-0002-confidence-before-cutover.md)
Blocks: lane-closeout
Blocked by: linux-corpus-refresh when parser claims change; provider-confidence-closeout when provider claims change; real-workspace-baseline-run when workspace claims change

Goal

Map user-facing LSP capability claims to proof commands, status docs, known
limitations, and next promotion proof.

Production delta

Added [SUPPORT_TIERS.md](../../docs/project/status/SUPPORT_TIERS.md), a
user-facing claim map for parser compatibility, module resolution, provider
surfaces, DAP subprocess seams, and real-workspace receipts. Each row links the
allowed claim to proof commands, status docs, known limitations, and next
promotion proof.

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

Status: completed; issue #8866
Linked proposal: [PLSP-PROP-0001](../../docs/proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked spec: all Real Perl Editor Trust specs
Linked ADR: all Real Perl Editor Trust ADRs
Blocks: none
Blocked by: none

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

Completion audit

- Source-of-truth stack: proposal, specs, ADRs, implementation plan, and active
  goal manifest are present.
- Parser control plane: `parser_accuracy_next.md` reports 0 active failure
  packets and hands capability work to `parser.md#raw-failure-buckets` only
  when generated parser status lists a nonzero raw bucket.
- Parser claim boundary: current generated parser status lists no nonzero raw
  bucket, so raw-bucket fixture work is not active from stale context. Future
  parser fixture/fix work must start from a fresh Linux corpus receipt, a
  generated nonzero raw bucket, or a current failing source-backed fixture and
  must not claim bucket-count movement without regenerated corpus proof.
- Linux corpus refresh: deferred on this Windows host because the Linux
  system-Perl roots are unavailable; a dedicated Linux corpus refresh lane owns
  the fresh receipt.
- Provider confidence: `provider_confidence_matrix.md` maps provider source,
  confidence, freshness, fallback, runtime comparison, real-workspace links,
  live state, and next proof.
- Real-workspace proof: the Mojolicious and Dancer2 Windows receipts record the
  covered editor-latency surfaces and explicit deferrals; the real-project
  resource receipt records fixture file, line, and byte shape while leaving RSS
  and heap ceilings to memory plateau receipts.
- Support claims: `SUPPORT_TIERS.md` maps user-facing claims to proof commands,
  status docs, known limitations, and next promotion proof.

Proof commands

```bash
cargo run -p xtask -- parser-corpus-sweep --manifest .ci/common-corpus-manifest.txt --enforce --receipt
cargo xtask update-status --only parser --check
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare --check
git diff --check
```

Rollback

Reopen the lane by changing the manifest status back to active and adding a
specific ready work item with proof commands and claim boundaries.

## Swarm Execution Follow-Up Queue

Status: active through [active.toml](../../.perl-lsp/goals/active.toml)

This section records the current swarm handoff after the original closeout. The
active goal manifest is the machine-readable source of truth; this plan only
states the PR order and claim boundary.

Recent routing

- `semantic-token-scoped-class-proof` is completed. The phase-block declaration
  proof added the scoped `token:phase_block_declaration:` class only when its
  source-backed span matches an existing live `macro` token, refreshes after
  `didChange`, and emits no new token output.

Current executable slice

- `constant-provider-proof` is active in the substrate lane.

Claim boundary

- Constant-provider work is substrate proof only unless a separate trust-lane PR
  promotes provider behavior.
- Do not change completion behavior, support tiers, or provider cutover claims
  from constant extraction tests alone.

Proof commands

```bash
rtk cargo test -p perl-semantic-analyzer use_constant --profile agent --locked -- --nocapture
rtk cargo test -p perl-lsp-rs-core --lib completion_shadow --profile agent --locked -- --nocapture
rtk cargo xtask check-active-goal-manifest
rtk cargo xtask check-support-claims
rtk cargo xtask check-provider-confidence-matrix
rtk git diff --check
```

Rollback

Revert the manifest/plan routing PR. If constant-provider proof is not ready,
mark `constant-provider-proof` blocked or planned in the active manifest and
select the next ready substrate item without changing provider behavior.
