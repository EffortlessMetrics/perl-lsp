# CI Hardening — Next Waves

## Snapshot (2026-04-29)

CI is materially healthier, but not complete. The cancellation cascade from PR label events has been addressed, `pr-fast` now runs via shared `xtask`, conflict-marker checks are in the fast lane, and non-admin merge flow has been re-validated.

The remaining gap is primarily operability: failure attribution, receipt quality, normalized status semantics, and merge/train discipline.

## Primary remaining gaps

1. PR-fast timeout attribution and per-gate receipts.
2. UX regression receipt enrichment — core classifier landed (#7386, #7394); `artifact_path` field and full routing coverage still outstanding.
3. Workflow-trigger hygiene verification across all CI workflows.
4. Stable status semantics for SKIPPED / path-conditioned lanes.
5. Merge-train / batch-validation operating protocol.
6. Deeper reconciler integration for receipts.
7. Parser Ratchet and semantic scorecard as evidence lanes (not opaque logs).

## Confirmed-good direction

- PR label events no longer restart core CI workflows.
- PR Smoke executes shared `cargo xtask gates --tier pr-fast ...`.
- `pr_fast` has policy-backed planning roles (`always_on`, `rust_scoped`, `rust_fallback`).
- UX external workflow trigger excludes `labeled` / `unlabeled`.
- UX regression receipt classifier landed: `xtask ux-regression-receipt` emits structured JSON with `failure_class`, `panic_location`, `repro`, `first_failing_line`, `route` (#7386, #7394).

## Wave 1 (implement first)

1. **Per-gate timeout attribution in `xtask gates`**
   - Add gate-level timeout enforcement and attribution before job-level timeout.
   - Receipts should include gate name, command, duration, timeout classification, and repro command.

2. **UX regression receipt — complete remaining gaps**
   - Core classifier already in master (#7386, #7394): `MatrixDrift`, `BaselineDrift`, `TestRace`, `NewTestBug`, `ProviderRegression`, `ServerCrash`, `Timeout`, `Infra`, `Unknown`.
   - Remaining: add `artifact_path` field to receipt struct; verify CI wiring covers all failure paths including non-harness exits.

3. **Close workflow trigger hygiene loop**
   - Verify all relevant workflows follow the same trigger contract and document exceptions.

4. **Land parser status marker pre-merge contract**
   - Keep parser marker integrity as a pre-merge signal, not a post-merge surprise.

5. **Normalize SKIPPED semantics**
   - Classify check states into: `required_and_passed`, `expected_skip`, `unexpected_skip`, `pending`, `failed`, `stale`.

## Wave 2

6. Merge-train / batch-validation protocol (ops playbook or small planner).
7. Review receipts projected by reconciler into labels.
8. `pr_fast` plan mode + planner coverage tests.
9. Bound and classify `semantic-scorecard --check` runtime/failure modes.

## Wave 3

10. Parser Ratchet advisory receipt lane.
11. Parser Ratchet base/head comparison lane.
12. Release-profile CPAN evidence lane.

## Guardrails (do not regress)

- Do not rely on global pre-push hooks as primary CI control.
- Do not normalize admin-merge as standard flow.
- Do not demote UX regression quality because receipts are weak.
- Do not make Parser Ratchet required before CI surfacing is stable.
- Do not let workflow-level timeout be the first timeout signal.
- Do not interpret SKIPPED as pass/fail without policy context.

## Core principle

Shift from **"CI catches failures"** to **"CI emits actionable, attributable receipts that tell agents what failed and what to do next."**
