# CI Hardening Status

## Snapshot (2026-04-30)

This page is the durable implementation status for CI hardening on `master` as of **2026-04-30**.
It separates what is already landed from what is partial/open so agents do not re-derive state from scattered PR threads.

## Landed items

1. **Label-trigger cancellation fixed**
   - Core CI no longer restarts from routine PR label churn; trigger hygiene has been tightened on the affected workflows.

2. **Shared `pr-fast` runner landed**
   - PR Smoke now routes through shared `xtask` gate execution (`cargo xtask gates --tier pr-fast ...`) instead of bespoke workflow-local command drift.

3. **Per-gate timeout attribution landed**
   - Gate-level attribution exists so timeout evidence can be tied to a specific gate command instead of only job-level cancellation.

4. **UX receipt classifier landed**
   - `xtask ux-regression-receipt` emits structured failure classification and triage fields.
   - Landed references: **#7386**, **#7394**.

5. **Status marker contract landed**
   - Parser/status marker validation has a contract path so marker integrity is caught before merge rather than discovered post-merge.

## Partial items (landed core, follow-up still needed)

1. **UX receipt coverage completion**
   - Core classifier is landed (#7386, #7394).
   - Remaining follow-up: artifact-path completeness and full routing coverage for all failure exits.

2. **Workflow trigger hygiene verification breadth**
   - High-risk workflows were corrected.
   - Remaining follow-up: full inventory verification + explicit exception registry.

## Open items

1. **`update-status` streaming remains open**
   - Tracking: **#7404**.

2. **Expected-skip normalizer remains open**
   - Need canonical `SKIPPED` outcome normalization (`expected_skip` vs `unexpected_skip`) before receipts/labels can be trusted at scale.

3. **Review receipts / reconciler projection remains open**
   - Need consistent projection of review receipt outcomes into reconciler-visible state.

4. **Merge-train protocol remains open**
   - Need an explicit batch/merge-train operating protocol to reduce queue thrash and stale-green churn.

## Exact verification commands

Run these from repo root on current `master`:

```bash
# Shared pr-fast execution path
just pr-fast

# Direct xtask gate invocation used by shared runners
cargo xtask gates --tier pr-fast --receipt

# Status marker / status docs contract check
just status-check

# Optional: regenerate + validate status outputs when touching status docs
just status-update
just status-check

# Canonical local merge receipt
nix develop -c just ci-gate
```

## Known non-goals (current wave)

- Making Parser Ratchet a hard required merge gate before CI receipt semantics stabilize.
- Treating every `SKIPPED` check as pass/fail without policy context.
- Reverting to workflow-specific hand-written `pr-fast` command stacks.
- Using admin-merge shortcuts as normal CI flow.

## Next-wave order (execution sequence)

1. **Close #7404 (`update-status` streaming)** so status evidence can be emitted incrementally and consumed reliably.
2. **Land expected-skip normalizer** to stabilize status semantics for reconciler and gate policy.
3. **Land review-receipt ↔ reconciler projection** so labels and receipts agree on actionable state.
4. **Codify merge-train protocol** (batch policy, stale-state invalidation, and rerun rules).
5. **Finish UX receipt tail work** (artifact-path completeness + edge-path routing coverage).

## Source notes

- Planning baseline: [`docs/project/CI_HARDENING_NEXT_WAVES.md`](../CI_HARDENING_NEXT_WAVES.md).
- This document is intentionally status-oriented (landed/partial/open + next actions) rather than replacing roadmap detail.
