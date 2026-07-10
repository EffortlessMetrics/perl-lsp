# Review Convergence — Canonical Authority

**Script:** [`scripts/ci/check-pr-review-convergence`](../../scripts/ci/check-pr-review-convergence)

Every skill or agent that needs to know "has this PR converged on review?"
calls this script. Nothing else re-derives the query. This doc is the
human-readable companion; the script's own header comment is the
authoritative contract — read it first if the two ever disagree.

## Why this exists

Three review rounds on this repo's control-plane fixes (culminating in PR
#3621, which addressed #3598) re-introduced the same defect in five
different files: `pr-ready.md`, `pr-respond.md`, `ops-merge-batch.md`,
`pr-responder.md`, and `ops.md` each computed "is review done?" with their
own hand-copied GraphQL query. The queries drifted from each other every
time one surface was fixed and the others weren't. The fix is not "fix the
query again" — it's "make it impossible to have more than one query."

## The contract

Review convergence is **not** the same question as `reviewDecision`.
`reviewDecision` (APPROVED / CHANGES_REQUESTED / REVIEW_REQUIRED) answers
"what did GitHub compute from review states" — it does not tell you:

- whether a reviewer's approval is stale (predates the current push), or
- whether any review **thread** is still open.

Both are separate, necessary conditions. A PR is only review-converged when
**all** of the following hold, as of the PR's current `headRefOid`:

1. **No pending review requests.** `reviewRequests` is empty — nobody who
   was asked to review has failed to respond at all.
2. **No stale human reviews.** Every non-bot reviewer's *latest* review
   (`latestReviews`, never the full historical `reviews` connection —
   earlier submissions are superseded and don't count) has
   `commit.oid == headRefOid`. A reviewer who approved three commits ago has
   not reviewed the current code. **Bot-authored reviews are excluded from
   this check** (author `__typename == "Bot"` — sourcery-ai, coderabbitai,
   chatgpt-codex-connector, cubic-dev-ai, factory-droid, etc.): they're
   unrequested auto-review apps, never appear in `reviewRequests`, aren't
   required by branch protection, and don't reliably re-run on every push
   (rate limits). A stale bot review is reported as **ADVISORY**, not
   `BLOCK` — the same treatment as an outdated thread. This was found by
   running the script against #3621's own PR: blocking on bot staleness
   meant the PR could never converge again once any auto-review app fell
   behind a push, without a human manually re-triggering it — the exact
   over-block class this script exists to eliminate.
3. **No active threads.** Every `reviewThreads` node with
   `isResolved == false && isOutdated == false` blocks convergence. Threads
   that are `isOutdated == true` (the diff hunk they commented on no longer
   exists at HEAD) are reported as **ADVISORY**, not **BLOCK** — they don't
   gate the machine verdict, but a human/agent pass should still glance at
   them since GitHub's UI can hide them by default.

## Pagination

Both `reviewThreads` and `latestReviews` are paginated GraphQL connections.
The script pages through both with `hasNextPage`/`endCursor` until
exhausted — never trust a bare `first: 50` on either field for a PR that
might have more than 50 threads or more than 50 review submissions.

## Usage

```bash
scripts/ci/check-pr-review-convergence <pr-number> [owner/repo]
```

- Exit `0` — converged (review-wise; this does **not** check CI, labels, or
  draft state — those are separate gates).
- Exit `1` — not converged; `BLOCK`/`ADVISORY` reasons on stderr, plus a
  human-readable `FAIL` summary line and a machine-readable JSON object on
  stdout.
- Exit `2` — usage or fetch error (not a verdict either way).

## Where it's called from

- `/pr-ready` step 3.5 (verify conversation resolution and reviewer
  completion before marking a draft ready)
- `/pr-respond` step 4.5 (before signaling `pr-responded` / requesting
  re-review)
- `/ops-merge-batch` step 2 (fresh green check, immediately before merge)
- `.claude/agents/pr-responder.md` (before treating the PR as ready)
- `.claude/agents/ops.md` (principles section — never merge/auto-merge
  while unconverged)

If you are adding a new surface that needs this check, call the script —
do not write a new GraphQL query for it, even if it looks like "just this
one field."
