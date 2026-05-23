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
- `constant-provider-proof` is completed. The substrate proof hardened static
  `use constant` extraction for scalar, quoted scalar, hash, quoted-hash,
  plus-hash, and nested-value hash forms and recorded a completion shadow trace
  for constants as fresh `CompilerFact` / `SemanticAnalyzer` evidence without
  changing live completion behavior.
- `semantic-token-support-review` is completed. The class registry, human
  provider-promotion ledger, and machine ledger now agree on the scoped
  lexical-variable declaration/use rows, while semantic tokens remain
  output-neutral and `partial-live-with-fallback`.
- `prototype-table` is completed. HIR now records named subroutine prototype
  content and precise source ranges in a prototype table, and
  `RegisterPrototype` compile effects derive from that table without changing
  provider behavior, diagnostics, parser bucket claims, support tiers, PIR
  state, or determinism claims.
- `bareword-classifier` is completed. HIR now records source-backed syntactic
  roles for parsed identifier barewords without changing PL109 suppression,
  provider behavior, parser bucket claims, support tiers, PIR state, or
  determinism claims.
- `determinism-receipt-v1-spec` is completed. `PLSP-SPEC-0026` now defines the
  determinism receipt v1 contract, input classes, valid/invalid PR shapes,
  acceptance, proof commands, non-goals, and claim boundaries without adding a
  receipt generator, PIR implementation, runtime probe, provider behavior,
  support-tier promotion, release-lineage sync, or determinism claim.
- `cleanup-train-queue-review` is completed. The swarm queue was drained back
  to a controlled state before resuming spec-lock work.
- `differential-oracle-contract` is completed. `PLSP-SPEC-0027` now defines the
  differential real-Perl oracle contract, comparison classes, fixture and
  environment authority, disagreement classes, valid/invalid PR shapes,
  acceptance, proof commands, non-goals, and claim boundaries without adding an
  oracle runner, executing Perl, probing workspaces, changing provider
  behavior, promoting support tiers, moving parser/corpus buckets, syncing
  release lineage, or claiming conformance.
- `provider-promotion-ledger-maintenance` is completed. The direct class
  type-definition safe subset is recorded in the human ledger, machine ledger,
  dashboard, and plan without broadening provider behavior or support tiers.
- `oracle-receipt-schema-after-manifest` is completed. The checked-in receipt
  schema and validator now lock the future differential oracle receipt shape
  without adding an oracle runner, executing Perl, probing workspaces, changing
  provider behavior, moving parser/corpus buckets, promoting support tiers,
  syncing release lineage, or claiming conformance.
- `post-oracle-schema-routing-review` is completed. The active manifest no
  longer points at the completed receipt-schema slice, and the next substrate
  items remain assignment-gated until a separate PR explicitly starts an oracle
  runner or PIR contract lane.

Current executable slice

- `oracle-fixture-manifest-after-contract` is completed. The checked-in
  differential oracle fixture manifest and schema now declare fixture
  identities, source snapshots, path classes, Perl constraints, module roots,
  environment denials, dynamic/unsupported boundaries, and comparison classes.
- `oracle-receipt-schema-after-manifest` is completed. The receipt schema names
  comparison class, source snapshot, Rust extractor, Perl oracle, module-path
  authority, ambient/generated/dynamic/stale/unsupported inputs, normalized
  facts, comparison result classes, promotion effect, redaction,
  provider-behavior-change flag, and editor-runtime dependency denial.
- `devex-storage-safe-validation` is completed in the reliability lane. It was
  a control-plane routing item for storage-safe local validation and queue
  health; it did not change `cargo-safe`, build behavior, provider behavior,
  support tiers, parser/corpus buckets, release-lineage sync, or source-repo
  development routing.
  The 2026-05-22 post-burndown validation pass captured a clean queue-health
  checkpoint before the release/readiness follow-up PRs reopened the swarm
  queue. The same pass confirms `pr-fast` passes on `92f4a1b`, the manifest and
  gate policy checks pass, `cargo xtask devex-doctor` completes with required
  tooling available, and `storage-doctor` reports repo-local `target/` at
  `0.0G`. The only devex-doctor warnings are local hook advisories and the
  presence of a repo-local `target/` directory that storage-doctor measures as
  empty. That is a local storage receipt only, not a `cargo-safe`, build-system,
  provider, parser, support-tier, or release-lineage change.
