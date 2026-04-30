# Failure Modes Reference

Recurring operational failure patterns for the perl-lsp pipeline. Each entry is a named pattern with detection heuristics and mitigation steps. These are operating conditions, not crises.

Memory references point to `C:\Users\<user>\.claude\projects\H--Code-Rust-perl-lsp\memory\` for agents with access to that store; external contributors can use this doc directly.

---

## Coordination Failures

### Collision Cascade

**Symptom**: Multiple open PRs for the same issue, often from the same Codex generation run. Triage agents see N "winner candidates" and have to deduplicate.

**Root cause**: Codex generates 3–5 variants per prompt (the ensemble pattern). Without a pre-merge deduplication step, all variants enter the review pipeline simultaneously and compete for the same merge slot.

**Detection**: `gh pr list --search "closes #<issue>" --state open` returns >1 result. Titles share a prefix. File-path overlap is high across diffs.

**Mitigation**: Cluster-triage: pick the winner by file-path coverage, extract unique edge cases from losers (tests, impl ideas), close losers with cross-refs. See salvage classifier doctrine.

**Prevention**: Rate-limit Codex fanout to 4 per prompt. Run `ensemble-detect` before dispatching curator. Triage within 24h of generation before the PR pool deepens.

**Memory**: `feedback_codex_ensemble_pattern.md`, `project_salvage_classifier_doctrine.md`

**Example PRs**: #5631–5634 (OpenClaw hallucination cluster), #6853 sub-task cluster (three curators flagged same orphan change across #6868, #6873, #6891)

---

### Stale Base Cascade

**Symptom**: A batch of recently-queued PRs all fail merge due to conflict or stale base after master moves.

**Root cause**: Master advances (new merges, a master fix PR, or a history rewrite). Open PRs retain their old base SHA. `gh pr update-branch` calls propagate the fix, but PRs opened during the window don't get refreshed automatically.

**Detection**: Ops reports "NOT MERGED — conflict" on PRs that were clean yesterday. `git log master..HEAD` on a stale branch shows commits that no longer exist on master.

**Mitigation**: After each master fix: `gh pr update-branch <N>` for every blocked PR. Don't rebase unless there is an actual merge conflict — update-branch is cheaper and sufficient.

**Prevention**: Merge in batches of ≤3. Between batches, run cascade-update. For history rewrites, all pre-rewrite PRs require cherry-pick to a new branch (rebase produces a poisoned diff).

**Memory**: `feedback_fresh_root_master_rebuild.md`, `feedback_gh_run_rerun_stale_context.md`, `feedback_merge_conflict_cascade.md`

**Example PRs**: #5679 → #6716 (cherry-pick after master rebuild on 2026-04-24)

---

### Worktree Branch Confusion

**Symptom**: Agent pushes to the wrong branch; PR diff contains commits from a different PR or from master.

**Root cause**: Three failure modes — (1) spec-planner branches from the main checkout's current HEAD rather than origin/master; (2) cross-PR commit bleed when two concurrent agents share a branch lineage; (3) agent's worktree is empty/pruned and writes fall through to the main checkout.

**Detection**: `git log --oneline master..HEAD` on the PR branch shows unexpected commits (commits not authored by this agent, or commits from a closed PR). PR diff stat is abnormally large for the stated scope.

**Mitigation**: Every spec-planner must run `git checkout -b <branch> origin/master` explicitly. Before opening a PR, run `git log --oneline master..HEAD` and fail if the count is wrong. If the worktree appears empty, stop — do not write to main checkout as fallback.

**Prevention**: Preflight check 2 (worktree isolation) catches the empty-worktree case. Orchestrator should verify `git worktree list` and restore main checkout after each wave with `git restore`.

**Memory**: `feedback_concurrent_worktree_contamination.md`, `feedback_worktree_file_leak.md`, `feedback_nested_worktree_main_switch.md`

---

### Dirty Tail PR Salvage

**Symptom**: An older PR has failing CI, merge conflicts, or a stale base — but contains real signal (correct implementation shape, useful tests, an edge case no fresh candidate covers).

**Root cause**: PRs age out of sync with master during high-throughput merge waves. The PR is not worthless, but it can't auto-merge.

**Detection**: `gh pr view <N>` shows conflict, stale-base CI, or large diff relative to PR purpose. The issue body confirms the work is still valid. No cleaner sibling exists.

**Mitigation**: Run the salvage classifier (rescue cost vs reimplementation cost). Options: salvage-rebase (small conflicts), salvage-cherry-pick (stale branch, clean patch), salvage-extract-tests / extract-impl (conflicted PR, valuable contents). Close only when premise is obsolete, architecture is wrong, or a better sibling already exists.

**Prevention**: Classify dirty PRs within 48h of going stale. Blind rebase is cheaper in the short term but tends to produce junk diffs. Cherry-pick is the safer default.

**Memory**: `project_salvage_classifier_doctrine.md`, `feedback_fresh_root_master_rebuild.md`, `feedback_windows_worktree_rebase_hangs.md`

---

## State Failures

### Label Rot / Stale Routing Flag

**Symptom**: A `needs-*` label persists on a PR or issue after the actual condition is resolved. Ops skips the PR as "has unaddressed work." Routing agents loop back unnecessarily.

**Root cause**: The label state machine is append-only via agent actions. No agent is responsible for cleaning up stale `needs-builder-fix` / `needs-ci-fix` / `in-build` flags when the underlying work completes.

**Detection**: PR has `deep-reviewed` + `merge-ready` + `needs-builder-fix` simultaneously — contradictory pair. Or: issue labeled `in-build` with no open PR linked and no commit activity in 7+ days.

**Mitigation**: For contradictory label pairs where timeline-based "later applied wins" applies (deep-review, diff-audit, maintainer-pr, review): strip the stale flag. For CI-pair (`ci-green` vs `needs-ci-fix`): query live `statusCheckRollup`; if green, strip the stale flag; if red, live CI wins regardless.

**Prevention**: Reconciler (#7085 addresses) grounds CI-pair decisions in live state. Session-start sweep: `gh issue list --label "in-build" --state open` filtered to >7 days with no open PR. Orchestrator bulk-applies missing labels after verifying agent self-reports with `gh pr view --json labels`.

**Memory**: `feedback_label_skill_silent_failure.md`, `feedback_stale_inbuild_claims.md`, `feedback_live_signals_vs_label_signals.md`

**Example PRs**: #5365/#5353 (stale `needs-deep-review` from 4 days before valid `deep-reviewed`); 10 stale `in-build` issues found in 2026-04-11 session

---

### Stale Signoff

**Symptom**: A PR carries `review-reviewed`, `deep-reviewed`, or `diff-audited` from a pass that predates the current HEAD SHA. The label is stale; the reviewed diff is not the current diff.

**Root cause**: A cascade-update, a reviewer push-fix, or a pr-responder commit lands after the sign-off label was applied. The label does not track the SHA it was applied to.

**Detection**: Compare `headRefOid` from `gh pr view --json headRefOid` against the SHA visible in the last sign-off comment. Mismatch = stale. Also: any `gh pr update-branch` call invalidates prior ci-green.

**Mitigation**: Strip `ci-green` and `merge-ready` after any branch update. Re-run green-ci before promoting. For review signoffs: if the diff since the review is purely mechanical (fmt, comment), the signoff is still valid; if it's substantive, re-run the relevant gate.

**Prevention**: green-ci agents must read `headRefOid` before declaring green. Diff-auditor and deep-reviewer should note the SHA in their verdict comment to make staleness detectable.

**Memory**: `feedback_green_ci_false_positive_pattern.md`, `feedback_status_check_rollup_stale_entries.md`

**Example PRs**: #6447, #6355, #6351 (green-ci applied ci-green on stale SHA; ops caught all three on 2026-04-25)

---

### Label-Skill Silent Failure

**Symptom**: Agent reports "label X set" in its verdict but the label never lands on the PR. Ops finds no merge-ready candidates despite many sign-off rounds. Pipeline appears stalled.

**Root cause**: `gh pr edit --add-label` calls from inside worktree agents fail silently — likely due to repo-context mismatch, label-namespace conflicts, or transient network errors. The API call returns without error but the label isn't applied.

**Detection**: After a wave of agent returns all claiming "label set," check 3–5 PRs directly: `gh pr view <N> --json labels -q '.labels[].name'`. If claimed labels are absent across >30% of the sample, silent failure is active.

**Mitigation**: Orchestrator bulk-applies missing labels from outside worktree agents: `gh pr edit <N> --add-label "review-reviewed,deep-reviewed"`. Never trust agent self-reports for label state — verify periodically.

**Prevention**: Orchestrator runs periodic spot-checks every 10–20 agent returns. Treat "ops says 0 ready candidates" as a trigger to verify label state, not pipeline correctness.

**Memory**: `feedback_label_skill_silent_failure.md`

**Example PRs**: PRs #6219–6227 (security cluster): ~80% silent failure rate on `maintainer-pr-reviewed` and `diff-audited` labels in 2026-04-24 session

---

## CI Failures

### Master Bit-Rot Cascade

**Symptom**: 3+ unrelated PRs fail the same CI gate identically within a short window. The failure looks like a PR-side issue but reproduces on fresh `origin/master`.

**Root cause**: A merged PR introduced a formatting drift, compile error, duplicate method, or bad test that breaks the workspace-wide gate. Every open PR inherits the failure once it rebases.

**Detection**: Same error message across N PRs. Error reproduces when running the gate command locally against `origin/master` (not a PR branch). Key commands: `cargo xtask fmt --check` (fmt drift), `cargo test --workspace --lib --locked` (test failures), `cargo check --all-targets` (compile errors).

**Mitigation**: Fix on master in a narrow PR (≤6 lines). Admin-merge once local verification passes — don't wait for full CI on the fix PR. Then: `gh pr update-branch` for every blocked PR. Do not try to fix N PRs individually; fix master once.

**Prevention**: Cluster serialization for ≥8-PR Codex bursts (merge serially, not in batches). Post-cluster fmt PR absorbs residual drift. Duplicate-name check before merging clusters touching the same crate. CI must include `on: push: branches: [master]` to detect landed regressions.

**Memory**: `feedback_master_bit_rot_cascade_fixes.md`, `feedback_master_bit_rot_recurrence_pattern.md`, `feedback_master_bitrot_cascade_8plus_pattern.md`

**Example PRs**: #5749, #5751/#5783, #5965, #5986 (2026-04-24: 4 master fixes, unblocking 60+ PRs); #6451–6461 (2026-04-25: 8 master fixes in 3.5 hours during 16-PR perl-token cluster)

---

### xtask fmt False Cascade

**Symptom**: Multiple PRs fail identically on Compile + PR Smoke + Windows Guardrails (module-separator-regressions). Looks like a single upstream master failure but is actually N independent PR-side format issues.

**Root cause**: `cargo xtask fmt` aborts at the first failing crate and emits a misleading "Failed to format <crate>/Cargo.toml" message regardless of the actual problem. N PRs each with their own unformatted file produce visually-identical output at the CI level.

**Detection**: Before declaring master cascade, verify on master: `cargo fmt --manifest-path crates/<crate>/Cargo.toml -- --check`. If master is clean for the crate but the PR fails, it is a PR-side issue. If master fails too, it is a cascade.

**Mitigation**: Fix per-PR by running `cargo xtask fmt` on the branch and pushing. Do NOT push a no-op fmt commit to a PR that doesn't have a fmt issue — it obscures the real failure.

**Prevention**: Green-CI agents must verify master health before declaring cascade. When the failure pattern matches xtask-fmt-abort signature (Compile + PR Smoke + Windows module-separator), check master first. Individually verify each PR in a suspected cluster (2026-04-25 calibration: only 7/12 flagged PRs actually had fmt issues).

**Memory**: `feedback_xtask_fmt_false_cascade.md`, `feedback_master_bit_rot_recurrence_pattern.md`

**Example PRs**: #6391, #6375, #6355, #6157, #6126, #5728, #5687, #5898, #5881, #6428, #6395, #6159 (2026-04-25 session — 12 PRs each with their own fmt issue, appeared as one cascade)

---

### Master Test Panic Blocker

**Symptom**: Master CI `unit_core` / `unit_full` exits with code 101 (Rust panic). Every open PR's CI Gate fails after rebasing. The queue is blocked for hours or days.

**Root cause**: A test bug on master — typically a logic error in test setup (e.g., variable shadowing a `tempfile::tempdir()` binding, causing the test to inspect an empty directory and assert a non-zero count). The test compiles and appears to execute but panics at the assertion.

**Detection**: Reproduce locally against `origin/master` (not a PR branch): `cargo test -p perl-parser -p perl-lexer -p perl-parser-core --lib --locked -- --test-threads=4` (unit_core), `cargo test --workspace --lib --locked --exclude tree-sitter-perl` (unit_full). GitHub log truncation hides the panic site; local repro is required to find the exact test name.

**Mitigation**: Fix the test on master. Admin-merge once local verification passes. Cascade-update all blocked PRs.

**Prevention**: Red-TDD and green-TDD skills should scan for shadowed `tempfile::tempdir()` and similar resource-binding shadows. When ≥3 PRs fail an identical CI gate after rebase: reproduce locally on `origin/master` before trying to fix any individual PR. Fixing only fmt and assuming done is insufficient — verify both `unit_core` and `unit_full` independently.

**Memory**: `feedback_master_test_panic_blocks_queue.md`, `feedback_master_bitrot_cascade_8plus_pattern.md`

**Example PRs**: PR #5985 (commit 2b827cc5c3, variable shadow in `test_cleanup_respects_retention_count`) — blocked queue for ~24h; investigated via #7031

---

### CI Cancellation Cascade

**Symptom**: Rapid back-to-back master merges (3+ in quick succession) cancel each other's CI runs. PRs that passed locally never get a complete CI result on master. The queue appears partially green but is actually inconclusive.

**Root cause**: GitHub Actions cancels in-progress runs when a new push arrives on the same branch. During a burst merge phase, every merge cancels the previous run. The net result: no single master SHA has a full green CI pass.

**Detection**: `gh run list --branch master` shows multiple CANCELLED runs in rapid succession. No successful `CI Gate (Merge-Blocking)` run exists for the latest master SHA.

**Mitigation**: Merge in batches of ≤3. Wait for CI to complete between batches. Do not ops-merge while a cancellation cascade is in progress.

**Prevention**: Merge batch size of 3 is the documented protocol. Enforcing it prevents cascades. During high-throughput cluster merges (≥8 PRs), plan for longer inter-batch waits.

**Memory**: `feedback_master_bitrot_cascade_8plus_pattern.md` (section: cascade timing)

---

### Workflow Trigger Feedback Loop

**Symptom**: A workflow that manages labels (e.g., a reconciler that strips stale flags) loops infinitely because it triggers on `labeled` / `unlabeled` events, and its own label changes re-trigger it.

**Root cause**: GitHub Actions workflows with `on: pull_request: types: [labeled, unlabeled]` fire whenever any label changes, including changes made by the workflow itself.

**Detection**: Workflow run history shows the same workflow firing in rapid succession (several runs per minute) with no external user action. Each run modifies a label, which triggers the next run.

**Mitigation**: Guard the workflow body: check whether the triggering label is one the workflow itself manages; if so, skip. Or scope the workflow to specific label names via `if: github.event.label.name == 'X'`.

**Prevention**: When designing label-management workflows, explicitly test the trigger loop case. Avoid `types: [labeled, unlabeled]` without a label-name guard.

**Memory**: (pattern from #7085 reconciler fix-forward on 2026-04-27)

---

### Workflow PR-Only Trigger Observability Gap

**Symptom**: A quality gate (e.g., UX Regression Gate, snapshot comparison, performance benchmark) shows "no change" on every new PR even while master is regressed. The gate appears healthy but is comparing regression-to-regression.

**Root cause**: The workflow is declared with `on: pull_request:` only. It never runs on master push events. After a regression merges, the baseline becomes the regressed state. Every subsequent PR compares against the bad baseline and reports "no change."

**Detection**: No master-branch runs exist in the workflow's Actions history. `grep -L 'push:' .github/workflows/*.yml | xargs grep -l 'pull_request:'` identifies PR-only workflows.

**Mitigation**: Add `on: push: branches: [master]` to the workflow. The master run publishes the baseline artifact; subsequent PRs compare against true master state.

**Prevention**: Audit all quality-comparison workflows for `on: push: branches: [master]`. Any gate that asserts a quality bar relative to a stored baseline needs a master push trigger or it cannot detect baseline rot.

**Memory**: `feedback_ci_workflow_trigger_observability_gap.md`

---

### rtk gh pr checks Masks Aggregator Failure

**Symptom**: `rtk gh pr checks <PR>` reports "Passed: N, Failed: 0" but the PR is actually blocked. The CI Gate (Merge-Blocking) aggregator is FAILURE on the latest SHA.

**Root cause**: `rtk`'s filter counts individual job conclusions and may drop or suppress the aggregator result. The aggregator is a single job that rolls up many sub-checks; if only the aggregator is red and all sub-checks are individually SUCCESS/SKIPPED, `rtk` may count 0 failures while the real gate is blocked.

**Detection**: PR has `mergeStateStatus: UNSTABLE` in `gh pr view --json mergeStateStatus`. Direct rollup check: `gh pr view <PR> --json statusCheckRollup -q '.statusCheckRollup | group_by(.name) | map(sort_by(.completedAt) | last) | map(select(.conclusion == "FAILURE" or .conclusion == "CANCELLED" or .status == "IN_PROGRESS"))'`.

**Mitigation**: Never trust `rtk gh pr checks` for promote/merge decisions. Use raw rollup query above. A PR with full sign-off chain + `UNSTABLE` mergeStateStatus + rtk reporting "0 failed" should always trigger a raw rollup re-check.

**Prevention**: Ops agents must use direct rollup queries, not rtk summaries, for merge-readiness decisions. Document this in the ops prompt: rtk is for human-readable summaries, not merge gates.

**Memory**: `feedback_rtk_pr_checks_masks_failures.md`

**Example PRs**: #7016 (rtk reported "Passed: 14, Failed: 0"; raw rollup showed `CI Gate FAILURE` on latest SHA)

---

## Code-Generation Failures

### Codex Framework Hallucination

**Symptom**: One or more PRs teach the codebase to recognize a named Perl framework that has no CPAN presence — adding entries to `WebFrameworkKind`, `IMPLICIT_STRICT_MODULES`, `COMMON_MODULES_TIER_1`, `PERL_SOURCE_EXTENSIONS`, or similar registries. Code quality is high, tests pass, clippy is clean.

**Root cause**: Codex conflates AI-product names, JavaScript tools, and C++ game engines with Perl frameworks. It generates 3–4 reinforcing PRs per hallucination, each adding a different layer of the "framework" support. The coherent multi-PR shape makes it look like a real feature on quick review.

**Detection**: Before approving any PR that adds entries to `WebFrameworkKind`, `IMPLICIT_STRICT_MODULES`, `IMPLICIT_EXPORT_SKIP_LIST`, `COMMON_MODULES_TIER_1`, or `PERL_SOURCE_EXTENSIONS`: `curl -s "https://fastapi.metacpan.org/v1/module/_search?q=<NAME>&size=5"`. Zero results = hallucination. If the name is an AI product or tool, the PR almost certainly belongs in `docs/EDITORS/` as setup documentation.

**Mitigation**: Close with "Codex hallucination — no CPAN distribution." If a legitimate fix uses hallucinated module names in test fixtures, replace the fixtures and keep the fix logic.

**Prevention**: MetaCPAN verification is a required check for any PR adding to the registries above. Research-verifier pre-filter catches this at the first pass. Hallucination-check skill runs MetaCPAN verification as part of ensemble curation.

**Memory**: `feedback_codex_framework_hallucination.md`

**Example PRs**: #5631–5634 (OpenClaw — C++ game remake, 4 PRs); #5619, #5641 (Droid/Factory.ai agent); #5627–5630 (Builder::IO::Fusion — builder.io JS tool); #5592 (Google::Antigravity)

---

### Codex Phantom Scope Drift (Orphan File Pattern)

**Symptom**: A Codex cluster PR contains a change to a file unrelated to its stated scope. The change appears in 25–30% of variants from the same generation run, is plausible-looking, and is consistent across PRs (same file, same line area).

**Root cause**: Codex bundles a "free" cleanup edit into a subset of sub-task variants during multi-shot generation. The edit survives author review because it looks reasonable in isolation but is unambiguously out of scope relative to the issue body.

**Detection**: For each PR in a Codex cluster, check: `gh api repos/.../pulls/<N>/files | grep -i '<suspected-file>'`. If the same file appears across ≥2 sibling PRs and the issue doesn't mention that file's crate, it's orphan content.

**Mitigation**: Flag as SCOPE_DRIFT. Do not merge the orphan change with the cluster winner. If the orphan change has merit, file it as its own narrow PR.

**Prevention**: Cluster curators run a file-path diff across siblings before picking a winner. Known orphan: `crates/perl-dap/src/eval/validator.rs` near line 89 (appeared in #6868, #6873, #6891 — three independent curators, one sprint).

**Memory**: `feedback_perl_dap_validator_scope_drift_pattern.md`, `feedback_codex_framework_hallucination.md`

**Example PRs**: #6868, #6873/#6874, #6891/#6892 (three sub-task clusters from #6853 sprint, all carrying the same unrelated `validator.rs` change)

---

### Scope Drift with Conventional-Commit Mismatch

**Symptom**: A PR titled `docs:` or `test:` ships multi-crate production-source changes (hundreds or thousands of lines of code). The four-signoff chain completes without catching it because diff-audit checks for foreign agent artifacts, not for title-vs-diff-weight mismatches.

**Root cause**: Diff-auditor checks for contamination (wrong `.hermes/` subdir, cross-PR spec leaks) but lacks a rule comparing the conventional-commit type against diff scope. A `docs:`-titled PR adding 3000+ lines of production code is mechanically a scope mismatch even if no individual file is "foreign."

**Detection**: Parse the PR title's conventional-commit type. Sample diff stats: `gh api repos/.../pulls/<N>/files`. Flag:
- `docs:` → expect <30 lines, only docs/markdown/comment files
- `test:` → expect changes confined to `crates/*/tests/`, `tests/`, `benches/`, snapshots
- `chore:`, `ci:` → expect tooling/config only, no production-source changes

**Mitigation**: Post SCOPE_DRIFT verdict and apply `needs-diff-fix` regardless of individual file-contamination status. The title is the contract.

**Prevention**: Diff-auditor skill should include title-vs-diff-size sanity check as a first-pass rule before individual file inspection.

**Memory**: `feedback_diff_audit_title_diff_size_check.md`

**Example PRs**: #5543 (`docs(work-00ef571c): resolve metric drift` — 24 files, 3082 additions covering UX gold scorecards, async-await tests, and code-action infrastructure; passed full 4-signoff; caught by salvage classifier)

---

### Cross-PR Audit-Trail Contamination

**Symptom**: A PR's diff contains `.hermes/`, `.spec/`, `.jules/`, or `.codex/` content from a different issue than the one this PR addresses. Diff-audit may pass it as "agent's own trail" if it only checks author-agent match and not subdir-name match.

**Root cause**: When multiple Codex or Hermes tasks run concurrently, artifacts from one task's working directory can bleed into another task's PR via shared filesystem paths or checkout confusion.

**Detection**: `gh pr diff <N> --name-only | grep -E '^\.(hermes|jules|spec|codex|run)/'` lists agent-dir entries. For each: does the directory name reference THIS PR's issue number or feature? Mismatches are cross-PR leaks.

**Mitigation**: Strip the foreign audit-trail content (revert those files) before merging. The PR's own audit-trail dir (matching its issue number) is always kept.

**Prevention**: Diff-audit must apply cross-PR-leak check, not just agent-author check: `awk -F/ '{print $2}'` on the listed dirs, compare against this PR's issue number. Mismatch → SCOPE_DRIFT.

**Memory**: `feedback_agent_audit_trail_directories.md`, `feedback_spec_folders_are_history.md`

**Example PRs**: 8 PRs from 2026-04-25 session carried `.hermes/` artifacts from different issues; all had been labeled `diff-audited` (clean) because the gate checked author-match but not subdir-issue-match.

---

## See Also

- [ORCHESTRATION_DOCTRINE.md](ORCHESTRATION_DOCTRINE.md) — pipeline design principles
- [PIPELINE_GATES.md](PIPELINE_GATES.md) — full gate model with skip criteria
- [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md) — label vs live-CI distinction
- [PROCESS_LESSONS.md](PROCESS_LESSONS.md) — adjacent operational lessons
