---
description: Pre-mutation guard — confirm an issue is build-ready before creating a writer worktree/branch
argument-hint: "<issue#> [--mechanical]"
user-invocable: true
---

# Start Work

Advisory pre-mutation guard. Confirms one controlling issue is actually
build-ready **before** any writer worktree, branch, or production edit gets
created, then hands off to the existing entry points — it replaces none of
them. Context: **$ARGUMENTS**

This is **not** a required check and does not fork the `builder-ready` gate.
It composes over `.claude/agents/spec-planner.md`, the `worktree-manager`
skill, and [WORKTREE_PROTOCOL.md §Fresh branch](../../docs/reference/WORKTREE_PROTOCOL.md).
See issue [#3971](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3971)
for the full model.

## Step 1: Parse arguments

Extract `<issue#>` and the optional `--mechanical` flag from $ARGUMENTS. If no
issue number is given, stop and ask for one — this command never guesses a
target.

If `--mechanical` is present, skip to **Mechanical fast path** below instead
of the build-readiness check (Steps 3-4 only) — the issue-open check (Step 2),
collision check (Step 5), and fresh-`origin/main` fetch (Step 6) still apply.
The flag must be passed explicitly; never infer it from the issue body or
title.

## Step 2: Read the issue

```bash
gh issue view <issue#> --json title,body,labels,state,comments
```

Stop immediately if the issue is not `OPEN` — report its current state and
the PR/commit that closed it, if any.

## Step 3: Confirm `builder-ready` — the existing authoritative gate

Check `labels` for `builder-ready`. This is the **same** live signal
`lead-build.md` and `LIVE_SIGNALS_VS_LABELS.md` already treat as authoritative
— this step does not introduce a second gate, it turns the trust `lead-build`
already places in the label into an explicit pre-check.

**If `builder-ready` is absent: STOP.** Report:
```
#<issue#> is not builder-ready. Read-only investigation may continue, but no
writer worktree may be created. Next step: route through plan-review
(/plan-review-read, /plan-review-verify, /plan-review-stress,
/plan-review-improve) to reach a BUILD verdict. If this is a new concern that
hasn't been scoped yet, file or reconcile the controlling issue first — there
is no dedicated intake command for that step yet.
```

## Step 4: Confirm the latest plan revision has a matching BUILD verdict

Scan `comments` for the plan/plan-review comment-header convention already in
use on this repo (HTML comment markers, e.g. `<!-- implementation-plan:v1
revision: N ... -->` and `<!-- implementation-plan-review:v1 plan_revision: N
... -->` — see #3807 for a worked example):

1. Find the **highest** `revision` number among `implementation-plan:v1`
   comments — this is the latest plan revision.
2. Find every `implementation-plan-review:v1` comment whose `plan_revision`
   equals that number, posted **after** the plan comment it reviews.
   **Latest-wins:** if the latest plan revision has more than one review
   (e.g. an initial `REVISE`, then a later `BUILD` after the plan was
   patched in place, or vice versa), only the **chronologically last**
   review of that revision is authoritative. A stale `REVISE` followed by a
   later `BUILD` does NOT block; a stale `BUILD` followed by a later
   `REVISE`/`SPLIT` DOES block, even if an earlier review said `BUILD`.
3. Confirm that latest review's verdict is `BUILD` (not `SPLIT`, `REVISE`, or
   `REJECT`).
4. Check every comment posted **after** that review for language that
   materially invalidates it (a later `REVISE`/`SPLIT` verdict, a
   research-verifier finding that contradicts a load-bearing claim, a
   maintainer objection). A routine status update or an unrelated label
   change does not invalidate it.

Not every issue uses the formal comment-header convention — for simpler
issues, treat a plan-reviewer comment that explicitly states a build
recommendation as equivalent. Use judgment; when genuinely ambiguous, treat
it as unmet and stop.

**If no matching BUILD verdict exists, or a later comment invalidates it:
STOP.** Report:
```
#<issue#> carries builder-ready but the latest plan revision (rev N) has no
matching BUILD verdict [/ was invalidated by a later comment: <what and
why>]. Next step: re-run plan-review on the current revision before any
worktree is created.
```

## Step 5: Collision check — is someone already writing this?

Fetch remote branches **first** — `git branch -a` only shows remote-tracking
refs that have already been fetched into this checkout. A stale checkout
would silently miss a branch another agent just pushed, defeating the
collision check:

```bash
git fetch origin "+refs/heads/impl/*:refs/remotes/origin/impl/*"
gh pr list --search "#<issue#>" --state open
git branch -a --list "impl/<issue#>-*" "*/impl/<issue#>-*"
git worktree list
python3 scripts/worktree-manager.py query
```

Use the precise `impl/<issue#>-<slug>` branch-name convention spec-planner and
`WORKTREE_PROTOCOL.md` already use — **not** a bare `*<issue#>*` substring
glob. A bare substring glob false-matches on shared digit sequences (e.g.
searching `39` also matches `392`, `3971`, `4390`), producing phantom
collisions or masking real ones. Same care applies to the PR search: after
`gh pr list --search "#<issue#>"` returns hits, confirm the match is this
issue and not a substring/false hit — e.g. cross-check
`closingIssuesReferences` or the PR title/body for the exact `#<issue#>`
token, not just "the search returned something."

Look for: an open PR referencing the issue, a local or remote branch matching
`impl/<issue#>-*`, a live worktree on such a branch, or a worktree-manager
slot already owned by another agent for this issue.

**If any of these exist: STOP.** Report what was found and its state (e.g.
"PR #NNN is open and in-build — continue there via `/builder-read-pr`, do not
open a second worktree"). This mirrors `lead-build.md`'s existing duplicate-PR
and active-builder guards; `/start-work` just runs the same check earlier,
before mutation instead of before spawning a second builder.

## Step 6: Fetch fresh `origin/main`

```bash
git fetch origin main
```

Confirm the fetch succeeded before proceeding — a stale base produces the
exact `origin/master`-base-ref class of defect this repo has hit before.

## Step 7: Hand off — delegate, don't fork

All five conditions (issue open, `builder-ready`, matching BUILD verdict, no
collision, fresh `origin/main`) are the settled packet. `/start-work` does
not create the branch, worktree, or `.spec/` files itself — it hands off to
the existing entry point:

- **Pipeline work** (going through the standard build gate): spawn the
  `spec-planner` agent on the issue. It reads the `builder-ready` issue,
  creates `impl/<issue#>-<slug>` off `origin/main`, and writes
  `.spec/<issue#>-<slug>/{checklist,acceptance,context}.md` per its own todo
  list.
- **Direct/non-pipeline work** (no spec-builder workflow needed): use the
  `worktree-manager` skill's `allocate` command. **`allocate` already creates
  the git worktree itself** — it runs `git worktree add -B <branch> <path>
  <base>` for a new slot, or `git -C <path> checkout -B <branch> <base>` when
  reusing an idle slot (verified in `scripts/worktree-manager.py`). Do
  **not** follow it with a separate `git worktree add` — that would try to
  create a second worktree for the same branch at a different path and
  fail, or silently diverge from the one `allocate` already set up:
  ```bash
  python3 scripts/worktree-manager.py allocate --slot issue-<issue#> --branch impl/<issue#>-<slug> --owner <agent-id>
  ```
  The command's own stdout (`allocated slot=... path=... branch=... ref=...`)
  names the worktree path already created — `cd` into that path directly.
  [WORKTREE_PROTOCOL.md §Fresh branch (new work)](../../docs/reference/WORKTREE_PROTOCOL.md)
  documents the equivalent raw-`git worktree add` form for when the
  worktree-manager isn't in use; don't run both forms for the same slot.

Report which path was used and the resulting branch/worktree location.

## Mechanical fast path (`--mechanical`)

Reachable **only** via the explicit `--mechanical` flag — never inferred from
issue content. For work with no planning decision: generated-file
regeneration, dependency-lock bumps, format-only changes, typo fixes,
deterministic-fixture updates. Architecture, parser/compiler semantics,
concurrency, security, public LSP/DAP behavior, CI authority, merge policy,
and control-plane behavior always take the full path above, even with the
flag.

Still run **Step 2** (confirm the issue exists and is `OPEN`), **Step 5**
(collision check), and **Step 6** (fresh `origin/main`) — none of those are
planning-decision gates, they're basic write-safety (there must be a real,
open, uncontested issue to attach the mechanical change to). Only Steps 3-4
(`builder-ready` label / matching BUILD verdict) are skipped — those are the
planning-readiness gate, which mechanical work has no planning decision to
clear.

Before proceeding, record and print all five fields — do not create the
worktree until every field is filled in:

```
Scope: <exact files/change, one line>
Deterministic oracle: <what mechanically proves this is correct — a
  regeneration command, a formatter, a lockfile diff — not "looks right">
No behavior/policy change: <confirm — if this touches behavior, it is not
  mechanical; stop and use the full path instead>
No collision: <confirm Step 5 found nothing>
Rollback: <exact command to revert if this is wrong>
```

Then hand off exactly as in Step 7.

## Output

On success, print the settled packet (issue #, plan revision, BUILD verdict
comment link, collision check result, base SHA) and the branch/worktree
handed off to. On stop, print exactly one next action — never both a stop and
a partial handoff.

## Bootstrap note

`/start-work` cannot gate its own creation — the PR that adds this file is
exempt from its own check by construction. Do not add a `/start-work` guard
step to the PR that first introduces it.
