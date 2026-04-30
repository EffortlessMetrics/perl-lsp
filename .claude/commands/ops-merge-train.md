---
description: Ops bounded merge-train planner/checker for batch merges without making master the first integration branch
user-invocable: false
---

# Ops Merge Train (Issue #7288)

Use this protocol when processing a larger merge-ready queue. It creates a **bounded train plan + verification receipt** before any merge actions, so `master` is not the first integration branch.

## Scope and guardrails

- This is a **planner/check protocol**, not a merge bot.
- It does **not auto-merge** and does not require `--admin`.
- It does not weaken CI; it reuses existing required checks.
- Build train plans from **current green `master`** only.

## Candidate requirements (hard gate)

For each PR candidate, capture and verify:

1. PR number and **current head SHA** (`headRefOid`)
2. No active `needs-*` labels
3. CI status is green on the current head SHA, or explicitly passes **expected-skip normalization**
4. Mergeability:
   - `CLEAN`/`MERGEABLE`, or
   - intentionally ordered behind another train member to resolve overlap safely

Suggested query:

```bash
gh pr view <number> \
  --json number,title,isDraft,headRefOid,mergeable,mergeStateStatus,labels,statusCheckRollup
```

## Train sizing rules

Choose one train profile per run:

- **3**: overlapping cluster (default for shared files / higher risk)
- **5**: normal code cluster
- **10**: docs/leaf non-overlapping cluster

If uncertain, choose the smaller profile.

## Train planning

1. Confirm latest `master` CI is green.
2. Freeze `BASE_SHA` to current `master` head.
3. Build ordered list with dependency/overlap awareness:
   - infra/fixes before features
   - prerequisite PRs before dependents
   - high-overlap items earlier (or split to another train)

Record for each candidate:

- PR number
- pinned head SHA
- planned order index
- overlap/dependency note

## Train check loop (from BASE_SHA)

For each candidate in order, run a local integration simulation when possible:

1. Verify PR head SHA still matches pinned SHA.
2. Apply/simulate PR in train order (checkout/cherry-pick/merge simulation according to local ops tooling).
3. Run required checks:

```bash
just check-conflict-markers
cargo xtask fmt --check
cargo xtask gates --tier pr-fast --base origin/master --receipt
```

> Note: `--base origin/master` is required for comparable pr-fast scope behavior.

## Stop conditions (halt train immediately)

Stop and mark train as blocked on first occurrence of:

- conflict during apply/simulation
- stale candidate SHA (PR updated after plan freeze)
- failed check (conflict markers / fmt / pr-fast)
- unexpected skip outcome (non-normalized skip, missing required check)
- red `master` detected while train is in progress

No merges should proceed after stop until plan is regenerated from a fresh green master SHA.

## Output: merge-train receipt

Emit a markdown receipt (file or comment) with:

- train id/timestamp
- base master SHA used for planning
- selected train size profile (3/5/10)
- candidate list with pinned SHAs and order
- per-candidate check results
- final verdict: `READY` or `BLOCKED`
- stop reason (if blocked)

Template:

```markdown
## Merge Train Receipt

- Train: <id>
- Base master SHA: <sha>
- Profile: <3|5|10>
- Verdict: <READY|BLOCKED>

### Candidates (ordered)
1. #1234 @ <sha> — <note>
2. #1235 @ <sha> — <note>

### Checks
- #1234: conflict-markers ✅, fmt ✅, pr-fast ✅
- #1235: conflict-markers ✅, fmt ❌

### Stop reason
- fmt check failed on #1235 while simulating against base <sha>
```

## Minimal operator flow

1. `/ops-check-queue` to identify merge-ready set.
2. Build merge-train plan with candidate hard gates + profile sizing.
3. Run train check loop and write receipt.
4. Only if receipt verdict is `READY`, proceed to existing `/ops-merge-batch` execution.
5. After each real merge batch, re-verify `master` green before the next train.

This keeps integration confidence ahead of merge pace while preserving existing review and CI policy.
