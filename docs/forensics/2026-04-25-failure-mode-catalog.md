# 2026-04-25 — Failure Mode Catalog

**Lens**: Every distinct failure mode observed during the 2026-04-25 session, with frequency and mitigation
**Purpose**: Defensive checklist — "before dispatching wave X, check that none of these failure modes are in play"

This catalog is concentrated reference, not narrative. For each failure mode: what it looked like, how often it happened, what mitigations worked.

---

## Tier 1 — High-frequency / high-impact

### F1. Per-PR fmt issues misclassified as master cascade
- **Symptom**: 3+ PRs failing identically on Compile + PR Smoke + Windows module-separator
- **Root cause**: xtask fmt aborts at first crate failure with misleading "Failed to format `<crate>/Cargo.toml`" message; same message appears across N PRs each with their own per-PR fmt drift
- **Frequency this session**: ~5 instances triggered "investigate master cascade" wave; only 7/12 flagged PRs actually had fmt drift
- **Mitigation**: verify master health (`cargo fmt --manifest-path crates/<crate>/Cargo.toml -- --check` on fresh master) BEFORE declaring cascade; if master clean for the crate but PR fails → per-PR issue; codified as `feedback_xtask_fmt_false_cascade.md`

### F2. Pre-rebuild PR branches cannot be auto-rebased
- **Symptom**: `gh pr update-branch <N>` returns "Cannot update PR branch due to conflicts"
- **Root cause**: master was rebuilt as fresh root commit `3e06aef40` on 2026-04-24 12:07 EDT; PRs branched before that timestamp share zero ancestry with current master
- **Frequency this session**: 17 PRs hit this pattern
- **Mitigation**: cherry-pick the head commit onto a fresh branch from current master, open a new PR, close the original with cross-ref; codified as `feedback_fresh_root_master_rebuild.md`

