# 2026-04-26 — Next-Session Work Plan

**Source**: Synthesized from 2026-04-25 PR queue drain session forensics + final-state snapshot + memory.
**Master HEAD at handoff**: `e6d5ed0a2` (verify still current at session start).
**Open PRs at handoff**: 327. **Open issues**: 430.
**API state at handoff**: GraphQL exhausted (0/5000), REST search 29/30 — quota should be fully restored; verify before fan-out.

This plan is sequenced so each block can be aborted on rate-limit pressure without leaving the queue worse off.

---

## First 30 minutes — label catchup + verify state

### A. Verify the world hasn't changed under us

```bash
# 1. API quota healthy?
gh api rate_limit --jq '.resources | {core: .core.remaining, graphql: .graphql.remaining, search: .search.remaining}'

# 2. Master HEAD still e6d5ed0a2? Has CI gone green on it?
gh api repos/EffortlessMetrics/perl-lsp/commits/master --jq '.sha'
gh api repos/EffortlessMetrics/perl-lsp/commits/master/check-runs --jq '.check_runs | map({name, conclusion, status}) | group_by(.name) | map(sort_by(.completedAt) | last)'

# 3. Open PR bucket counts (REST link-header trick, cheap on quota)
for label in merge-ready ci-green diff-audited needs-ci-fix needs-diff-fix needs-builder-fix; do
  count=$(gh api "search/issues?q=is:pr+is:open+label:${label}+repo:EffortlessMetrics/perl-lsp&per_page=1" --jq '.total_count')
  echo "${label}: ${count}"
done
```

Expected ballpark from handoff: `merge-ready=3, ci-green=2+, diff-audited≈129, needs-ci-fix≈19, needs-diff-fix≈10, needs-builder-fix≈13`. Significant deltas mean someone (Codex, scheduled jobs, the user) moved things — re-read before acting.

### B. Apply the 6 pending `maintainer-pr-reviewed` labels (single-shot, idempotent)

Pre-condition: each PR's last comment chain shows the maintainer-pr-reviewed verdict was actually written. If any PR was force-pushed since the verdict, strip the verdict and re-route, don't apply a stale label.

```bash
for pr in 5320 5220 5213 5051 5034 5032; do
  gh pr edit "$pr" --add-label maintainer-pr-reviewed
done

# Verify per feedback_label_skill_silent_failure
for pr in 5320 5220 5213 5051 5034 5032; do
  echo -n "#${pr}: "
  gh pr view "$pr" --json labels --jq '.labels | map(.name) | map(select(. == "maintainer-pr-reviewed")) | length'
done
# All should print "1". Any "0" → retry with --add-label, then escalate if it still fails.
```

### C. Strip the false-positive `needs-diff-fix` from #5750

Per session learnings, #5750's diff-audit verdict was a false positive. Strip the routing label so it can re-enter the pipeline naturally:

```bash
gh pr view 5750 --json labels --jq '.labels | map(.name)'   # confirm needs-diff-fix is present
gh pr edit 5750 --remove-label needs-diff-fix
gh pr comment 5750 --body "Stripping \`needs-diff-fix\` — prior diff-audit verdict was a false positive (no artifacts/regression/drift in the cumulative diff). Re-routing through pipeline."
```

### D. Refresh the .hermes/ contamination policy in working memory

Per the corrected policy from yesterday's reopen of the hermes false-positive:
- **Keep**: this-PR's own audit-trail entries (legitimate self-attribution).
- **Strip only**: cross-PR audit-trail entries naming a *different* PR's work.

