# Fresh Facts Fast (LSP Freshness) Implementation Plan

Status: Phases 1-5 done; Phase 6's duplicate production traversal and Phase
8's representative replay are complete. Declaration extraction/cache churn
remains a bounded #4047 follow-up; fact provenance remains with #3046;
request-scan retirement remains a class-scoped #1658/#4002/#4046 lane.
Phase 10 (freshness runtime closeout) is blocked only on explicit successor
routing. See
[docs/reference/FRESH_FACTS_FAST_DONE_CONDITIONS.md](../../docs/reference/FRESH_FACTS_FAST_DONE_CONDITIONS.md)
for the narrower off-lock-async-parse-worker program proof (that document's
program scope is #3396's core substrate only — it does not cover the
#3046/#1711/#1658/#2674 goal-level extensions tracked by this plan and by
[.perl-lsp/goals/lsp-freshness.toml](../../.perl-lsp/goals/lsp-freshness.toml)).
Owner: perl-lsp maintainers
Goal manifest: [.perl-lsp/goals/lsp-freshness.toml](../../.perl-lsp/goals/lsp-freshness.toml)
Tracker: [#3396](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3396)
References: #3046 (fact producer provenance), #1711 (index publication receipts),
#1658 (request-time workspace scan retirement), #2674 (references replay)
Subordinate to: Real Perl Editor Trust (`.perl-lsp/goals/active.toml`) — this is
a lane-local program manifest, not a replacement for the active trust
objective.

## Purpose

Define the phase order for making every exact perl-lsp editor answer derive
from one immutable, generation-matched parsed/semantic snapshot. This plan is
a routing map for the `lsp-freshness` work items, not a product-claim
document. It mirrors the compiler-program plan's shape
([plans/compiler-program/implementation-plan.md](../compiler-program/implementation-plan.md)).

## Current State (reconciled 2026-07-13)

- `textDocument/didChange` commits the text-only mutation, `drop(documents)`s
  the document-map guard, then hands off to `ParseWorker::enqueue` — it
  returns before parsing. Verified in
  `crates/perl-lsp-rs/src/runtime/text_sync.rs`.
- Work items 1-5 (`lsp-freshness-parsed-snapshot` #3579,
  `lsp-freshness-pending-parse-contract` #3589,
  `lsp-freshness-latest-only-worker` #3618,
  `lsp-freshness-current-provider-policy` #3610 (+#3589),
  `lsp-freshness-generation-owned-analysis` #3765 (+#3811 hover cache
  retirement)) are merged to `origin/main` and marked `status = "done"` in
  the goal manifest.
- Status pointers: [lsp status](../../docs/project/status/lsp.md),
  [status index](../../docs/project/status/index.md),
  [neovim didChange latency receipt](../../docs/project/status/neovim-didchange-latency-receipt.md).
- The duplicate production reference traversal is retired through #4013/#4022;
  declaration extraction/cache churn remains bounded follow-up #4047.
- The representative references replay is merged through #3998/#4057. It is
  corpus-weighted measurement evidence, not a traffic-weighted report, and it
  authorizes no scan removal.