### F3. Stale labels persist after agent verdict changes
- **Symptom**: PR shows `merge-ready` + `ci-green` despite actually being CONFLICTING with FAILURE checks
- **Root cause**: labels applied during earlier agent run; subsequent CI failures didn't trigger label cleanup
- **Frequency this session**: 3 stale `merge-ready` PRs at session start (#5549/#5551/#5552); ~5-8 stale `ci-green` discovered during ops drain attempts
- **Mitigation**: ops drain agents must re-verify CI on current SHA before merging, NOT trust ci-green label alone; codified as `feedback_green_ci_false_positive_pattern.md`

### F4. GraphQL `--changedFiles` count drifts from actual diff
- **Symptom**: `gh pr view <N> --json changedFiles` returns inflated count (e.g., #6051 reported as 3943 files when actual diff is 1 file / 21 lines)
- **Root cause**: GraphQL field appears cached and lags actual diff; merge-commit content may be counted; field semantics unclear
- **Frequency this session**: maintainer-pr batch L mis-classified all 8 reviewed PRs as "branch contaminated" using this field
- **Mitigation**: for diff-size questions, use REST `gh api repos/.../pulls/N --jq '{additions, deletions, changed_files}'` (authoritative); document as memory entry candidate (gap noted in #6761)

---

## Tier 2 — Medium-frequency / medium-impact

### F5. .hermes/ cross-PR contamination
- **Symptom**: `.hermes/conveyor/work-XXXXX/` directories committed to PR with work-id NOT matching the PR's branch
- **Root cause**: Codex agent workspace files leaking across parallel PR generations
- **Frequency this session**: 8 PRs flagged (#4890, #5684, #5685, #5691, #5714, #5750, #5785, #5870); 7 confirmed cross-contamination, 1 (#5750) was self-attributed
- **Mitigation**: diff-audit must check `awk -F/ '{print $2}'` of `.hermes/conveyor/<work-id>/` against PR's branch work-id; only flag mismatches; codified in `feedback_agent_audit_trail_directories.md`

### F6. Stale CI rollup display artifacts (UNSTABLE merge state)
- **Symptom**: PR shows mergeStateStatus=UNSTABLE but latest-per-check shows all green
- **Root cause**: GitHub's mergeStateStatus aggregates all check entries including stale CANCELLED ones
- **Frequency this session**: at least 4 PRs (e.g., #6001 sandbox-fail-closed CANCELLED was a 20-min timeout artifact, latest run succeeded)
- **Mitigation**: filter `[group_by(.name) | map(sort_by(.completedAt) | last)]` before evaluating; codified as `feedback_status_check_rollup_stale_entries.md`

### F7. Agents reviewing already-closed PRs
- **Symptom**: agent applies labels or pushes commits to a CLOSED PR
- **Root cause**: agent dispatched against a query result that was current at dispatch time but stale by execution time
- **Frequency this session**: 1 confirmed (#6090 reviewed by deep-review after being closed by ensemble)
- **Mitigation**: every reviewer-style prompt should include "check PR state at start; skip if state=CLOSED"

### F8. Parallel agents collide on overlapping PR scope
- **Symptom**: two agents review the same PR independently; produce conflicting verdicts (one approves, one sends back)
- **Root cause**: ambiguous "high-value PR" criteria across multiple wave-prompt instances
- **Frequency this session**: 1-2 confirmed (#5403 maintainer-pr fixed + reviewer-deep send-back same issues)
- **Mitigation**: explicit "skip already-covered: #N1, #N2, #N3" lists in agent prompts; stagger dispatches by ~30s for ensemble vs reviewer types

### F9. Worktree branch drift on main checkout
- **Symptom**: main checkout (H:/Code/Rust/perl-lsp/) ends up on `cherry-XYZ-rebased` or `worktree-agent-XYZ` instead of starting branch
- **Root cause**: cherry-pick agent or ops agent switched the main checkout's branch to do its work
- **Frequency this session**: 1 confirmed (cherry-pick agent left main on `cherry-5695-rebased`)
- **Mitigation**: orchestrator should `git checkout <starting-branch>` after agent waves; documented in #6761 open observations as missing memory entry candidate

### F10. Cross-PR contamination of code hunks (not just .hermes)
- **Symptom**: byte-identical code change appears in 2-3 unrelated PRs
- **Root cause**: Codex thread that touched multiple files generates PR for each, all carrying the same incidental hunk
- **Frequency this session**: 2 instances (#5602/#5604/#5566 all have same xtask gemini-alias hunks; #5604 deletes Trae section that #5579 owns)
- **Mitigation**: diff-audit must flag "this hunk modifies files not mentioned in the PR title's scope"; ensemble triage should detect cross-PR-byte-identical content

---

## Tier 3 — Lower-frequency / situational

### F11. xtask CI gate registered as blocking but unconditionally fails
- **Symptom**: new CI gate fails on every PR including master CI
- **Root cause**: gate has environmental requirement (e.g., CPAN installed) that CI runner doesn't satisfy
- **Frequency this session**: 1 instance (#6230's `cpan_corpus_ratchet` gate)
- **Mitigation**: new blocking gates must be tested on current CI runner image before merge; deep-reviewer should explicitly test new gates on master not just PR branch

### F12. Ratchet-comparison without epsilon causes false failures
- **Symptom**: `clean_rate(report) < clean_rate(baseline)` fires on identical inputs across machines due to FP rounding
- **Root cause**: missing tolerance epsilon (other ratchets in same file use `+ 1e-6`)
- **Frequency this session**: 1 instance (#6230's `enforce_ratchet`)
- **Mitigation**: code review checklist for ratchet implementations: "does this have an epsilon?"

### F13. Dead code function with tests asserting it would fire
- **Symptom**: `is_ambiguous_sub_reference` written but `find_refs(key)` query path can't return refs that would reach it; tests use `expect_err` but get `Ok(...)`
- **Root cause**: builder didn't trace the data flow through workspace-index storage logic
- **Frequency this session**: 1 instance (#6053)
- **Mitigation**: deep-reviewer must trace data flow not just assert local correctness; for "is_X" predicate functions, verify a real input path exists that reaches the predicate

### F14. Off-by-one due to API confusion (`text.lines().count()` vs `bytes().filter(|b| b == b'\n').count()`)
- **Symptom**: line numbers reported one too high for any non-first-line position
- **Root cause**: `lines()` iterator counts trailing partial lines; `bytes().filter(|&b| b == b'\n').count()` is the correct line-index calc
- **Frequency this session**: 1 instance (#5368), with the regression tests that would have caught it deleted in the same PR
- **Mitigation**: deep-reviewer should be suspicious of test deletions in fix PRs ("if this test no longer applies, why?")

### F15. Test file uses array literal `[T; N]` where `Vec<T>` expected
- **Symptom**: test file fails to compile; PR description claims "all tests pass"
- **Root cause**: builder used array literal syntax for a function expecting `Vec`; in Rust, `From<[T; N]>` impl doesn't trigger at function call sites
- **Frequency this session**: 1 instance (#6379, fixed via fix-forward `0381fb6c3`)
- **Mitigation**: standards review should attempt to compile the test file (or at least visually check `[]` vs `vec![]` mismatches)

### F16. CI step timeout too tight for master baseline
- **Symptom**: CI step CANCELLED at 20-min hard timeout; master itself runs same step at 18m51s = only 6% margin
- **Root cause**: timeout set conservatively without sampling actual master runtime distribution
- **Frequency this session**: ~3 PRs hit this on Windows Guardrails (sandbox-fail-closed)
- **Mitigation**: codified as `feedback_ci_timeout_too_tight_for_master.md` — sample 5 master runs and use p95 + 30-50% margin

### F17. CI workflow trigger gap (PR-only workflow misses master regressions)
- **Symptom**: master develops a regression that only surfaces when PRs are dispatched; master CI shows green throughout
- **Root cause**: workflow has `on: pull_request:` only, never runs against master pushes
- **Frequency this session**: 1 instance (UX Regression Gate; the perl-dap UX cluster #6715 root-cause regression invisible on master)
- **Mitigation**: codified as `feedback_ci_workflow_trigger_observability_gap.md`

### F18. CRLF line endings break xargs pipelines on Windows
- **Symptom**: `<input> | xargs -I {} <cmd> {}` silently no-ops; cmd receives `<arg>\r` and fails
- **Root cause**: Git Bash on Windows emits CRLF; xargs substitutes the trailing `\r` into args
- **Frequency this session**: 1 instance (hermes sweep first 2 attempts returned zero matches before CRLF detected)
- **Mitigation**: always `tr -d '\r'` before piping to xargs on Windows; codified as `feedback_crlf_breaks_xargs_pipelines.md`

### F19. PR title with #PR-number reference instead of issue-number
- **Symptom**: validate-title check passes (because `(#NN)` exists) but auto-close fires on the wrong target after merge
- **Root cause**: builder confused PR number with issue number when filling in `(#NN)` ref
- **Frequency this session**: 6 PRs flagged by issue-linkage-health agent (#6396 self-ref, #6052 self-ref, #6140→#5387, #6138→#5386, #6133→#5388, #6051→#3729)
- **Mitigation**: validate-title should additionally check that `(#NN)` references an ISSUE, not a PR; meanwhile, manual catch in pr-create checklist

### F20. Cherry-pick agent leaves main checkout on cherry-pick branch
- **Symptom**: post-session, `git rev-parse --abbrev-ref HEAD` in main checkout shows `cherry-XXX-rebased` instead of starting branch
- **Root cause**: agent did `git checkout master` then cherry-pick, never restored
- **Frequency this session**: 1 instance (cherry-5695-rebased left in main)
- **Mitigation**: cherry-pick agents should always work in isolated worktree, not main checkout

### F21. Doc PR partially superseded by sibling PR's content extension
- **Symptom**: PR #5455 sat open with content that master `5676a2dae` had as a SUPERSET (master: 220 lines, PR: 185 lines)
- **Root cause**: doc was extended by a different PR landing on master after #5455 was opened
- **Frequency this session**: 1 instance (#5455)
- **Mitigation**: docs PRs need a periodic "is master's version of this file already a superset?" check; when superseded, close with cross-ref

---

## Tier 4 — Operational housekeeping

### F22. Locked worktrees accumulate across sessions
- **Symptom**: `git worktree list` shows 173+ entries from prior sessions
- **Root cause**: agents killed by quota limits before wrap-up step ran; locks remain
- **Frequency this session**: persistent — ~173 entries observed
- **Mitigation**: pre-session cleanup pass with selective `git worktree remove --force` for orphaned (PID-not-running) entries

### F23. Multiple worktrees competing for `master` branch checkout
- **Symptom**: `git worktree add <path> master` fails with "master is already used by worktree at <other-path>"
- **Root cause**: prior session's worktree never released master; new worktree creation fails
- **Frequency this session**: 1 instance (had to use `git worktree add -b <new-branch> <path> origin/master`)
- **Mitigation**: always create worktrees on a NEW tracking branch from origin/master, never on `master` directly

### F24. GitHub secondary rate limit (abuse detection) triggers before primary quota
- **Symptom**: `gh pr view` calls fail with abuse detection long before primary 5000/hr quota exhausted
- **Root cause**: parallel `gh pr view` salvos from multiple agents trip secondary throttling
- **Frequency this session**: ~3 instances during high-parallelism waves
- **Mitigation**: agents should sequentialize their `gh pr view` calls (don't `xargs -P`); REST API has separate quota pool when this happens

### F25. Org-level Anthropic monthly cap blocks all sub-agent dispatch
- **Symptom**: every sub-agent returns "You've hit your org's monthly usage limit" within seconds of dispatch
- **Root cause**: hard organizational cap on Anthropic API spend
- **Frequency this session**: 1 instance (mid-Wave 7); recovered when weekly reset hit ~9 minutes later
- **Mitigation**: orchestrator should fall back to direct API ops (gh CLI) when sub-agents are blocked; the cap doesn't affect orchestrator's own tool use

---

## Defensive checklist (use before dispatching a wave)

Run through this checklist before each wave to avoid known failure modes:

1. **Master state**: is master CI green on the latest SHA? (F11)
2. **Quota**: how much Anthropic/GitHub quota remains? (F25, F24)
3. **Worktree availability**: are unlocked worktree slots available? (F23)
4. **Stale labels**: any PRs with merge-ready but no ci-green? (F3)
5. **Stale rollup**: any UNSTABLE PRs that latest-per-check would clear? (F6)
6. **Pre-rebuild PRs in target set**: any PRs older than 2026-04-24 12:07 EDT? (F2)
7. **Cluster overlap**: are different agent waves about to touch the same PR set? (F8)
8. **Closed-state risk**: did any prior wave close PRs the new wave will pick up from queries? (F7)
9. **Policy clarity**: are policies on .hermes attribution, scope drift, conservative close pre-elicited? (F5, anti-pattern from #6763)
10. **Branch drift**: is main checkout on the expected starting branch? (F20)

---

## Cross-references

- Sibling docs: #6757 (economics), #6761 (process meta), #6763 (orchestration anatomy), #6764 (repo direction)
- Memory entries that codify many of these: see `feedback_xtask_fmt_false_cascade.md`, `feedback_fresh_root_master_rebuild.md`, `feedback_status_check_rollup_stale_entries.md`, `feedback_agent_audit_trail_directories.md`, `feedback_ci_timeout_too_tight_for_master.md`, `feedback_ci_workflow_trigger_observability_gap.md`, `feedback_crlf_breaks_xargs_pipelines.md`, `feedback_green_ci_false_positive_pattern.md`, `feedback_label_skill_silent_failure.md`
- Open questions about these: see `2026-04-25-open-questions-research-backlog.md` (sibling)