This means the 8-PR list (#4890, #5684, #5685, #5691, #5714, #5785) is **not a blanket strip job** — each PR needs per-file inspection. Schedule for backlog (see below), not first-30.

---

## First hour — low-risk drains

### A. Ops drain on truly merge-ready

Sequence:

1. Re-confirm `merge-ready` set is non-empty and not draft:
   ```bash
   gh pr list --search "label:merge-ready -is:draft" --json number,title,headRefName,updatedAt
   ```
2. Spawn `ops-merge-batch` (batch of 3 max). Current candidates from handoff: **#5424, #5420, #5419**. After labels are caught up in Block B above, **#5320** should also be eligible (it had `ci-green`, just needed maintainer-pr label and merge-ready promotion).
3. Wait for the merge cascade, run `just cpan-corpus-ratchet` per protocol if any parser PR landed.
4. Cascade-update the diff-audited tail with `gh pr update-branch` for the next 10 PRs in line — but **strip `ci-green` from each one you update** (per `feedback_green_ci_false_positive_pattern`); they need re-verification after a cascade.

### B. Close 3 superseded PRs (#6240, #6239, #5388)

Cheap mechanical cleanup. Each gets a close-with-comment naming the superseding PR/issue:

```bash
for pr in 6240 6239 5388; do
  # First read each PR to get the supersession reason — DO NOT bulk-close blind
  gh pr view "$pr" --json title,body,comments --jq '{title, body: .body[0:500]}'
done

# After confirming supersession reason for each:
gh pr close 6240 --comment "Superseded by <PR-NUM>. Closing in favor of the merged/queued alternative."
gh pr close 6239 --comment "Superseded by <PR-NUM>. ..."
gh pr close 5388 --comment "Superseded by <PR-NUM>. ..."
```

Estimated time: 5 minutes if supersession reasons are clear from the PR bodies, 15 if they need cross-referencing.

### C. Verify the branch contamination claim (small, targeted, decides rebase scope)

For each of `#6379, #6252, #6192, #6138, #6051, #5320, #5220, #5213` run a non-mutating check:

```bash
for pr in 6379 6252 6192 6138 6051 5320 5220 5213; do
  branch=$(gh pr view "$pr" --json headRefName --jq '.headRefName')
  echo "=== #${pr} (${branch}) ==="
  # Count commits NOT on master + check for merge commits (the contamination signal)
  gh api "repos/EffortlessMetrics/perl-lsp/compare/master...${branch}" \
    --jq '{behind: .behind_by, ahead: .ahead_by, merge_commits: [.commits[] | select(.parents | length > 1)] | length, sample_titles: [.commits[].commit.message | split("\n")[0]] | .[0:5]}'
done
```

Decision tree on output:
- `merge_commits > 0` → contamination confirmed; needs `gh pr update-branch` (not local rebase) or close-and-recreate.
- `merge_commits == 0` and `ahead_by` matches expected scope → false alarm; strip whatever flagged it and let pipeline continue.
- Mixed → inspect the `sample_titles`; if they include unrelated work, contamination confirmed.

This is read-only and burns ~1 REST call per PR. Safe to fan out.

---

## First 2 hours — medium-difficulty work

### A. Architecture decision on #5793 vs #5795 (exporter metadata for #3416)

Both PRs implement `feat(semantic): extract per-file Exporter metadata`. Both labeled only `codex`, both `mergeable_state: unstable`. No gates run yet on either.

Process:
1. Apply the **`cluster-triage`** skill to issue #3416 (per handoff recommendation).
2. Read both diffs side-by-side; pick winner by:
   - Which fits the existing `perl-semantic-analyzer` module structure (architecture-reviewer lens).
   - Which has cleaner test surface.
   - Which extracts richer metadata (covers more `Exporter` patterns).
3. Post a synthesis comment on #3416 naming the winner.
4. Close the loser with cross-ref; route the winner into the normal verification pipeline (it's still pre-`needs-plan-review`).

Time budget: 45 minutes.

### B. Hard-conflict cherry-picks: #5711, #5710, #5695

These three carry the full sign-off chain (`research-reviewed`, `deep-reviewed`, `review-reviewed`, `maintainer-pr-reviewed`, `refactor-planner-reviewed`, `diff-audited`) and have `mergeable_state: unknown`. Auto-rebase failed on these yesterday.

For each, in a fresh worktree (per worktree-manager skill — never touch main checkout):
1. Allocate a worktree slot, checkout the PR's branch.
2. `git rebase origin/master` and resolve conflicts manually.
3. If conflicts touch `scope_analyzer.rs`, expect cascade pattern from `feedback_merge_conflict_cascade` — handle one PR at a time, not all three in parallel.
4. Force-push with `HEAD:refs/heads/<branch>` form (per `feedback_absorption_operational_lessons`):
   ```bash
   git push --force-with-lease origin "HEAD:refs/heads/<branch>"
   ```
5. Cascade-update needed: strip `ci-green` after the push, leave the rest of the labels intact.

Time budget: 60 minutes for all three if conflicts are small; abort to backlog if any single one takes >25 minutes.

### C. C21 Unicode resolution

Per handoff: reopen #6098 + cherry-pick #6099's emoji tag tests.

Process:
1. Reopen #6098 (it was closed prematurely):
   ```bash
   gh issue reopen 6098 --comment "Reopening — emoji tag range still needs plan-review. Pulling in emoji-specific tests from #6099 as we close the duplicate."
   ```
2. Cherry-pick the emoji tag tests from #6099 onto a new branch off `origin/master`. **Do not** touch the main checkout's branch state — use a worktree.
3. Open a new PR linking back to both #6098 and #6099.
4. Route through `needs-plan-review` so the verification ladder runs.

Time budget: 30 minutes.

### D. Investigate #6001 PR Smoke FAILURE

#6001 has the full chain (`deep-reviewed`, `review-reviewed`, `maintainer-pr-reviewed`, `ci-green`, `diff-audited`) but is flagged as having a PR Smoke failure that wasn't diagnosed yesterday.

```bash
gh pr checks 6001 --json name,state,bucket | jq 'map(select(.state != "SUCCESS"))'
gh pr view 6001 --json statusCheckRollup --jq '.statusCheckRollup | group_by(.name) | map(sort_by(.completedAt) | last) | map(select(.conclusion != "SUCCESS"))'
```

If the failure is mechanical (timeout, flake, infra) → re-run via `gh workflow run` or push an empty commit per `feedback_pre_push_hook_windows_race`. If it's a real test failure → strip `ci-green` and route to `needs-ci-fix` with a diagnostic comment.

Time budget: 20 minutes.

---

## Backlog — anytime / opportunistic

### .hermes/ cross-contamination cleanup (8 PRs, per corrected policy)

PRs: `#4890, #5684, #5685, #5691, #5714, #5785` (handoff lists 6 — need to rediscover the other 2 if they exist).

**Per-file inspection required** — not a blanket strip. For each PR:
1. List `.hermes/` paths added: `gh pr diff <pr> --name-only | grep '^\.hermes/'`
2. For each `.hermes/` path, check whether the work-id naming the PR matches the PR number under review.
3. **Strip only** entries naming a *different* PR. Keep this-PR's own audit trail.
4. Comment the diff applied so the trail is auditable.

This is a candidate for a single dedicated agent (haiku, scoped to one PR at a time) using the `check-agent-audit-trail` skill.

### Reviewer-deep continued sweep on the diff-audited bucket

129 PRs in the `diff-audited` bucket are queued behind green-ci. As ops drains the merge-ready tail and master moves forward, these need:
1. Stale-CI detection (their `ci-green` becomes stale on every master cascade).
2. `gh pr update-branch` to refresh.
3. Re-spawn `green-ci` to re-verify on the new HEAD SHA.

This is the long-tail work of the next session. Pace by master cadence — do not pre-update-branch all 129; do them in waves of 10-20 paced behind ops merges.

### New PR triage from sonnet web runs

The user mentioned fresh sonnet-web-generated PRs will land. Triage protocol:
1. Apply `ensemble-detect` skill — identify if they're part of a cluster.
2. If clustered: `cluster-triage` to pick winners, close losers with cross-ref.
3. If solo: standard scout-issue + needs-plan-review entry.

Reserve ~15% of session capacity for this; sonnet-web bursts arrive without warning.

---

## Master watch — continuous

### Bit-rot detection thresholds

Per `feedback_master_bitrot_cascade_8plus_pattern`: **3+ PRs failing the same individual gate identically = master signal**, not flakiness.

Detection script (run after every ops batch):

```bash
# Collect fresh CI failures from the last 30 minutes' worth of pushes
gh pr list --search "is:open label:diff-audited updated:>$(date -u -d '30 min ago' +%Y-%m-%dT%H:%M:%SZ)" \
  --json number,statusCheckRollup --jq '
    .[] | {pr: .number,
           failed: [.statusCheckRollup[] | select(.conclusion == "FAILURE") | .name] | unique}
    | select(.failed | length > 0)
  '
```

If 3+ PRs report the same failed check name, treat it as master bit-rot:
1. Reproduce locally (1-line fix + admin-merge per pattern).
2. Cascade-update the failing batch via `gh pr update-branch`.
3. Strip `ci-green` from the cascaded PRs to force re-verification.

Plan for **5-10 fixes** during this session if drainage is heavy (per the cascade pattern). Each fix unblocks 20-30 PRs.

### Sandbox-fail-closed timeout

Per session learning: 20-min timeout tripped a reviewer-deep mid-`cargo test --workspace`. **Recommend raising to 30 min** for full-workspace verifications.

To change: edit harness settings (out of scope for this doc — file as a separate config request if the user wants it raised). Workaround: scope test agents to `--lib` only per `feedback_ci_runs_lib_tests_only` to stay under 20 min.

### Other gotchas to watch for

- **`statusCheckRollup` stale entries** (`feedback_status_check_rollup_stale_entries`): always filter with `group_by(.name) | map(sort_by(.completedAt) | last)` to avoid acting on pre-update-branch results.
- **Local `origin/master` shadow branch** (`feedback_ambiguous_origin_master_branch`): if a worktree spawn fails wave-wide, run `git branch -a | grep master` and delete any literal `origin/master` local branch.
- **`green-ci` false positives** (`feedback_green_ci_false_positive_pattern`): trust ops bounces over green-ci verdicts; cascade-update invalidates prior CI passes.
- **Builder context exhaustion** (`feedback_builder_context_exhaustion`): for any 7+ step build, pre-split at plan-review.

---

## Estimated time-to-clean

Working assumptions: GraphQL quota healthy, no master bit-rot fire (i.e. baseline cadence, not crisis cadence).

| Phase | Hours | Cumulative |
|---|---|---|
| First 30 min (label catchup, state verify) | 0.5 | 0.5 |
| First hour (ops drain, supersedure closes, contamination check) | 1.0 | 1.5 |
| First 2 hours (architecture decision, hard rebases, C21, #6001) | 2.0 | 3.5 |
| Drain diff-audited bucket (129 PRs at ~5/batch with cascade) | 5–8 | 8.5–11.5 |
| .hermes cleanup (8 PRs, per-file inspection) | 1.5 | 10–13 |
| Bit-rot fixes (5–10 expected per cascade pattern) | 2 | 12–15 |
| New sonnet-web PR triage (variable, reserve 15%) | as-needed | n/a |

**Realistic single-session range**: 4–5 hours of focused work covers Phases 1–3 plus the first ~30 PRs of the diff-audited drain. Full clean of all 327 open PRs takes 2–3 sessions at this cadence.

**Hard stop signals**: GraphQL <500 remaining, search <10 remaining, or any agent reports `rm -rf .git` attempts (per `feedback_agent_damaged_main_checkout`). Stop and report.

---

## Sources

- `docs/forensics/2026-04-25-pr-queue-drain-session.md`
- `docs/forensics/2026-04-25-session-final-state.md`
- `CLAUDE.md` orchestration model and label-routing tables
- `docs/project/ROADMAP.md` (v0.13.0 active milestone framing)
- `~/.claude/projects/H--Code-Rust-perl-lsp/memory/MEMORY.md` recent feedback entries
