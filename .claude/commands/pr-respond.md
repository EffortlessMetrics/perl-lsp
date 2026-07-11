---
description: Address PR review comments — read feedback, fix issues, push updates
argument-hint: "<PR number>"
---

# PR Respond

Read and address review comments on PR **$ARGUMENTS**.

## Process

### 1. Read the review feedback
```bash
gh pr view $ARGUMENTS --comments
gh api repos/:owner/:repo/pulls/$ARGUMENTS/reviews --jq '.[] | "\(.user.login) (\(.state)): \(.body)"'
gh api repos/:owner/:repo/pulls/$ARGUMENTS/comments --jq '.[] | "\(.path):\(.line) \(.body)"'
```

### 2. Categorize each comment

Classify each comment, then dispose of it for a real reason — never resolve a
thread performatively:
- **Fix**: valid issue — make the change
- **Refute**: not a real issue — explain why in the reply, with evidence
- **Supersede**: overtaken by a later commit/PR — cite the commit/PR
- **Follow-up**: valid but out of scope here — file an issue, cite it in the reply

(Maps onto the legacy triage labels: **Blocking** -> fix; **Suggestion** -> fix
or refute; **Question** -> refute (answer) or follow-up; **Nitpick** -> fix if
trivial, refute if subjective.)

### 3. Address every Fix-classified comment
For each comment classified **Fix** in step 2 (not just the ones a bot flagged
"blocking" — a non-blocking `Fix` still requires the change, or it shouldn't have
been classified `Fix`):
1. Read the file and understand the concern
2. Make the fix
3. Commit: `fix(review): address <reviewer> feedback — <what>`

### 4. Reply to comments

**Before resolving any thread**, reply with a machine-readable disposition —
see the canonical convention in
[.claude/reference/review-convergence.md § Disposition-reply
convention](../reference/review-convergence.md#disposition-reply-convention-before-calling-resolvereviewthread):

```
Disposition: fixed | refuted | superseded | follow-up
Evidence: <commit sha + test name>  /  <file:line + why>  /  <superseding head sha + seam>  /  <issue #N + why non-blocking>
```

```bash
gh api repos/:owner/:repo/pulls/$ARGUMENTS/comments/<comment-id>/replies \
  -f body="Disposition: fixed
Evidence: <commit-hash> + test <name>"
```

Or for general review comments:
```bash
gh pr comment $ARGUMENTS --body "Addressed review feedback:
- <comment 1>: Disposition: fixed
  Evidence: <hash> + test <name>
- <comment 2>: Disposition: refuted
  Evidence: <file:line>: <reasoning>
"
```

### 4.5 Resolve the conversation thread

**A thread must never be resolved with zero reply** — that's the
resolved-to-clear anti-pattern the #3647 incident shipped through (a
responder silently `resolveReviewThread`'d 15 threads with no reply and no
evidence; the PR merged with 6 live P1 defects). Follow this now as
process discipline: `check-pr-review-convergence` does **not yet**
mechanically detect a missing disposition reply — that detection
(`resolved_without_disposition`) is proposed in #3732, held back for a
dogfood-advisory-first rollout so it doesn't retroactively block PRs
already in flight. A resolved thread with zero reply currently passes the
script silently. Once #3732 lands, a resolved thread whose
`comments.totalCount <= 1` (no reply beyond the original comment) will be
`BLOCK`ing.

Only after step 4's disposition reply has been posted, resolve the GitHub
review thread:

```bash
gh api graphql -f query='mutation { resolveReviewThread(input: {threadId: "<thread-id>"}) { thread { isResolved } } }'
```

To find which threads still need attention (including their IDs), run the
canonical review-convergence check (see
[.claude/reference/review-convergence.md](../reference/review-convergence.md)),
which pages through every thread rather than truncating at 50:

```bash
scripts/ci/check-pr-review-convergence $ARGUMENTS
```

Do not reproduce or modify its query locally.

### 5. Re-verify
```bash
cargo xtask fmt
cargo clippy -p <crate> --tests -- -D warnings
cargo test -p <crate>
```

### 6. Push, set label, and request re-review
```bash
git push
```

Apply the `pr-responded` label with verification (see `/label-apply-verified`):
```
/label-apply-verified pr $ARGUMENTS "pr-responded"
```

```bash
gh pr edit $ARGUMENTS --add-reviewer <original-reviewer> 2>/dev/null || true
```

Before this PR can be marked ready or auto-merge enabled, every requested
reviewer must finish on the current HEAD SHA and every substantive thread must
be resolved. `reviewDecision` alone doesn't prove either: it says nothing about
thread resolution, and a review can predate the current push. Run the
canonical review-convergence check (see
[.claude/reference/review-convergence.md](../reference/review-convergence.md)):

```bash
scripts/ci/check-pr-review-convergence $ARGUMENTS
```

Do not reproduce or modify its query locally. Never enable or retain
auto-merge while the check exits non-zero.
