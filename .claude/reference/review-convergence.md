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
   `BLOCK`. This was found by running the script against #3621's own PR:
   blocking on bot staleness meant the PR could never converge again once
   any auto-review app fell behind a push, without a human manually
   re-triggering it — the exact over-block class this script exists to
   eliminate. **Note:** stale bot reviews are now the *only* advisory-only
   (non-blocking) category the script reports — unresolved review threads
   (item 3, below) block regardless of `isOutdated`, so "the same
   treatment as an outdated thread" is no longer an accurate parallel to
   draw; see issue #3679.
3. **No unresolved threads — active or outdated.** Every `reviewThreads`
   node with `isResolved == false` blocks convergence, regardless of
   `isOutdated`. `isOutdated` (the diff hunk a thread commented on no
   longer exists at HEAD) is reported as metadata — it splits the
   unresolved count into `unresolved_active` and `unresolved_outdated` —
   but it is **not** a blocking discriminator. Both categories emit
   `BLOCK`.

   This was previously wrong: an earlier version of this script treated
   `isOutdated == true` unresolved threads as **ADVISORY**, non-blocking,
   reasoning that a thread pointing at code that no longer exists doesn't
   need to gate a machine verdict. That reasoning doesn't match GitHub's
   actual mergeability computation — this repo's active `main`
   branch-protection ruleset enforces `required_conversation_resolution`,
   which GitHub applies to **every** unresolved thread and does not consult
   `isOutdated` at all. PR #3621 proved this directly: it sat `BLOCKED` for
   many review cycles with 0 active threads but 9 outdated-unresolved
   threads; resolving those 9 (no other change) made the merge fire
   immediately. The old behavior meant this script could report
   `converged:true` on a PR GitHub would still refuse to merge — the exact
   "internal verdict disagrees with GitHub mergeability" defect class this
   script exists to eliminate. See issue #3679.
