# PLSP-PROP-0001: Real Perl editor trust

Status: proposed
Owner: perl-lsp maintainers
Created: 2026-05-13
Target milestone: Real Perl Editor Trust lane
Linked specs: planned `PLSP-SPEC-0001` through `PLSP-SPEC-0004`
Linked ADRs: planned `PLSP-ADR-0001`, `PLSP-ADR-0002`
Linked plan: planned `plans/real-perl-editor-trust/implementation-plan.md`
Support/status impact: parser, provider, semantic, UX, and real-workspace status
Policy impact: generated status remains source of truth; no stale receipt overclaims

## Problem

`perl-lsp` has strong synthetic coverage and growing provider receipts, but real
Perl projects expose parser, semantic, and provider boundary cases that isolated
fixtures do not fully represent.

The repo now has generated parser status, parser accuracy receipts, raw corpus
buckets, semantic scorecards, provider cutover status, UX dashboards, and
real-workspace tracking. The remaining gap is turning those artifacts into an
execution lane that improves user-visible editor trust without overclaiming.

Users should experience fewer false completions, safer navigation and refactor
behavior, clearer diagnostic boundaries, and better behavior on CPAN-style
projects. Maintainers and agents should be able to continue the lane from repo
artifacts alone instead of relying on chat history or operator memory.

## Users and Surfaces

- Perl developers using VS Code or another LSP client on CPAN-style projects
- maintainers debugging parser, semantic, or provider regressions
- agents continuing parser and provider work from generated status
- release reviewers deciding which user-facing claims are stable
- DAP users whose module paths and runtime seams depend on trustworthy Perl
  environment handling

## Current Evidence

Current facts live in generated or human-owned status docs. This proposal links
to those sources instead of duplicating their tables.

- [parser accuracy next](../project/status/parser_accuracy_next.md) reports no
  active failure packets and no measurement gaps, then hands off capability
  work to raw parser buckets only when generated parser status lists a nonzero
  raw bucket.
- [parser status](../project/status/parser.md) tracks the three parser baseline
  model: Ubuntu system Perl, CPAN top 1000, and the repo-owned project corpus.
