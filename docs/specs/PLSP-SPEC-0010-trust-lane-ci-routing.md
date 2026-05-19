# PLSP-SPEC-0010: Trust-lane CI routing

Status: proposed
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked ADRs: [PLSP-ADR-0001](../adr/PLSP-ADR-0001-generated-status-is-control-plane.md),
[PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: PR plan, CI risk packs, CI lane whitelist, support tiers,
provider confidence matrix, parser status, Real Perl Editor Trust dashboard

## Current implementation status

The repository already has advisory CI economics and risk-pack infrastructure:

- [PR plan](../ci/pr-plan.md)
- [risk packs](../ci/risk-packs.md)
- [test evidence lanes](../ci/test-evidence-lanes.md)
- [CI lanes policy](../../policy/ci-lanes.toml)
- [CI risk packs policy](../../policy/ci-risk-packs.toml)
- [CI lane whitelist](../../policy/ci-lane-whitelist.toml)

This spec defines the trust-lane contract those surfaces must satisfy before
trust-lane routing can become a review or CI decision boundary. Current next
work, branch order, and open PR queue state belong in the routing dashboard,
implementation plans, PR bodies, and issue comments rather than this spec.

## Contract

CI proof should follow the claim boundary. A PR should pay for the proof
required by what it changes and claims, not broad proof by habit.

Trust-lane routing classifies a PR by the strongest claim it makes. The class
selects required checks, optional or label-gated checks, skipped-by-policy
checks, widening triggers, receipt paths, and support-claim impact. The routing
result must be visible to reviewers through the PR plan summary, a receipt, or
an equivalent CI step summary.

This spec does not authorize skipping required branch-protection checks by
itself. A later policy or validator PR may encode the classes in files such as
`policy/trust-lanes.toml` or extend existing risk-pack policy, but it must
preserve the contract below.

## Lane Classes

Every trust-lane class must define:

- required checks
- optional checks
- skipped-by-policy checks
- widening triggers
- receipt path or summary surface
- support claim impact

The initial classes are:

| Class | Claim boundary |
|---|---|
| `parser_fixture_only` | Adds or updates parser fixtures/status receipts without parser runtime changes. |
| `parser_runtime_fix` | Changes parser, lexer, AST, token, POD, regex, or source-position runtime behavior. |
| `provider_receipt` | Adds provider proof, traces, shadows, labels, or blocker receipts without live cutover. |
| `provider_live_cutover` | Broadens or enables live provider behavior from proven facts. |
| `support_claim_change` | Changes public support tiers, README claims, status claim rows, or proof wording. |
| `subprocess_seam` | Changes Perl binary, `@INC`, perldoc, DAP, launch config, or subprocess environment behavior. |
| `real_workspace_receipt` | Adds or changes real-project editor baseline, latency, memory, or livability receipts. |
| `release_proof` | Changes release, publish, packaging, semver, managed-binary, or distribution proof. |
| `dependency_update` | Changes dependency graph, lockfile, toolchain, or dependency policy. |
| `docs_status_only` | Changes prose, specs, ADRs, dashboards, or generated status without behavior changes. |

When a PR matches multiple classes, routing must use the highest-risk class or
the union of required checks. A lower-cost class must not hide a behavior change
behind docs, fixture, or receipt wording.

## Class Requirements

### `parser_fixture_only`

Required checks:

- focused parser fixture test
- parser accuracy check
- parser status or generated-status check
- ratchet or freshness check when the fixture affects generated status
- formatting and diff whitespace checks

Optional checks:

- parser runtime crate check when fixture shape touches shared parser helpers
- corpus receipt refresh when a claim references corpus movement

Skipped by policy:

- provider UX tests when no provider behavior changes
- live provider shadow or cutover checks
- release packaging checks

Widen if:

- parser runtime source changed
- support claim changed
- generated status changed unexpectedly
- corpus bucket movement is claimed
- provider behavior, diagnostics, or completion output changed

Receipt path:

- [parser status](../project/status/parser.md)
- [parser accuracy next](../project/status/parser_accuracy_next.md)

Support claim impact:

- no support-tier promotion unless a separate support-claim change cites fresh
  corpus or provider proof

### `parser_runtime_fix`

Required checks:

- focused parser runtime tests for the changed grammar family
- parser accuracy check
- parser status generation check
- ratchet or corpus freshness proof when bucket movement is claimed
- impacted downstream parser consumer check when AST/token contracts change

Optional checks:

- property, fuzz, or corpus lane when the changed runtime surface is broad
- UX/provider smoke when parser output feeds user-visible behavior

Skipped by policy:

- release proof unless release files changed

Widen if:

- AST shape changes
- generated status shifts beyond the targeted bucket
- provider behavior changes
- support tiers or README claims change

Receipt path:

- parser status receipts and focused parser test output

Support claim impact:

- parser bucket or compatibility claims require fresh generated status evidence

### `provider_receipt`

Required checks:

- provider-specific receipt test or shadow comparison
- confidence/freshness/fallback evidence check where available
- provider confidence matrix check
- support-claim check when wording or tiers change

Optional checks:

- semantic scorecard
- semantic shadow compare
- real-workspace receipt for noisy or project-shaped surfaces

Skipped by policy:

- live provider cutover proof when behavior remains shadow, labeled, or
  receipt-only

Widen if:

- live provider behavior changes
- fallback behavior changes
- edit-producing provider starts returning edits
- generated, dynamic, stale, or low-confidence facts become user-visible

Receipt path:

- [provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [provider cutover](../project/status/provider_cutover.md)
- [semantic scorecard](../project/status/semantic_scorecard.md)
- [semantic shadow compare](../project/status/semantic_shadow_compare.md)

Support claim impact:

- receipt-only PRs may add evidence but must not promote support tiers

### `provider_live_cutover`

Required checks:

- provider-specific receipt proof
- fallback proof
- blocker proof for stale, dynamic, generated, low-confidence, and ambiguous
  facts
- semantic shadow compare
- provider confidence matrix check
- support-claim check
- real-workspace receipt when the provider is noisy, destructive, or
  project-scale
- rollback or no-edit proof for edit-producing providers

Optional checks:

- UX scenario tests
- workspace memory or latency smoke
- VS Code command-surface smoke when the cutover changes editor commands

Skipped by policy:

- none by default; skipped checks must state why the cutover cannot affect that
  surface

Widen if:

- support tier changes
- returned edits are introduced
- fallback is removed or reordered
- generated/no-source or dynamic facts are promoted
- workspace index freshness or identity guards change

Receipt path:

- provider-specific runtime receipts and the provider status surfaces above

Support claim impact:

- support tier may change only after proof and an explicit support-claim update

### `support_claim_change`

Required checks:

- support-claim validator
- provider confidence matrix validator when provider rows are referenced
- link or docs check when available

Optional checks:

- semantic scorecard or shadow compare for semantic/provider claim changes
- parser status check for parser claim changes

Skipped by policy:

- provider runtime tests when the PR is docs-only and only narrows or clarifies
  claims

Widen if:

- public docs claim broader live behavior
- a support tier is promoted
- proof commands change to reference newly generated evidence

Receipt path:

- [support tiers](../project/status/SUPPORT_TIERS.md)

Support claim impact:

- every changed claim must map to tier, proof, status owner, known limitation,
  and next promotion proof

### `subprocess_seam`

Required checks:

- seam-specific unit or runtime test
- workspace trust or setup-report receipt when user-visible state changes
- support-claim check when setup claims change
- platform proof when paths, process environment, or Perl binary resolution
  changes

Optional checks:

- UX scenario for PL701, perldoc, DAP, or launch-config behavior
- Windows guardrails for path/env changes

Skipped by policy:

- parser corpus checks when parser behavior is unchanged

Widen if:

- Perl binary selection changes
- `@INC`, include paths, `PERL5LIB`, perldoc, DAP, or launch env authority
  changes
- sensitive path redaction behavior changes

Receipt path:

- workspace trust report output or seam-specific receipt

Support claim impact:

- setup claims must say whether state is observed, configured, supplied by the
  client, or actively probed

### `real_workspace_receipt`

Required checks:

- scenario-specific real-workspace receipt
- parser/provider/status check for any claim the receipt supports
- memory or latency receipt when resource behavior is claimed

Optional checks:

- broader UX suite
- nightly real-repo or memory plateau lane

Skipped by policy:

- release proof unless packaging changes

Widen if:

- a real-project receipt is used to promote a support tier
- resource claims are added to README or support tiers
- new project baselines change fixture download, checkout, or cache behavior

Receipt path:

- real-workspace receipt artifact and owning status doc

Support claim impact:

- real-workspace claims must name project shape, measured surface, and known
  limits

### `release_proof`

Required checks:

- release packaging or publish dry-run proof
- changelog or release-history check where applicable
- managed-binary or extension packaging smoke when release surface changes

Optional checks:

- full CI or release-check label
- install smoke

Skipped by policy:

- parser/provider deep proof when no runtime or support claim changes

Widen if:

- package graph changes
- published artifacts, managed binaries, or marketplace files change
- version or release-history claims change

Receipt path:

- release evidence artifact or release-history status

Support claim impact:

- release PRs must not imply new provider support unless support proof exists

### `dependency_update`

Required checks:

- lockfile or manifest validation
- security/dependency audit lane when dependency risk changes
- affected crate check or smoke

Optional checks:

- full merge gate for broad dependency churn
- release dry-run for publish-sensitive dependencies

Skipped by policy:

- provider receipts when public behavior is unaffected

Widen if:

- dependency is used in parser, provider, subprocess, security, or release code
- MSRV or toolchain changes
- generated code or build scripts change

Receipt path:

- dependency audit output or CI policy summary

Support claim impact:

- dependency PRs do not change support claims unless public behavior changes

### `docs_status_only`

Required checks:

- diff whitespace check
- docs checker or status validator for the touched docs surface
- generated-status check when generated docs are changed

Optional checks:

- support-claim validator when public claim wording changes
- provider matrix check when provider status links or rows change

Skipped by policy:

- Rust compile and runtime tests when no code, generated receipt, or support
  claim behavior changed

Widen if:

- docs add or promote public behavior claims
- specs redefine provider or edit-producing behavior
- generated status is hand-edited
- docs-only PR touches workflow, policy, or release files

Receipt path:

- docs gate output or status validator output

Support claim impact:

- docs-only PRs may clarify or narrow claims; promotions require proof

## Acceptance

A trust-lane CI routing change satisfies this spec when:

- it identifies the PR class or classes from changed files and claimed behavior
- it lists required, optional, and skipped-by-policy checks
- skipped checks include a reason
- widening triggers are explicit
- receipt or summary paths are visible to reviewers
- support-claim impact is stated
- parser fixture, provider receipt, live cutover, and support-claim changes are
  not collapsed into one cheap lane
- docs/status-only routing cannot hide behavior changes

## Valid PR Shapes

Valid PRs include:

- adding this spec as a docs-only contract
- adding `policy/trust-lanes.toml` or an equivalent policy table
- adding or extending PR summary output to show trust-lane class, changed
  surface, required proof, hosted-CI estimate, and skipped-by-policy checks
- adding a validator that ensures class names, lanes, labels, and receipt paths
  resolve
- adding a cheap parser fixture lane that widens when runtime or support claims
  change
- updating risk packs to route a proven trust class to existing lane IDs

## Invalid PR Shapes

Invalid PRs include:

- using docs-only routing for behavior changes
- treating parser fixture PRs as parser bucket movement without fresh generated
  status proof
- letting receipt-only provider PRs promote support tiers
- letting live provider cutovers skip fallback, blocker, shadow, or support
  claim proof
- using planned CI routing as evidence that a provider behavior is safe
- silently skipping expensive checks without a skipped-by-policy reason
- adding `full-ci` as the default answer for every trust lane
- changing CI routing and broad provider behavior in the same PR

## Proof Commands

A docs-only PR for this spec must run:

```bash
python scripts/ci/validate_risk_packs.py --strict
git diff --check
```

Policy or validator PRs that encode this spec must also run the relevant
policy checks. At minimum:

```bash
python scripts/ci/validate_risk_packs.py --strict
cargo xtask check-support-claims
```

Provider or parser claim-routing PRs must add the owning status checks, for
example:

```bash
cargo xtask check-provider-confidence-matrix
cargo xtask semantic-shadow-compare --check
cargo xtask update-status --only parser --check
```

The PR body must state which class was changed, which proof was run, and which
checks are intentionally skipped by policy.

## Non-goals

- no broad full-CI default
- no replacement for existing risk packs, lane whitelist, or PR plan docs
- no provider live cutover from CI policy alone
- no support-tier promotion from CI routing alone
- no generated parser status counts in this spec
- no current open PR queue, branch names, or temporary CI failure details
- no release approval, publish approval, or tag execution

## Claim Boundaries

Trust-lane CI routing proves that a PR bought the proof required for its claim
class. It does not prove the product behavior unless the selected checks and
receipts cover that behavior.

Docs/status-only routing may claim only documentation or control-plane
coverage. Provider receipt routing may claim only evidence collection.
Provider live cutover routing may support a behavior claim only when the
provider-specific receipts, fallback, blocker, support-claim, and real-workspace
proof required by the class pass.

Status docs own current evidence. CI policy owns routing mechanics. Specs own
the invariant boundary between claim, proof, and cost.
