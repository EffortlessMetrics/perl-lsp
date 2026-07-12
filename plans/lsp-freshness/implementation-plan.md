# Fresh Facts Fast (LSP Freshness) Implementation Plan

Status: Phases 1-5 done (core freshness invariant merged and verified on
origin/main); Phases 6-9 remain (workspace-index scoping #1711, fact
provenance #3046, references replay #2674, request-scan retirement #1658);
Phase 10 (closeout) blocked on 6-9. See
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

## Current State (reconciled 2026-07-12)

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
- Remaining before the broader goal (not just the #3396 core substrate) is
  complete: workspace-index full re-extraction scoping (#1711), fact
  provenance (#3046), references representative replay (#2674), and
  request-time full-workspace scan retirement (#1658) — see Phases 6-9
  below, still `planned`/`blocked` in the goal manifest. A
  responsiveness-regression receipt for the freshness path and
  provenance-backed zero-false-exact confirmation on stale/dynamic/ambiguous
  fixtures are not yet produced as durable repository receipts.

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

### Phase 6 — Current-generation workspace fact publication (#1711) — REMAINING

Workspace-index updates and extraction receipts are current-generation only.
Any preview/pre-pass added on this path must ship a receipt proving it costs
less than the extraction it avoids.

Stop condition: index publication is current-generation gated, with an
extraction-cost receipt attached. **Not yet met — tracked by #1711.**

### Phase 7 — Fact producer provenance (#3046) — REMAINING

Fact provenance (AST vs PIR-A vs framework vs oracle vs dynamic) is truthful
end to end — no compiler-backed claim ships without naming its producer.

Stop condition: provenance is attached to emitted facts and verified against
the real producer for each fact family touched. **Not yet met — tracked by
#3046.**

### Phase 8 — References representative replay (#2674) — REMAINING

A references scorecard proves representative correctness on the current
generation-owned model before any fallback is removed.

Stop condition: the references replay scorecard is captured and attached.
**Not yet met — tracked by #2674.**

### Phase 9 — Request-time workspace scan retirement (#1658) — REMAINING

Remove the `ready-index` full-workspace request-time text scan only after
Phase 8's replay evidence proves parity, or the refusal boundary is
explicitly documented instead.

Stop condition: either parity evidence retires the scan, or a documented
refusal boundary replaces it — never a silent removal.

### Phase 10 — Closeout

Closure receipts (ranged-edit, generation-gap, latest-only worker, fact
build/reuse, references scorecard, sub-foo->bar canary) are consolidated.
Goal manifest and this plan are updated to `status = "complete"`. Successor
work is routed to a follow-up tracker.

Stop condition: all closure receipts listed above exist and are linked from
this plan; tracker #3396 is closed with a merge-proof comment.

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