- [parser status](../project/status/parser.md#parser-accuracy-observability)
  owns the current parser accuracy denominator and fixture/family counts.
- [parser raw failure buckets](../project/status/parser.md#raw-failure-buckets)
  own the current bucket queue and receipt freshness note.
- [provider cutover](../project/status/provider_cutover.md) and
  [UX capability dashboard](../project/status/ux_capability_dashboard.md) own
  provider claim boundaries and live/shadow state.
- [semantic scorecard](../project/status/semantic_scorecard.md) and
  [semantic shadow compare](../project/status/semantic_shadow_compare.md) own
  compiler-backed provider proof and regression status.

At proposal creation time, the generated parser handoff pointed to the largest
raw bucket inside the largest nonzero failure cluster, with stale bucket counts
treated as discovery input until a refreshed corpus receipt proved movement.
When current generated status lists `none`, agents must not start parser
bucket work from those historical names; they should refresh corpus evidence,
use a current failing source-backed fixture, or move to the next provider or
real-workspace trust lane.

## Operational Evidence

Recent parser and provider PRs show the lane already works manually:

- parser status and handoff PRs made the empty measurement queue route to raw
  parser capability buckets
- raw-bucket and freshness PRs made stale corpus data usable for fixture
  discovery without allowing bucket-count overclaims
- source-backed fixture PRs locked real Perl idioms from Unicode::Collate,
  Regexp::Common, ExtUtils, Carp, fields.pm, and related modules
- provider receipt PRs made stale and low-confidence compiler facts block or
  label behavior instead of silently authorizing unsafe edits

The proposal exists because that loop should become repo-owned method, not an
operator pattern remembered from prior transcripts.

## Success Criteria

- parser measurement queue routes to capability work when clear
- raw buckets are split into PR-sized fixture or parser-fix lanes only when
  generated status lists a current nonzero bucket or a current source-backed
  fixture fails
- stale corpus receipts are labeled and refreshed before bucket-count claims
- fixture-only PRs lock source-backed real-Perl shapes without claiming count
  reduction
- provider confidence and freshness receipts exist before live cutover
- real-workspace baseline covers at least one real project
- user-facing claims link to proof commands and known limitations
- skipped lanes are skipped by explicit policy, not silently ignored
- Codex and swarm agents can continue from active goals, implementation plans,
  status docs, and receipts without chat history

## Proposed Shape

Use generated status as a control plane.

`parser_accuracy_next.md` points to measurement gaps when they exist. When
measurement wiring is clear, it points to `parser.md#raw-failure-buckets`.
Raw bucket lanes proceed only from current nonzero generated bucket rows or
current failing source-backed fixtures. If generated status lists `none`, the
valid next step is to refresh corpus evidence or move to provider and
real-workspace trust work, not to restart stale bucket names.

Provider cutover proceeds only after confidence and freshness receipts prove the
candidate behavior. Stale facts, low-confidence facts, dynamic boundaries, and
generated members must block, label, or fall back instead of silently
authorizing edits or definitive navigation claims.

Real-workspace baselines bridge the gap between synthetic fixtures and user
trust. At least one CPAN-style project should prove cold start, indexing, module
resolution, completion latency, goto, hover, diagnostics, memory behavior, and
provider confidence before support claims broaden.

## Alternatives Considered

### Continue with status docs only

Generated and human-owned status docs already contain the most current facts.
Using only those docs would avoid more structure, but it leaves the lane's why,
contracts, PR sequence, and active goal state implicit. Agents can still do good
work, but they must infer claim boundaries from prior PRs or chat.

### Put all lane guidance in one planning document

A single document would be easy to find, but it would mix product rationale,
behavior contracts, durable decisions, current work, and generated evidence.
That makes review harder and increases the chance that stale prose competes
with generated status.

### Treat raw parser buckets as a normal bug queue

Raw buckets can identify useful parser work, but treating them as ordinary bugs
would encourage broad parser fixes and premature count-reduction claims. The
lane needs the stricter distinction between fresh receipt proof and stale
fixture discovery.

## Source-of-Truth Stack

The lane uses separate artifact layers:

- Proposal: why the lane exists and what user trust means
- Spec: behavior contracts, acceptance, proof requirements, and claim limits
- ADR: durable decisions, especially generated status as control plane and
  confidence before cutover
- Plan: PR order, proof commands, rollback, and handoff state
- Active goal manifest: machine-readable current state for Codex and swarm
  execution
- Status docs and policy ledgers: source of current evidence and claim
  boundaries

No artifact should do every job. Specs should link to generated status instead
of copying generated content, and plans should sequence work without becoming
product-claim documents.

## Risks

- Status/docs drift: mitigated by keeping generated status as current truth and
  using specs only to describe interpretation and proof rules.
- Overclaiming parser compatibility: mitigated by requiring fresh corpus
  receipts before bucket-count or support-tier claims.
- Broad parser rewrites from bucket labels: mitigated by PR-sized fixture or
  narrow-fix lanes.
- Unsafe provider cutover: mitigated by confidence, freshness, fallback, and
  blocker receipts before live behavior changes.
- Control-plane sprawl: mitigated by keeping proposals, specs, ADRs, plans,
  active goals, and status docs in separate roles.

## Non-goals

- no full CPAN-clean claim
- no broad parser rewrite
- no live refactor cutover without blocker receipts
- no dynamic Perl inference beyond documented boundaries
- no replacement of generated status docs with hand-maintained prose
- no mixing measurement, parser fixes, provider cutover, and docs scaffolding in
  one PR
- no bucket-count reduction claim without a refreshed corpus receipt

## Evidence Plan

Parser status proof:

```bash
cargo xtask metrics parser-accuracy --check
cargo xtask update-status --only parser --check
cargo xtask metrics ratchet-check parser_accuracy
```

Raw bucket proof:

```bash
cargo xtask parser-corpus-sweep --baseline .ci/parser-corpus-baseline.json --enforce --receipt
cargo xtask update-status --only parser --check
```

Provider confidence proof:

```bash
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare --check
```

Docs proof:

```bash
git diff --check
```

## Exit Criteria

The lane can close when all of these are true:

- parser measurement status routes clear measurement queues to raw capability
  buckets without chat context only when generated status lists current
  nonzero bucket evidence
- raw-bucket fixture/fix lanes state receipt freshness and avoid stale
  bucket-count claims
- provider confidence receipts cover completion, goto, hover, references,
  rename, safe-delete, diagnostics, and DAP module paths before broader live
  cutover
- at least one real-workspace baseline is linked to provider confidence and
  support-claim status
- implementation plan and active goal manifest can tell Codex the next
  parser/provider/real-workspace trust slice and proof commands
- user-facing support claims link to status docs, proof commands, and known
  limitations

## Claim Boundary

This proposal defines the lane and the product trust outcome. It does not create
behavior specs, alter generated status, claim parser bucket movement, broaden
provider behavior, or promote live refactor cutover. Those changes require their
own specs, plans, receipts, and PR-sized proof.