- Remaining successor work is fact provenance (#3046), class-scoped request
  scan retirement (#1658/#4002/#4046), and startup/readiness measurement plus
  workload/indexing follow-up (#4048/#4049/#4050). A responsiveness receipt and
  provenance-backed zero-false-exact confirmation remain future evidence.

## Objective

Make every exact perl-lsp editor answer derive from one immutable,
generation-matched parsed/semantic snapshot. `textDocument/didChange` must
commit text and return without parsing. Only the latest eligible generation
may publish AST state, diagnostics, workspace-index updates, symbols, or
semantic facts. Providers must consume current-generation facts or choose an
explicit bounded fallback, refusal, pending state, or documented
stale-compatible behavior.

## Claim Boundaries (apply to every phase)

- No parser grammar changes.
- No Perl-core base/comp/run burndown (another thread owns it).
- No HIR/PIR expansion unless a separate compiler issue requires it.
- No true-incremental-AST-reuse claim.
- No `TextDocumentSyncKind` change.
- No provider support-tier promotion without a promotion-ledger row.
- No fallback deletion without representative correctness replay.
- No request-time scan removal until replay proves parity or names the
  refusal boundary.
- No hard millisecond CI budgets from shared/debug hardware.
- Native Windows Rust/Cargo + isolated Windows cache paths only on this lane
  (no WSL/Bash cargo).

## Freshness Invariants (apply to every phase)

- Current access returns `None` on generation mismatch.
- Latest access is never the default provider path.
- Stale work has no side effects.
- Failed parse publishes a current-generation failure snapshot.
- `ParsedSnapshot` fields cannot be assembled inconsistently.
- rename/safe-delete/AST-edits fail closed without current proof.
- No compiler-backed claim without provenance.

## Phase Order

### Phase 1 — ParsedSnapshot seam (#3579) — DONE

Owned Arc accessors, no dual state, invariant-owning construction, 3-way
generation validation at publish. Real edits preserve latest across a gap; a
failed parse publishes a current-generation failure snapshot.

Stop condition: the ParsedSnapshot type is the sole parsed-state truth, with
owned-Arc accessors and generation-validated publish, merged to main. **Met —
merged via #3579.**

### Phase 2 — Pending-parse provider contract + canary (#3589) — DONE

Stacked on #3579. Providers gain an explicit pending-parse contract (bounded
fallback, refusal, or pending state — never silent staleness). Proven with a
`sub foo` -> `sub bar` rename canary that exercises the pending-state path.

Stop condition: the pending-parse contract is documented and enforced for at
least one provider family, with the rename canary passing. **Met — merged
via #3589.**

### Phase 3 — Latest-only off-lock parse worker (#3618) — DONE

`didChange` returns before parse. Deterministic barriers, not sleeps, gate
test synchronization. A burst-of-20-edits receipt proves: `full_parse=0`
during the burst window, exactly 1 generation published, more than 0 stale
generations discarded, and 0 stale side effects.

Stop condition: the burst-of-20 receipt is captured and attached to the PR;
`didChange` no longer blocks on parse. **Met — merged via #3618**
(`drop(documents)` precedes `worker.enqueue` in
`crates/perl-lsp-rs/src/runtime/text_sync.rs`).

### Phase 4 — Off-lock provider snapshot consumption (#3610) — DONE

Providers drop the document-map guard before analysis. Lock-hold
instrumentation confirms the guard is released before semantic work begins.

Stop condition: lock-hold instrumentation shows document-map locks are held
only for the snapshot handoff, not for analysis. **Met — merged via #3610
(+#3589).**

### Phase 5 — Generation-owned analyzer/type-env/facts (#3765, #3811) — DONE

Derived semantics (analyzer, type-env, facts) are generation-owned, lazy, and
built once per generation. Once-per-generation counters and explicit eviction
prove no duplicate builds and no unbounded retention.

Stop condition: once-per-generation build counters and eviction are
observable and tested. **Met — merged via #3765 (generation-owned lazy
analyzer + type environment); #3811 migrated hover to it and retired the
legacy uri+hash caches.**

### Phase 6 — Current-generation workspace fact publication (#1711) — COMPLETE

Workspace-index updates and extraction receipts are current-generation only.
Any preview/pre-pass added on this path must ship a receipt proving it costs
less than the extraction it avoids.

Stop condition: duplicate production traversal is retired and publication is
current-generation gated. **Met for the cutover via #4013/#4022.** Remaining
declaration extraction/cache-churn measurement is intentionally transferred
to #4047; it is not claimed complete here.

### Phase 7 — Fact producer provenance (#3046) — REMAINING

Fact provenance (AST vs PIR-A vs framework vs oracle vs dynamic) is truthful
end to end — no compiler-backed claim ships without naming its producer.

Stop condition: provenance is attached to emitted facts and verified against
the real producer for each fact family touched. **Not yet met — tracked by
#3046.**

### Phase 8 — References representative replay (#2674) — COMPLETE

A references scorecard measures representative behavior on the current
generation-owned model before any fallback is removed.

Stop condition: the references replay scorecard is captured and attached.
**Met via #3998/#4057.** Six initialized-lexical candidates remain for
#4002; method-shaped and Mojolicious package-sub rows are unexercised.

### Phase 9 — Request-time workspace scan retirement (#1658) — REMAINING

Retire the `ready-index` full-workspace request-time text scan by request
class only after first-failure instrumentation (#4002), bounded fallback
receipts (#4046), and representative parity/refusal evidence. Never remove it
globally from the replay result alone.

Stop condition: either parity evidence retires the scan, or a documented
refusal boundary replaces it — never a silent removal.

### Phase 10 — Freshness runtime closeout — BLOCKED

Closure receipts (ranged-edit, generation-gap, latest-only worker, fact
build/reuse, references scorecard, sub-foo->bar canary) are consolidated.
The runtime closeout records successor ownership in the manifest; the broader
product trust goal is not closed by this runtime closeout.

Stop condition: successor ownership is explicit in the manifest and tracker
#3396 is closed with a merge-proof comment. The remaining successor lanes keep
their own claims and proof obligations.

## Work Items

See [.perl-lsp/goals/lsp-freshness.toml](../../.perl-lsp/goals/lsp-freshness.toml)
for the machine-readable work-item list (`lsp-freshness-parsed-snapshot`
through `lsp-freshness-closeout`), each with its own `claim_boundary`,
`files`, and `commands`.

## Proof Commands (this orientation PR)

```bash
git diff --check
cargo xtask check-active-goal-manifest
```
