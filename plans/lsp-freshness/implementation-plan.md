# Fresh Facts Fast (LSP Freshness) Implementation Plan

Status: planned
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

## Current State

- `textDocument/didChange` currently parses inline on the request path in
  parts of `crates/perl-lsp-rs/`; the ParsedSnapshot seam (#3579) is the first
  step toward an off-lock, generation-tagged parse.
- Status pointers: [lsp status](../../docs/project/status/lsp.md),
  [status index](../../docs/project/status/index.md),
  [neovim didChange latency receipt](../../docs/project/status/neovim-didchange-latency-receipt.md).
- Work item 1 (`lsp-freshness-parsed-snapshot`, #3579) is active and is the
  gating dependency for every later phase in this plan.

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

### Phase 1 — ParsedSnapshot seam (#3579)

Owned Arc accessors, no dual state, invariant-owning construction, 3-way
generation validation at publish. Real edits preserve latest across a gap; a
failed parse publishes a current-generation failure snapshot.

Stop condition: the ParsedSnapshot type is the sole parsed-state truth, with
owned-Arc accessors and generation-validated publish, merged to main.

### Phase 2 — Pending-parse provider contract + canary

Stacked on #3579. Providers gain an explicit pending-parse contract (bounded
fallback, refusal, or pending state — never silent staleness). Proven with a
`sub foo` -> `sub bar` rename canary that exercises the pending-state path.

Stop condition: the pending-parse contract is documented and enforced for at
least one provider family, with the rename canary passing.

### Phase 3 — Latest-only off-lock parse worker

`didChange` returns before parse. Deterministic barriers, not sleeps, gate
test synchronization. A burst-of-20-edits receipt proves: `full_parse=0`
during the burst window, exactly 1 generation published, more than 0 stale
generations discarded, and 0 stale side effects.

Stop condition: the burst-of-20 receipt is captured and attached to the PR;
`didChange` no longer blocks on parse.

### Phase 4 — Off-lock provider snapshot consumption

Providers drop the document-map guard before analysis. Lock-hold
instrumentation confirms the guard is released before semantic work begins.

Stop condition: lock-hold instrumentation shows document-map locks are held
only for the snapshot handoff, not for analysis.

### Phase 5 — Generation-owned analyzer/type-env/facts

Derived semantics (analyzer, type-env, facts) are generation-owned, lazy, and
built once per generation. Once-per-generation counters and explicit eviction
prove no duplicate builds and no unbounded retention.

Stop condition: once-per-generation build counters and eviction are
observable and tested.

### Phase 6 — Current-generation workspace fact publication (#1711)

Workspace-index updates and extraction receipts are current-generation only.
Any preview/pre-pass added on this path must ship a receipt proving it costs
less than the extraction it avoids.

Stop condition: index publication is current-generation gated, with an
extraction-cost receipt attached.

### Phase 7 — Fact producer provenance (#3046)

Fact provenance (AST vs PIR-A vs framework vs oracle vs dynamic) is truthful
end to end — no compiler-backed claim ships without naming its producer.

Stop condition: provenance is attached to emitted facts and verified against
the real producer for each fact family touched.

### Phase 8 — References representative replay (#2674)

A references scorecard proves representative correctness on the current
generation-owned model before any fallback is removed.

Stop condition: the references replay scorecard is captured and attached.

### Phase 9 — Request-time workspace scan retirement (#1658)

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
