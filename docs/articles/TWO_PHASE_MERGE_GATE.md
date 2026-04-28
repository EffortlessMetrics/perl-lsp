# Two-Phase Merge Gate: Cache Once, Delta After

**Date:** 2026-04-23
**Status:** Design proposal

## The problem

Current merge-gate runs `just gates` on every PR push — roughly 8-15 minutes of full-workspace validation per run. When a PR is in the queue for 3 hours while other PRs merge, the full gate re-runs on each cascade-update, re-checking nearly everything. This is pure waste.

Observed this session: 21 deep-reviewed PRs cascade-updated after #5152 merged. Each re-ran the full merge-gate. Cumulative minutes: >300 GitHub runner-minutes for what was effectively the same validation against slightly-different parents.

## Two-phase model

**Phase 1 — First-approval full gate (current behavior, unchanged):**
- On `deep-reviewed` label application, run full `just gates`
- On success, cache a **baseline receipt** (`gate_baseline_SHA.json`) against the commit SHA that passed
- Record: which gates passed, their durations, the scope_json that produced the lane selection

**Phase 2 — Post-cascade delta gate:**
- On subsequent pushes to the same PR (e.g., after `pr update-branch`), compute the delta between the previous HEAD and the new HEAD
- If the delta is **empty** (branch-update pulled in master with no new PR commits): skip gate entirely, reuse baseline
- If the delta is **additive** (new commits): run only `just check-all-targets` + scoped clippy/test for the changed crates + any gate whose scope mask intersects the delta
- If the delta is **rewritten** (force-push, rebase): fall back to Phase 1 full gate

## Implementation sketch

```yaml
# .github/workflows/ci.yml
merge-gate:
  steps:
    - id: baseline-check
      run: |
        prev_receipt=$(cat target/receipts/gate_baseline_${{ github.event.pull_request.base.sha }}.json 2>/dev/null)
        if [ -n "$prev_receipt" ] && git diff --quiet ${{ github.event.pull_request.base.sha }}..HEAD; then
          echo "skip=true" >> "$GITHUB_OUTPUT"
          echo "::notice::Cascade-only update; baseline receipt reused"
        fi

    - id: scope-delta
      if: steps.baseline-check.outputs.skip != 'true'
      run: cargo xtask ci-scope --base prev_baseline_sha --format json

    - id: delta-gate
      if: steps.baseline-check.outputs.skip != 'true' && steps.scope-delta.outputs.diff_class != 'prose_only'
      run: just check-all-targets && cargo clippy $scoped_crates -- -D warnings

    - id: baseline-write
      if: success() && steps.scope-delta.outputs.full_required == 'true'
      run: cargo xtask write-gate-baseline --receipt target/receipts/receipt.json
```

## Expected savings

- **Cascade-update storm** (N PRs update-branched after a master merge): 1 full gate + N delta gates instead of N full gates. For N=20: ~290 runner-minutes saved.
- **Steady-state PR churn** (Codex pushes fmt-fixes, reviewer-deep pushes fix-forward): delta gate runs instead of full. ~5 min vs ~12 min per push.
- **First-time PR approval**: unchanged — full gate still runs.

## Failure modes to guard

- **Stale baseline across master divergence**: if master has moved, baseline is invalid. Mitigate: baseline SHA must be an ancestor of current master; if not, fall back to Phase 1.
- **Delta classifier false-negative**: ci-scope may not detect that a seemingly-unrelated crate was actually affected (via feature unification). Mitigate: keep `check-all-targets` on every delta pass as a safety net.
- **Test data drift**: snapshot regen between baseline and delta. Mitigate: snapshot tests run in every delta pass, regardless of scope.

## Prerequisite

Requires the gate receipt to be cacheable (currently is, via `target/receipts/receipt.json` + `gate-receipt-${{ github.sha }}` artifact). The baseline-lookup mechanism needs the pre-merge SHA to be knowable at CI time, which `github.event.pull_request.base.sha` provides.

## Relationship to existing work

- **#4939** (ci-scope classifier, merged) — provides the delta scope JSON
- **#5005** (tier-wiring in pr-smoke, merged) — established the pattern for scope-aware CI
- **#5263** (agent receipt, merged) — provides the machine-readable baseline format
- **#5271** (scoped integration-test compile, merged) — proved scope-delta works for the cheap path

## Next steps

1. File issue for the baseline-cache mechanism
2. Prototype the `baseline-check` step in a draft workflow
3. Measure delta gate wall-time vs full gate across 10 cascade-update scenarios
4. Ratchet: enable two-phase only when baseline SHA is within 24 hours of master HEAD (avoid stale-baseline risk)

Not blocking v0.13.0rc1 — this is post-release throughput optimization.

---

_Related: `docs/forensics/2026-04-23-tier-wiring-reviewer-fix-forward-session.md` (observed the waste pattern), `docs/articles/CONTINUOUS_REVIEW_PATTERNS.md`._