- `policy-cleanup-routing-review` is completed in the reliability lane. It was
  a control-plane routing pass after the ancestry sync and queue burn-down: the
  swarm queue is empty, source-lineage ancestry is attached, the manifest and
  gate-policy checks pass, support/provider claim checks pass, and
  `storage-doctor` is green after removing stale repo-local build output. It did
  not broaden policy enforcement, provider behavior, support tiers,
  parser/corpus buckets, release-lineage sync, or source-repo development
  routing.
- `published-api-hygiene` is completed in the reliability lane. The slice kept
  public API checks storage-safe by routing the `just public-api-*` recipes
  through `scripts/cargo-safe` and refreshed the committed public API baselines
  to the current code surface. It did not change Rust API surface, broaden
  provider behavior, promote support tiers, move parser/corpus buckets, sync
  release lineage, or continue source-repo development.
- `provider-promotion-ledger-parity-review` is completed in the trust lane. The
  parity check now records 17 machine policy rows, 17 human ledger rows, and 18
  blocker registry entries. This was a control-plane parity review after the
  queue burn-down and public API hygiene closeout; it did not add or promote
  fact classes, broaden provider behavior, promote support tiers, move
  parser/corpus buckets, sync release lineage, or continue source-repo
  development.
- `semantic-token-class-receipts-as-needed` is completed as a routing marker.
  The next semantic-token PR may start only when a new scoped compiler-token
  class is ready to prove the same promotion, fallback, blocker, didChange
  freshness, and output-neutral span-invariant rules. Until then, semantic-token
  work remains assignment-gated and is not a broad compiler-backed token
  cutover.
- `semantic-token-class-declaration-readiness-review` is completed in the trust
  lane. `class_declaration` remains deferred because the runtime receipt does
  not prove exact live-output parity against an existing live `class` token; the
  reviewed class therefore receives no semantic-token class policy row, provider
  promotion row, support-tier movement, or provider behavior change.
- `receiver-real-workspace-quality-receipt` is completed in the trust lane. It
  added a receipt-only multi-file completion fixture that records constructor
  assignment, hashref slot, dynamic-key, and unknown-receiver behavior.
  Constructor assignment currently acts with source-backed detail; hashref-slot,
  dynamic-key, and unknown-receiver probes remain fallback or blocked. The
  receipt did not broaden completion behavior, promote support tiers, or treat
  generated, dynamic, stale, low-confidence, or unproven receiver facts as
  exact.
- `receiver-method-accessor-fallback-receipt` is completed in the trust lane. It
  added a receipt-only completion fixture for project-shaped accessor-return and
  method-return receiver chains. These medium-confidence receiver forms preserve
  low-confidence fallback and do not become exact source-backed completion
  evidence without a later promotion receipt.
- `receiver-bless-confidence-receipt` is completed in the trust lane. It added a
  receipt-only completion fixture for literal and dynamic `bless` receiver
  chains. Literal `bless` evidence stays labeled as medium confidence, and
  dynamic `bless` receivers do not become exact source-backed receiver evidence.
- `receiver-array-index-fallback-receipt` is completed in the trust lane. It
  added a receipt-only completion fixture for static and dynamic array-index receiver
  chains. Array-index receiver facts must preserve low-confidence fallback and
  must not become exact source-backed completion evidence without a later
  promotion receipt.
- `receiver-self-framework-accessor-fact-fixture` is completed in the trust lane.
  It adds a facts-only semantic analyzer fixture proving source-derived
  `$self = MyApp::Service->new` evidence plus framework accessor-return facts.
  It also locks fallback when the source-derived `$self` package does not match
  the framework declaration. This keeps the receiver fact substrate current
  without changing completion provider output, support tiers, parser/corpus
  status, or release lineage.
