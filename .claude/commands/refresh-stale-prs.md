---
description: Refresh PR branches with stale CI against current master — for use after fire-fix cascades or large merge batches
---

# Refresh Stale PRs

After a master fire-fix cascade (multiple mechanical unblocks landing in sequence) or a large merge batch, many open PR branches have stale CI results. Their local CI reports errors that are already fixed on master. This skill systematically rebases them.

Pairs with:
- `memory/feedback_tier_wiring_exposes_bitrot.md` — the cascade-exposes-debt principle
- `docs/articles/FIRE_FIX_CASCADE_METHODOLOGY.md` — full methodology
- Issue #5786 — the ops-level request this skill answers

## When to use

- Right after a structural-blocker fire-fix merges to master
- After any batch of 5+ merges within a short window
- When ops reports "40 PRs ready for merge but most show red CI"
- Proactively every few hours during high-throughput merge sessions

## Procedure

### 1. Identify the stale-CI queue

```bash
gh pr list \
  --state open \
  --search "status:failure is:open" \
  --json number,mergeable,headRefOid \
  --limit 50
```

### 2. Refresh MERGEABLE branches

Skip CONFLICTING — they need real rebase work, not just `update-branch`.

```bash
gh pr list \
  --state open \
  --search "status:failure is:open" \
  --json number,mergeable --limit 50 | \
  jq -r '.[] | select(.mergeable == "MERGEABLE") | .number' | \
  while read pr; do
    echo "=== PR #$pr ==="
    gh pr update-branch "$pr" 2>&1 | head -1
  done
```

### 3. Strip stale label-receipts on refreshed PRs

Label receipts (`ci-green`, `diff-audited`) are bound to a specific HEAD SHA. When the SHA changes via update-branch, those sign-offs are stale and must be re-verified.

```bash
# For each refreshed PR, strip stale receipts
gh pr edit <N> --remove-label ci-green --remove-label diff-audited
```

(Skip this step if label-receipt-validate is automated in CI.)

### 4. Wait for CI + merge

New CI runs will start automatically on the rebased SHAs. Wait ~5 minutes, then query for newly-green PRs:

```bash
gh pr list \
  --state open \
  --search "label:deep-reviewed label:diff-audited is:open" \
  --json number,mergeable,mergeStateStatus --limit 40
```

MERGEABLE + CLEAN → merge.

### 5. Report

- Count refreshed / count still CONFLICTING / count newly mergeable after CI run
- Note any PRs that still fail CI after rebase — those have real issues, not just stale-base

## Context about the fire-fix cascade

When master fails multiple checks (PR Smoke, Compile All Targets, Windows Guardrails, etc.), each fix exposes the next. The full cascade from 2026-04-23 was 14 iterations; see the forensic.

Until the cascade fully settles, PR branches refresh cycle by cycle — PRs branched at cascade step 3 will still show errors that step 5 fixed. This skill is the mass-refresh after cascade settles.

## Don't use this skill for

- PRs with genuinely failing tests from their own changes (run-reverter territory)
- PRs with merge conflicts — those need builder or rebase agent
- PRs that have been untouched for 30+ days (stale drafts, not stale CI)

## Example output

```
=== PR #5660 ===
✓ PR branch updated
=== PR #5714 ===
X Cannot update PR branch due to conflicts
=== PR #5756 ===
✓ PR branch updated
...
Refreshed: 23 / Conflicting: 6 / Newly mergeable: 18
```

## Related issues

- #5786 — the ops request this skill answers
- #4507 — push-gate proposal (prevents cascade in the first place)
- `docs/articles/FIRE_FIX_CASCADE_METHODOLOGY.md` — when to cascade vs. admin-merge
