# Implementation Checklist: Green-CI Cancel Classification

## Overview
Implement logic in green-ci skill markdown to classify CI check cancellations as infrastructure noise vs. developer action. This prevents spurious RED verdicts when GitHub's concurrency group cancellation kills old checks.

**Files changed:** 2  
**Test harness:** N/A (skill markdown, no executable tests)  
**Verify command:** Manual empirical verification on next concurrency-cancellation event

---

## Step 1: Update green-ci-check.md — step 5 (verdict logic)

**File:** `/home/user/perl-lsp-swarm/.claude/commands/green-ci-check.md`

**Location:** Replace step 5 (current "Determine verdict")

**Change:**
- Add a new step 5a: **Classify cancellations** (before verdict)
  - For each check with `conclusion: cancelled`:
    - Extract `started_at` and `completed_at` timestamps
    - If `started_at == completed_at` (zero-duration) → mark as `INFRA-NOISE`
    - If `completed_at - started_at > 5s` → mark as `DEVELOPER-CANCEL`
  - For each check with `conclusion: failure` → mark as `RED` (ignore any cancel content)
  - For each check with `conclusion: success` → no change (existing behavior)

- Revise step 5b: **Determine verdict**
  - Count only RED checks (exclude INFRA-NOISE from failure tally)
  - RED verdicts: "Any check RED on current SHA → **RED**; list RED + DEVELOPER-CANCEL only"
  - GREEN remains: "All checks SUCCESS/NEUTRAL or INFRA-NOISE on current SHA"

**Verify:**
```bash
# Manually inspect the markdown syntax for clarity
grep -A 30 "5a. Classify" /home/user/perl-lsp-swarm/.claude/commands/green-ci-check.md
```

---

## Step 2: Update green-ci.md — Verdicts section

**File:** `/home/user/perl-lsp-swarm/.claude/agents/green-ci.md`

**Location:** "## Verdicts" section

**Change:**
- Add new verdict: **INFRA-NOISE** — check cancelled by concurrency group (zero-duration) → ignore and proceed to GREEN if no other RED checks
- Update **RED** verdict wording to clarify: "non-mechanical AND non-INFRA-NOISE failures"
- Update **GREEN** verdict to include: "or all checks are SUCCESS/NEUTRAL/INFRA-NOISE"

**Verify:**
```bash
# Confirm new verdict language is present
grep -A 5 "INFRA-NOISE" /home/user/perl-lsp-swarm/.claude/agents/green-ci.md
```

---

## Step 3: Update green-ci.md — "What you do NOT check" section

**File:** `/home/user/perl-lsp-swarm/.claude/agents/green-ci.md`

**Location:** "## What you do NOT check" section (after line ~47)

**Change:**
- Add new exclusion: "Concurrency-group-driven check cancellations (marked INFRA-NOISE in green-ci-check step 5a)"

**Verify:**
```bash
# Confirm new exclusion is listed
grep -A 1 "Concurrency-group" /home/user/perl-lsp-swarm/.claude/agents/green-ci.md
```

---

## Step 4: Commit and push

```bash
cd /home/user/perl-lsp-swarm
git add .spec/345-green-ci-cancel-classifier/
git add .claude/commands/green-ci-check.md
git add .claude/agents/green-ci.md
git commit -m "plan(green-ci): add cancel classification spec for #345"
git push -u origin impl/345-green-ci-cancel-classifier
```

---

## Dependency order

All three files are independent — can be edited in any order.

---

## Next: Builder handoff

- **Skip red-tdd:** No executable test harness for skill markdown. Verification is empirical.
- **Route to builder:** Assign to builder to edit the two markdown files per checklist.
- **Label:** Apply `spec-reviewed` after commit.
- **Expected outcome:** Next concurrency cancellation event will empirically validate the classification logic.
