# 2026-04-25 — Memory Entry Effectiveness Review

**Lens**: Which memory entries actually fired and shaped behavior this session, which were dormant, and which were missing
**Purpose**: Inform a future memory-consolidation pass

The auto-memory system has ~75 entries at session end. This doc audits which entries earned their place by firing this session, which sat unused, and where new entries needed to be created mid-session because existing ones didn't cover the case.

---

## Entries that fired and proved their value

| Entry | How it fired | Without it would have happened |
|---|---|---|
| `feedback_status_check_rollup_stale_entries.md` | Used "latest-per-check" filter `[group_by(.name) | map(sort_by(.completedAt) | last)]` in every green-CI agent prompt | Would have read stale CANCELLED entries as live failures |
| `feedback_xtask_fmt_false_cascade.md` (updated mid-session) | Master-bit-rot scout used the disambiguation logic to differentiate per-PR fmt vs real master cascade | Would have dispatched master-fix wave instead of per-PR fmt-fix wave |
| `feedback_codex_ensemble_pattern.md` | Drove the ensemble-curator dispatch pattern (read all 3-5 PRs, pick best, close losers with cross-ref) | Would have processed Codex bursts one-PR-at-a-time, missing the deduplication win |
| `feedback_master_bit_rot_recurrence_pattern.md` | Triggered the cluster investigation when 8 perl-dap PRs shared identical UX failure | Would have routed each PR individually to reviewer instead of filing tracking issue #6715 |
| `feedback_label_skill_silent_failure.md` | Every agent prompt included "VERIFY label landed via gh pr view --json labels" | Multiple labels would have silently not applied |
| `feedback_reweigh_prior_comments.md` | incremental_document.rs investigation correctly identified the codex bot comment as bound to a stale SHA | Would have spent agent time fixing a non-existent bug |
| `feedback_spec_folders_are_history.md` | Diff-audit prompts explicitly excluded `.spec/` from scope-drift flagging | Would have generated false-positive needs-diff-fix on PRs with cumulative spec folders |
| `feedback_concurrent_worktree_contamination.md` | Drove `git log master..HEAD` checks before opening cherry-pick PRs | Would have included unrelated commits in cherry-picks |
| `feedback_pre_push_hook_windows_race.md` | Cherry-pick agent used `HEAD:refs/heads/<branch>` push form | Would have hit Windows file-lock race |
| `feedback_deep_review_bug_catch_roi.md` | Justified the reviewer-deep dispatch budget despite cost | Would have been tempted to skip deep-review on "test-only" or "doc-only" PRs |

**Score**: 10+ entries actively shaping behavior. These are load-bearing.

---

## Entries that were dormant this session

These exist in memory but didn't fire because the conditions didn't arise:

| Entry | Why dormant |
|---|---|
| `feedback_publish_pipeline_gotchas.md` | No publish operations this session (release work paused) |
| `feedback_publish_allowlist_maintenance.md` | Same — no release |
| `feedback_dap_scout` related entries | No dedicated DAP scout dispatch (DAP work was via reviewer-deep on existing PRs) |
| `project_v013_structural_gaps.md` | Lists 3 focus areas (symbol visibility, parser error recovery, pragma tracker) — pragma tracker work happened (#6347, #6351, #6355) but the entry didn't directly drive it |
| `feedback_wave1_collapse_gotchas.md` | Wave 1 collapse already merged; entry covers what NOT to do, didn't trigger |
| `feedback_capacity_changes_defer_semantics.md` | DEFER decisions weren't a focus this session |

**Verdict**: dormant doesn't mean useless. Most are load-bearing for situations that happen monthly, not per-session. Don't archive on dormancy alone.

---

## Entries that should have existed but didn't (created mid-session as gaps surfaced)

This session created 5 new memory entries because existing entries didn't cover the case:

| New entry | Gap it filled |
|---|---|
| `feedback_xtask_fmt_false_cascade.md` (created earlier this session, updated again) | The fmt-cascade-vs-master-cascade disambiguation pattern wasn't codified |
| `feedback_fresh_root_master_rebuild.md` | The "master rebuilt as fresh root commit; pre-rebuild PRs need cherry-pick" pattern was new (had only happened once before, not coded) |
| `feedback_crlf_breaks_xargs_pipelines.md` | The Windows xargs CRLF issue was discovered live; no prior entry |
| `feedback_ci_workflow_trigger_observability_gap.md` | "PR-only workflows miss master regressions" pattern wasn't codified |
| `feedback_ci_timeout_too_tight_for_master.md` | "Sample 5 master runs and use p95 + margin" heuristic wasn't codified |
| `feedback_triage_at_scale_validates_ensemble.md` | Validation of the ensemble economics needed a dedicated entry |

**Updated entries**: `feedback_agent_audit_trail_directories.md` got the `.hermes` attribution policy (work-id matching) added.

**Score**: 6 new + 1 update = 7 memory entries gained from this session. ~10% growth.

---

## Entries that should exist but still don't (gaps surfaced but not coded)

These are gaps the session surfaced but didn't get codified yet:

1. **"Verifier-of-verifier pattern for high-blast-radius verdicts"** — when a cheap-model agent's verdict could close PRs or strip critical labels, dispatch a second agent to verify the claim before acting. Worth a feedback entry.

2. **"Always restore main checkout to its starting branch at session end"** — sub-agents drift the main checkout's branch (cherry-5695-rebased example). The `feedback_nested_worktree_main_switch.md` covers the risk but doesn't prescribe the restore step.

3. **"Stale GraphQL changedFiles count vs REST authoritative"** — saw 8 PRs misclassified as "branch-contaminated" because of GraphQL field staleness. Worth an entry: "for diff-size questions, use REST `gh api repos/.../pulls/N --jq '{additions, deletions, changed_files}'`, not GraphQL `--changedFiles`".

4. **"Agent finishes work but cannot push" recovery pattern** — the #6447 sortText fix was produced by an agent that couldn't push to the right branch. Saved to /tmp during cleanup. Worth an entry: "if an agent produces a fix it can't push, save the patch to /tmp with descriptive name and document in PR comment so the orchestrator or next builder can apply it".

5. **".hermes attribution policy" as a standalone entry** — this is currently buried in `feedback_agent_audit_trail_directories.md` as an update. May deserve its own entry given how often it came up this session.

6. **"Promotion sweeps need iteration"** — sign-offs land asynchronously across waves. The pattern is documented in #6763 (orchestration anatomy) but should be a feedback entry too.

7. **"Direct orchestrator API ops as cheap finisher"** — also in #6763 as a process pattern but worth its own entry.

8. **"Op cluster batched-fix instead of per-PR fixes"** — when 5+ PRs share a root cause (e.g., perl-dap UX cluster #6715), file tracking issue and fix upstream rather than per-PR. Generalizable pattern.

---

## Memory consolidation opportunities

Existing entries that overlap and could merge:

| Candidates for merging | Reason |
|---|---|
| `feedback_master_bit_rot_recurrence_pattern.md` + `feedback_master_bitrot_cascade_8plus_pattern.md` + `feedback_xtask_fmt_false_cascade.md` | All three are about master-cascade-detection variants. Could be one entry with three sub-cases. |
| `feedback_concurrent_worktree_contamination.md` + `feedback_swarm_worktree_contamination.md` + `feedback_worktree_file_leak.md` | All worktree contamination variants. |
| `feedback_label_skill_silent_failure.md` + `feedback_label_state_machine.md` (if exists) + label-related entries | Label hygiene is a single topic spread across entries. |
| `feedback_codex_ensemble_pattern.md` + `feedback_triage_at_scale_validates_ensemble.md` | Same lesson, different vintages. |
| `feedback_status_check_rollup_stale_entries.md` + `feedback_green_ci_false_positive_pattern.md` | Both about CI signal interpretation. |

**Consolidation principle**: combine when the entries share a *root cause* (not just a *symptom*). The 3 master-cascade entries share root cause = "downstream signal is identical-looking but root cause varies by failure type". The 3 worktree entries share root cause = "shared filesystem state across agents".

---

## Recommendation for next session's memory pass

1. Run a dedicated memory consolidation agent with full quota allocation (the parallel attempt this session ran out)
2. Specifically: merge the 5 candidate-for-merging clusters identified above
3. Add the 8 missing-entries identified in the gaps section
4. Archive entries that haven't fired in 30+ days (dormant + irrelevant, not just dormant)
5. Verify all `project_*` entries are still accurate (some reference completed initiatives that should be marked done)

Outcome target: ~75 entries → ~50 consolidated entries with broader coverage. Less to scan, more to retrieve correctly.