- `receiver-method-return-accessor-chain-fact-fixture` is completed in the trust
  lane. It extends method-return expression facts for a static constructor
  followed by a source-backed framework accessor, while preserving dynamic
  accessor-chain fallback. This is semantic substrate only and does not change
  completion provider output, support tiers, parser/corpus status, or release
  lineage.
- `receiver-local-accessor-chain-fact-fixture` is completed in the trust lane. It
  extends method-return expression facts for lexical locals initialized or
  assigned from a static constructor followed by a source-backed framework
  accessor, while preserving dynamic local accessor-chain fallback. This is
  semantic substrate only and does not change completion provider output,
  support tiers, parser/corpus status, or release lineage.
- `receiver-local-accessor-chain-fallback-receipt` is completed in the trust lane.
  It extends the receipt-only completion fallback fixture so lexical-local
  accessor-chain method-return receiver shapes preserve low-confidence fallback
  and do not become exact source-backed completion evidence without a later
  promotion receipt.
- `receiver-dynamic-local-accessor-chain-fallback-receipt` is completed in the
  trust lane. It extends the same receipt-only completion fallback fixture so a
  lexical local initialized from a dynamic accessor receiver preserves
  low-confidence fallback and does not become exact source-backed completion
  evidence.
- `receiver-conditional-local-reassignment-fallback-receipt` is completed in the
  trust lane. It extends the same receipt-only completion fallback fixture so a
  method-return receiver with conditional local reassignment preserves
  low-confidence fallback and does not become exact source-backed completion
  evidence.
- `receiver-source-backed-hash-slot-ux-receipt` is active in the trust lane. It
  extends the real-workspace receiver-quality receipt so the already-promoted
  plain hash-slot receiver pilot is visible in the editor UX harness as exact
  source-backed evidence, while hashref-slot, dynamic-key, and unknown receiver
  shapes remain fallback or blocked.