4. **No resolved thread lacking a disposition reply.** `0 unresolved
   threads` is necessary but **not sufficient** — #3647 merged out-of-band,
   while its independent correctness review was still running, with 6 P1
   defects live on main, because a responder silently `resolveReviewThread`'d
   15 threads with **no reply at all**. The ruleset enforces "resolved"; it
   cannot enforce "resolved for a justified reason." This script closes the
   mechanical half of that gap: for every `reviewThreads` node with
   `isResolved == true`, it checks `comments.totalCount`. A thread whose
   `totalCount <= 1` (only the original review comment, nobody — bot,
   human, or the original author — ever replied before it was resolved) is
   the mechanical signature of resolved-to-clear, reported as
   `resolved_without_disposition` and `BLOCK`ed.

   **Heuristic, not intent detection — documented limits:**
   - A thread the *original* reviewer resolves immediately after posting a
     genuine one-line follow-up ("fixed in a1b2c3d") is **not** flagged —
     that reply is the second comment, so `totalCount` is 2. The check is
     "did anyone say anything after the finding," not "did someone else."
   - A low-effort reply ("ok") that isn't a real disposition still passes —
     the script can verify a reply's *existence*, not its *content*. See
     issue #3693 for the `Disposition: fixed|refuted|superseded|follow-up`
     reply-format convention this check assumes reviewers/responders
     follow (content-quality enforcement is a human/reviewer judgment
     call, not a mechanical one).
   - `totalCount` is connection metadata, accurate independent of how many
     comment `nodes` are actually fetched (the script only fetches
     `first: 1`, for the opening commenter's login).
5. **No independent review in flight.** The `needs-deep-review` label is
   treated as a durable, repo-visible blocking marker — while present,
   `independent_review_pending` is `true` and convergence is `BLOCK`ed
   regardless of thread/review state. This makes an in-flight correctness
   review mechanically visible: the other half of the #3647 root cause was
   that the PR merged while its deep review was still running, with that
   review existing only in an orchestrator's task list — not in the repo.

## R1 protocol axes (#3693) — advisory by default

Conditions 6–11 are the **review-protocol** layer added in R1. They upgrade
convergence from "threads resolved with *some* reply" (conditions 4–5) to
"every disposition is machine-checkable, every substantive fix is
independently verified at head, and every in-flight review is repo-visible."

**Rollout is advisory-first.** As of R1's landing, ~10 of the 14
most-recent PRs would trip one of these axes because their threads were
resolved before the disposition-marker convention existed. So conditions
6–11 default to **advisory**: the closeout computes each axis, reports it in
the JSON object, and prints a `WARN` line — but it does **not** flip
`converged` and does **not** change the exit code. Set
`REVIEW_PROTOCOL_ENFORCE=1` to promote every one of them to a hard `BLOCK`
(flips `converged`, exit 1, `WARN` → `BLOCK`). Flipping the default to
enforce is a deliberate later PR (R4), gated on a dogfood window proving the
axes don't deadlock legitimate PRs.

Conditions 1–5 are **unaffected** by the flag — they were hard blocks before
R1 and stay hard blocks. In particular condition 4
(`resolved_without_disposition`, the `totalCount<=1` no-reply signature) is
still a hard block; condition 6 below is its content-aware *tightening*, and
that tightening is what defaults to advisory.

6. **Every resolved thread carries a `disposition:v1` marker.** A resolved
   thread whose reply bodies contain no `<!-- disposition:v1 {…} -->` marker
   is counted in `dispositions_missing_marker`. This catches the case
   condition 4 misses: a thread with a *prose* reply (`totalCount >= 2`) but
   no machine-readable disposition. Post dispositions via
   `scripts/reviews/disposition`, which emits the marker and then resolves.
7. **Substantive dispositions are independently verified at head.** For
   every resolved thread with a class∈{fixed,refuted} disposition, a
   PR-level `verification:v1` receipt must exist at the current head with
   `result:"pass"` and a `verifier` **outside the writer set** (PR author +
   every disposer). Otherwise `verification_receipt_head_match` is `false`.
   The branch writer cannot verify their own substantive findings — the
   writer!=verifier invariant. (Fix-commit author is a further writer-set
   member the offline closeout cannot see; see open decision #4 in the R1
   spec — the canonical writer definition is a pending human call.)
8. **Fixed dispositions cite a commit reachable from head.** A class=fixed
   disposition whose `evidence.commit` is not an ancestor of the current
   head (prod: `git merge-base --is-ancestor`; fixtures:
   `head_reachable_commits.json`) is counted in `unreachable_fix_commits` —
   the cited fix never landed on this branch.
9. **Follow-up dispositions cite an issue number.** A class=follow-up
   disposition whose `evidence.issue` is missing or non-numeric is counted
   in `followups_without_issue`.
10. **No review-run receipt is still running.** A PR-level `review-run:v1`
    receipt with `status:"running"` means an independent review is in
    flight; `review_runs_in_flight` counts them. Post receipts via
    `scripts/reviews/run review-start|review-done`.
11. **The deep review receipt is bound to the current head.** If a
    `kind:"deep"` review-run receipt was posted `status:"done"`, one must be
    bound to the current head or `deep_review_receipt_head_match` is
    `false` — a deep review that ran against an older push does not protect
    the current head (the receipt-bound-to-older-head class).

These axes are read/written by the `scripts/reviews/` surfaces:
`state` (lifecycle position), `disposition` (the only sanctioned resolve
path), `run` (receipt poster), and `lease` (per-branch push lease). Receipt
shapes: `.ci/receipts/schemas/review-{run,verification,disposition,lease}.schema.json`.

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

### JSON output shape

```json
{"pr": N, "headRefOid": "...", "converged": true|false,
 "pending_reviewers": [...], "stale_reviews": [...], "stale_bot_reviews": [...],
 "unresolved_active": N, "unresolved_outdated": N, "unresolved_total": N,
 "resolved_threads": N, "resolved_without_disposition": N,
 "independent_review_pending": true|false,
 "review_protocol_enforce": true|false,
 "review_runs_in_flight": N, "verification_runs_in_flight": N,
 "deep_review_receipt_head_match": true|false,
 "verification_receipt_head_match": true|false,
 "dispositions_missing_marker": N, "followups_without_issue": N,
 "unreachable_fix_commits": N}
```

`unresolved_active` and `unresolved_outdated` both block convergence;
`unresolved_total` is their sum. `resolved_without_disposition` (count of
resolved threads with `comments.totalCount <= 1` — see item 4 above) and
`independent_review_pending` (the `needs-deep-review` label — item 5 above)
both block convergence too. `stale_bot_reviews` is a non-blocking (ADVISORY)
count. The R1 fields (`review_protocol_enforce` and everything after it —
items 6–11 above) are **advisory by default**: reported and `WARN`ed but
non-blocking unless `review_protocol_enforce` is `true` (set
`REVIEW_PROTOCOL_ENFORCE=1`). `review_protocol_enforce` echoes which mode the
run used.

### Test seam (no network)

Set `CONVERGENCE_TEST_FIXTURE_DIR=<dir>` (plus an explicit `owner/repo` as
arg 2) to make the script read canned JSON fixture files
(`pr_view.json`, `latestReviews.json`, `reviewThreads.json`, and — for the
R1 axes — optional `pr_comments.json` and `head_reachable_commits.json`)
instead of calling `gh`. A fixture that omits the two optional files reads as
"no PR-level receipts / no reachable-commit oracle", so pre-R1 fixtures keep
passing unchanged. This is exercised by
`scripts/tests/test-check-pr-review-convergence.sh` (and the lease suite
`scripts/tests/test-review-lease.sh`) and must never be set outside of tests.
`REVIEW_PROTOCOL_ENFORCE` must likewise never be set in production merge gates
until the R4 flip.

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

## Disposition-reply convention (before calling `resolveReviewThread`)

**This is the canonical statement of the convention — every other doc
that mentions resolving a review thread links here instead of restating
it, to avoid the same five-file drift described in "Why this exists"
above.**

Before **any** `resolveReviewThread` GraphQL call, the agent MUST first
post a reply comment on that thread carrying a machine-readable
disposition:

```
Disposition: fixed | refuted | superseded | follow-up
Evidence: <commit sha + test name>  /  <file:line + why>  /  <superseding head sha + seam>  /  <issue #N + why non-blocking>
```

- `fixed` — the commit SHA that fixed it, plus the test name that proves it.
- `refuted` — the `file:line` and the invariant/reasoning showing the
  concern doesn't apply.
- `superseded` — the head SHA of the change that overtook this thread and
  the seam it replaces.
- `follow-up` — the tracked issue number and why it's non-blocking here.
  Never write "will follow up" without a real issue number — untracked
  follow-up work silently disappears once the thread closes.

**A thread must never be resolved with zero reply.** That is the
resolved-to-clear anti-pattern the #3647 incident shipped through: a
responder silently `resolveReviewThread`'d 15 threads with no reply and no
evidence, and the PR merged with 6 live P1 defects because nothing forced
a reason to exist. Sequence is always **reply, then resolve** — never
resolve first and explain later, never resolve without replying at all.

**Mechanical enforcement status (as of this writing): NOT YET LIVE.**
`scripts/ci/check-pr-review-convergence` currently blocks only on pending
reviewers, stale human reviews, and unresolved threads (items 1-3 above)
— it does not read `comments.totalCount` and does not emit
`resolved_without_disposition`. That detection (flagging any resolved
thread whose `comments.totalCount <= 1` — no reply posted beyond the
original review comment — as `BLOCK`ing) is proposed in #3732, which is
deliberately **held back** for a dogfood-advisory-first rollout: this
convention lands and is followed by agents first, so the mechanical gate
doesn't retroactively block PRs already in flight when it goes live. Until
#3732 merges, a resolved thread with zero reply passes
`check-pr-review-convergence` silently — follow this convention as
**process discipline**, verified by the agent/reviewer doing the work, not
yet by the script's exit code. Once #3732 lands, the script enforces it
mechanically. Even then, the script can only verify a reply's *existence*,
not its *content* — following the exact `Disposition:`/`Evidence:` format
above is what makes the reply useful to a human reader and to any future
content-quality check, not just sufficient to pass the mechanical gate.

Every agent/skill that calls or instructs `resolveReviewThread` follows
this convention: `.claude/commands/pr-respond.md` step 4.5,
`.claude/agents/pr-responder.md` step 5, and the verification-side
references in `.claude/commands/pr-ready.md` step 3.5 and
`.claude/agents/ops.md`.
