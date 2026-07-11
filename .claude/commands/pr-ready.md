---
description: Mark a reviewed draft PR as ready for CI
argument-hint: "<PR number>"
---

# Mark PR Ready

Mark a reviewed draft PR as ready for merge. This triggers CI. Context: **$ARGUMENTS**

## Steps

### 1. Parse PR number

Extract the PR number from $ARGUMENTS. If not provided, list open draft PRs:
```bash
gh pr list --state open --draft --json number,title,headRefName --template '{{range .}}#{{.number}} {{.title}} ({{.headRefName}}){{"\n"}}{{end}}'
```

### 2. Verify PR exists and is a draft

```bash
gh pr view $NUMBER --json isDraft,title,state
```

If the PR is not a draft, report: "PR #N is already marked ready" and stop.
If the PR is not open, report the current state and stop.

### 3. Verify review coverage

```bash
gh pr view $NUMBER --json labels,files
```

Policy:
- If the PR has the `deep-reviewed` label, proceed.
- If it does not have `deep-reviewed`, it may proceed **only** when every changed file is docs-only (`docs/**` or doc-text files such as `.md`, `.mdx`, `.txt`, `.rst`, `.adoc`).
- Otherwise: **STOP.** Report: "PR #$NUMBER cannot be marked ready — missing `deep-reviewed` on a non-docs PR. Route to reviewer-deep first."

Optionally validate receipt freshness when `deep-reviewed` is present to ensure the deep review covers the current HEAD:
```
/label-receipt-validate pr $NUMBER deep-reviewed
```

### 3.5 Verify conversation resolution and reviewer completion

`reviewDecision` alone doesn't prove either condition below — it says nothing
about thread resolution, and a review can predate the current push. Run the
canonical review-convergence check (see
[.claude/reference/review-convergence.md](../reference/review-convergence.md)):

```bash
scripts/ci/check-pr-review-convergence $NUMBER
```

Do not reproduce or modify its query locally.

**Never enable or retain auto-merge, and never mark a PR ready for merge
pickup, while the check above exits non-zero.** Threads must be resolved
for a reason (fixed/refuted/superseded/follow-up), each with
a machine-readable disposition reply posted BEFORE resolution — see the
canonical convention in
[.claude/reference/review-convergence.md § Disposition-reply
convention](../reference/review-convergence.md#disposition-reply-convention-before-calling-resolvereviewthread).
Never performatively — main mechanically requires conversation resolution
before merge. **Note:** the `resolved_without_disposition` detection
(flagging a resolved thread with no reply) is proposed in #3732 and is
**not yet live** in `check-pr-review-convergence` — it's held back for a
dogfood-advisory-first rollout. Until it lands, treat the disposition
requirement as process discipline you verify yourself, not something the
script's exit code proves. If the script exits non-zero, **STOP** and
report which reviewer or thread is still pending (its `BLOCK` lines name
them) instead of proceeding.

### 4. Mark ready and signal merge-readiness

```bash
gh pr ready $NUMBER
```

Apply the `merge-ready` label with verification (see `/label-apply-verified`):
```
/label-apply-verified pr $NUMBER "merge-ready"
```

The `merge-ready` label signals the ops agent that this PR has passed review and is cleared for merge pickup.

### 5. Write version-bound receipt

Record the label binding against the current HEAD SHA so the orchestrator can detect staleness:

```
/label-receipt-write pr $NUMBER merge-ready pr-ready
```

### 6. Report

Output: "PR #$NUMBER marked ready -- CI will trigger. Labeled merge-ready for ops pickup."

Include the PR URL for convenience:
```bash
gh pr view $NUMBER --json url --template '{{.url}}'
```