- The recent queue cleanup was tracked in
  [perl-lsp-swarm#88](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/88).

Claim boundary

- Oracle fixture manifest and receipt-schema work may align schemas, the
  checked-in manifest, active-goal routing, and planning docs. It must not add
  an oracle runner, execute Perl, probe workspaces, broaden provider behavior,
  promote support tiers, move parser/corpus buckets, sync release lineage, or
  use oracle agreement as provider promotion proof.
- Post-schema routing review may update only the active goal manifest and this
  plan. It must not select a parser bucket from stale status, start PIR, start
  determinism implementation, or promote provider behavior.
- DevEx storage-safe validation may update only active-goal routing and planning
  docs until a separate implementation PR names a concrete storage or validation
  contract. It must not change `cargo-safe`, build defaults, provider behavior,
  parser/corpus buckets, support tiers, or release-lineage sync.
- Published API hygiene may update the public API check recipes, routing docs,
  and committed public API baseline artifacts to match the current code surface.
  It must not claim semver compatibility, change Rust API surface, or promote
  provider/support/parser status.
- Provider promotion ledger parity review may update only active-goal routing,
  this plan, and ledger parity notes produced by existing checks. It must not
  change the ledger's promotion decision set, promote a fact class, or alter
  provider/support/parser status.
- Receiver real-workspace quality work may add a receipt-only UX test and status
  links for current completion behavior. It must not change completion provider
  logic, support tiers, parser/corpus buckets, generated/dynamic behavior,
  release-lineage sync, or source-repo development routing.
- Receiver method/accessor fallback work may add a receipt-only UX test and
  status links for current medium-confidence receiver behavior. It must not
  change completion provider logic, support tiers, parser/corpus buckets,
  generated/dynamic behavior, release-lineage sync, or source-repo development
  routing.
- Receiver conditional local reassignment fallback work may add a receipt-only
  UX test and status links for current method-return receiver fallback behavior.
  It must not change completion provider logic, support tiers, parser/corpus
  buckets, generated/dynamic behavior, release-lineage sync, or source-repo
  development routing.
- Receiver source-backed hash-slot UX receipt work may extend the existing
  receiver real-workspace quality receipt and status links for the already
  narrow source-backed plain hash-slot completion pilot. It must not change
  completion provider logic, support tiers, hashref-slot behavior, broader
  receiver promotion, parser/corpus buckets, generated/dynamic behavior,
  release-lineage sync, or source-repo development routing.
- Receiver local accessor-chain fallback work may extend the receipt-only
  method/accessor fallback UX test and status links for current lexical-local
  accessor-chain method-return receiver behavior. It must not change completion
  provider logic, support tiers, parser/corpus buckets, local accessor-chain
  receiver promotion, medium-confidence promotion, generated/dynamic behavior,
  release-lineage sync, or source-repo development routing.
- Receiver dynamic local accessor-chain fallback work may extend the
  receipt-only method/accessor fallback UX test and status links for current
  dynamic accessor-chain method-return receiver behavior. It must not change
  completion provider logic, support tiers, parser/corpus buckets, local
  accessor-chain receiver promotion, dynamic local accessor-chain receiver
  promotion, medium-confidence promotion, generated/dynamic behavior,
  release-lineage sync, or source-repo development routing.
- Receiver bless confidence work may add a receipt-only UX test and status links
  for current literal/dynamic `bless` receiver behavior. It must not change
  completion provider logic, support tiers, parser/corpus buckets,
  dynamic-boundary behavior, medium-confidence promotion, release-lineage sync,
  or source-repo development routing.
- Receiver array-index fallback work may add a receipt-only UX test and status
  links for current static/dynamic array-index receiver behavior. It must not
  change completion provider logic, support tiers, parser/corpus buckets,
  array-index receiver promotion, dynamic-boundary behavior, release-lineage
  sync, or source-repo development routing.
- Receiver self/framework accessor fact work may add facts-only semantic
  analyzer fixtures and status links for current source-derived `$self`
  constructor assignment plus framework accessor-return evidence and
  mismatched-package fallback. It must not change completion provider logic,
  support tiers, parser/corpus buckets, generated/dynamic behavior,
  release-lineage sync, or source-repo development routing.
- Receiver method-return accessor-chain fact work may add facts-only semantic
  analyzer fixtures and status links for current static constructor-to-framework
  accessor method-return evidence. It must not change completion provider
  logic, support tiers, parser/corpus buckets, generated/dynamic behavior,
  release-lineage sync, or source-repo development routing.
- Receiver local accessor-chain fact work may add facts-only semantic analyzer
  fixtures and status links for current lexical locals initialized or assigned
  from static constructor-to-framework accessor chains. It must not change
  completion provider logic, support tiers, parser/corpus buckets,
  generated/dynamic behavior, release-lineage sync, or source-repo development
  routing.

Proof commands

```bash
rtk gh pr list --repo EffortlessMetrics/perl-lsp-swarm --state open --limit 100 --json number,title,headRefName,mergeable,isDraft
rtk cargo test -p perl-lsp-ux-tests --test ux_scenario_47_receiver_method_accessor_fallback --profile agent --locked -- --nocapture --test-threads=1
rtk cargo test -p perl-lsp-ux-tests --test editor_ux_fixture_matrix --profile agent --locked -- --nocapture
rtk cargo xtask check-active-goal-manifest
rtk cargo xtask check-support-claims
rtk cargo xtask check-provider-confidence-matrix
rtk cargo xtask metrics parser-accuracy --check
rtk cargo xtask metrics ratchet-check parser_accuracy
rtk git diff --check
```

Rollback

Revert the receiver dynamic local accessor-chain fallback receipt PR. If the
receipt exposes an unintended provider-output promotion, keep provider behavior
unchanged, mark this work item planned, and open a separate fix with a narrower
claim boundary.
