# Memory Recalibration Pass 001 — 2026-04-25

**Pass type:** First post-substrate-shift sweep (Codex 5.4 → 5.5 launched 2026-04-24, ChatGPT-Pro + GitHub repo connector now in upstream-research loop).
**Trigger:** Operator-dispatched substrate-shift recalibration.
**Procedure:** Inventory → staleness detection → mechanical fixes → consolidation suggestions, per `.claude/agents/memory-recalibrator.md`.
**Authoring conventions:** `2026-04-25-forensics-and-memory-authoring-conventions.md`.

---

## Substrate context

| Layer | State at recalibration |
|---|---|
| Upstream generation | Codex 5.5 (replaced 5.4 on 2026-04-24) |
| Upstream research | ChatGPT-Pro web threads + GitHub repo connector |
| Downstream | Anthropic-only — Sonnet for deep-review/plan-review/build, Haiku for ladder |
| Configured-but-inactive | GLM, Fireworks, Minimax, OpenCode (all plans configured, none yet drawing dispatch) |

Most existing forensics/memory calibration numbers were measured against a Codex-5.4 + no-pre-planning substrate. Per `2026-04-25-substrate-shift-and-two-timescale-calibration.md`, those numbers are wrong-by-default until re-measured.

---

## Inventory

| Scope | Count | Notes |
|---|---:|---|
| `docs/forensics/` total entries | 47 | includes the older PR-archaeology subsystem (INDEX.md, README.md, pr-*.md, prompts/, calibration/) which is orthogonal to the prompt-fragment architecture |
| `docs/forensics/2026-04-*.md` (substrate-relevant) | 25 | the prompt-fragment forensics docs |
| `docs/forensics/dispatch-index.toml` situations | 21 | all referenced doc paths verified present |
| `feedback_*.md` memory entries | 78 | all but 3 indexed in MEMORY.md (mechanical fix applied, see below) |
| `project_*.md` memory entries | 7 | all currently indexed |
| MEMORY.md index entries | 81 (of 85 fragment files) | gap closed by this pass for 3 missing entries |

---

## Findings

### Fragments needing measurement (cannot re-measure in single recalibration session)

These contain calibration numbers measured under Codex-5.4 substrate. Substrate stamps were added by this pass; measurement against current substrate is operator-coordinated.

| File | Calibration claim | Suggested measurement protocol |
|---|---|---|
| `feedback_research_verifier_roi.md` | 6.3% scout error rate (9/142 claims caught) | Sample 15-20 PRs from past 7 days under current substrate; count research-verifier catches per scout claim. Expect lower number — ChatGPT-Pro pre-planning surfaces external-fact errors before Codex generates the PR. |
| `feedback_codex_ensemble_pattern.md` | "3-5 PRs per issue" cluster ratio | Sample 10 issues from past 7 days that received Codex bursts; count PRs per issue. Initial 5.5 observations suggest tighter scope and lower variant-spread, but this is unverified. |
| `feedback_deep_review_bug_catch_roi.md` | ~100% hit rate (17 findings / 14 PRs in 2026-04-24 session); 30% perf-bug share | Sample 10-15 PRs deep-reviewed in past 7 days under Codex 5.5; count findings per PR. Substrate shift should narrow residual catch surface — that is the intended direction, not a regression. |
| `feedback_master_bitrot_cascade_8plus_pattern.md` | "1 master fix per 2-3 PRs merged" during cluster bursts | Sample one cluster-merge segment under Codex 5.5; count master fixes vs PRs merged. Substrate shift hypothesis: tighter scopes reduce per-PR drift, but cluster size + parallel-edit dynamics may dominate regardless of model version. |
| `feedback_xtask_fmt_false_cascade.md` | 7/12 PRs were real fmt issues (5/12 false-positives) | Substrate-independent (it's a tool behavior pattern, not a model behavior pattern). Re-measure only if `xtask/src/tasks/fmt.rs` abort behavior changes (tracked in #6791). |

### Fragments needing substrate review (no number, but pattern claim may have shifted)

| File | What may have changed | What to verify |
|---|---|---|
| `feedback_codex_framework_hallucination.md` | The list of confirmed hallucinations is from 2026-04-23 (5.4 era). 5.5 + ChatGPT-Pro repo-connector reads MetaCPAN before generating, so this class may be largely upstream-suppressed now. | Spot-check the next 10 framework-detection PRs against MetaCPAN; if zero hallucinations, demote rule to "spot-check, not gate." |
| `feedback_broad_scope_codex_stack_diversity.md` | Layer-diversity behavior was a 5.4 prompt-laxity artifact. ChatGPT-Pro pre-planning specifies layer/files; layer-diversity may now be operator-controlled via prompt rather than emergent. | Compare 5 broad-prompt 5.4-era bursts vs 5 ChatGPT-Pro-pre-planned 5.5-era bursts on file-path overlap distribution. |
| `feedback_perl_feature_claims.md` | "Scouts hallucinate Perl features" is a 5.4 scout problem. Sonnet/Haiku scouts under current substrate may hallucinate at different rates. | Track research-verifier perldoc.perl.org catches per scout-issue over the next 20 issues. |
| `feedback_env_var_resource_limits.md` | Substrate-independent; this is a Linux kernel fact, not a model fact. | No verification needed; mark as substrate-stable. |
| `feedback_upstream_research_improves_pr_quality.md` | This entry IS the substrate-shift hypothesis. It needs re-grounding once the first round of measurement above completes. | After the three measurement protocols above complete, update this entry's evidence with quantified deltas. |
| `feedback_prompt_generation_is_cheap_web_thread.md` | Architectural framing of the upstream loop — substrate-stable as long as ChatGPT-Pro + GitHub-repo-connector remains the upstream layer. | Re-verify if upstream layer changes (e.g., a different research model takes over). |

### Broken cross-references

None found. `dispatch-index.toml` was the primary integrity-check target; all 21 situation_id entries point to fragment paths that resolve to existing files.

`MEMORY.md` had 3 fragment files not present in the index (see "Mechanical fixes applied" below). All linked entries in MEMORY.md resolve.

`docs/forensics/INDEX.md` and `docs/forensics/README.md` describe an older PR-archaeology subsystem (PR dossiers, work orders, calibration CSV) orthogonal to the prompt-fragment architecture. They are not broken; they document a different layer that pre-dates the 2026-04-25 architecture refactor. Flag for operator decision (below): should these be cross-referenced from the newer architecture docs, or kept fully separate?

### Consolidation candidates

These are explicitly named in `2026-04-25-memory-effectiveness-review.md` (which itself functions as a prior recalibration analysis). Surfacing here for operator decision; recalibrator does not consolidate substantively.

| Candidates | Recommendation | Rationale |
|---|---|---|
| `feedback_master_bit_rot_recurrence_pattern.md` + `feedback_master_bitrot_cascade_8plus_pattern.md` + `feedback_master_bit_rot_cascade_fixes.md` | Merge into one entry with three calibration sub-blocks (3+ instances signal, 8+ instances escalation, fix-and-cascade procedure), each dated. | Shared root cause; reading all three yields redundant rules with subtle divergences. |
| `feedback_concurrent_worktree_contamination.md` + `feedback_swarm_worktree_contamination.md` + `feedback_worktree_file_leak.md` | Merge into one entry with three failure-mode sub-blocks. | All worktree-contamination variants; shared root cause = "shared filesystem state across agents". |
| `feedback_status_check_rollup_stale_entries.md` + `feedback_green_ci_false_positive_pattern.md` | Merge into one entry titled "CI signal interpretation"; sub-blocks for stale-rollup filtering and ci-green false positives. | Both about reading CI signal correctly. |
| `feedback_codex_ensemble_pattern.md` + `feedback_triage_at_scale_validates_ensemble.md` | Merge: triage-at-scale entry is the validated/scaled version of the ensemble pattern. | Same lesson, different vintage. After substrate re-measurement, the merged entry's ratio claim should be re-stated under current substrate. |
| `feedback_pre_push_hook_windows_race.md` + `feedback_absorption_operational_lessons.md` (push subsection) | Cross-link only; both already reference the `HEAD:refs/heads/<branch>` pattern. No merge needed but add explicit "see also" lines. | Independent root causes (Windows file-lock vs tracking-ref ambiguity); same workaround. |

### Retirement candidates

None recommended this pass. Per agent definition, retirement requires operator sign-off with full context. The candidates with weakest current load-bearing:

| File | Why surfaced | Why not retire |
|---|---|---|
| `project_release_strategy.md` | Decided 2026-04-02; 0.13.0 plan has shifted (now driven by collapse, not "0.13.0 announcement readiness"). | Still describes the macro version-line meaning; the v0.13.0 = clean-break-via-collapse claim is captured in `project_microcrate_collapse_v014.md`. Cross-link rather than retire. |
| `project_v013_structural_gaps.md` | 21 days old; one of the three gaps (pragma tracker) saw work this session. | Still names the other two gaps (symbol visibility, parser error recovery) which are unresolved. Update in place rather than retire. |
| `feedback_publish_pipeline_gotchas.md`, `feedback_publish_allowlist_maintenance.md` | Listed as "dormant" in `2026-04-25-memory-effectiveness-review.md`. | Dormancy is not staleness — these load-bear when publish runs. Per the agent definition's "don't archive on dormancy alone" principle, keep. |
| `feedback_dry_run_gate_silent_failure.md` | Older, generic, not session-anchored. | Still load-bearing for any new gate addition; principle is substrate-independent. Keep. |

### Catalog reconciliation findings

`2026-04-25-failure-mode-catalog.md` covers F1–F25. Spot-checked claims against memory entries:

- F1 (per-PR fmt misclassified as cascade) — backed by `feedback_xtask_fmt_false_cascade.md`; consistent.
- F2 (pre-rebuild PR branches can't auto-rebase) — backed by `feedback_fresh_root_master_rebuild.md`; consistent.
- F3 (stale labels persist) — backed by `feedback_green_ci_false_positive_pattern.md` and `feedback_label_skill_silent_failure.md`; consistent.
- F5 (.hermes cross-PR contamination) — backed by `feedback_agent_audit_trail_directories.md`; consistent.
- F6 (stale CI rollup) — backed by `feedback_status_check_rollup_stale_entries.md`; consistent.

No catalog/memory contradictions detected.

---

## Mechanical fixes applied this pass

1. **Added 3 missing entries to MEMORY.md** (pointers were dead — files existed in memory dir but were not indexed):
   - `feedback_codex_framework_hallucination.md`
   - `feedback_broad_scope_codex_stack_diversity.md`
   - `feedback_git_config_test_identity_leak.md`

2. **Added substrate-stamp prefix to three Codex-5.4-era calibration entries** (per the authoring conventions doc, calibration numbers should carry their substrate stamp; these were missing and are explicitly flagged in `2026-04-25-substrate-shift-and-two-timescale-calibration.md`):
   - `feedback_research_verifier_roi.md` — added "Substrate at measurement: Codex 5.4 …" block before the 6.3% claim, with re-measurement protocol.
   - `feedback_codex_ensemble_pattern.md` — same pattern, with re-measurement protocol for the 3-5 PRs/issue ratio.
   - `feedback_deep_review_bug_catch_roi.md` — same pattern, with re-measurement protocol for the ~100% catch rate.

   Calibration numbers themselves were NOT changed. Re-measurement is operator-coordinated work.

3. **No fixes applied to forensics docs** in this pass. The 25 substrate-relevant forensics docs all date from 2026-04-25 (or earlier with explicit window stamps) and already follow the authoring conventions on date stamps and substrate-version naming. No mechanical fixes were warranted.

4. **No fixes to `.claude/agents/`** (out of recalibrator scope per agent definition).

5. **No fixes to `CLAUDE.md`** despite a duplicate `green-tdd-reviewed` row in the sign-off-labels table at lines 67 and 70 — `CLAUDE.md` is methodology infrastructure with a different lifecycle. Flagged below for operator decision.

---

## Recommended operator decisions

In rough priority order:

1. **(High) Schedule the three measurement protocols above.** The 5.4-era calibration numbers (6.3% scout error, ~100% deep-review catch, 3-5 PRs/issue) are now substrate-stamped but unverified against Codex 5.5 + ChatGPT-Pro. Until re-measured, downstream agents loading these fragments are reasoning from stale data. Each protocol fits in one ~20-minute orchestrator-coordinated sweep.

2. **(Medium) Fix duplicate `green-tdd-reviewed` row in `CLAUDE.md`.** Lines 67 and 70 of `CLAUDE.md` define the same `green-tdd-reviewed` label twice in the sign-off table. Mechanical typo. Out of recalibrator scope (CLAUDE.md is methodology infrastructure with different lifecycle); 30-second operator fix.

3. **(Medium) Decide on the three master-bit-rot consolidation.** `feedback_master_bit_rot_recurrence_pattern.md` + `feedback_master_bitrot_cascade_8plus_pattern.md` + `feedback_master_bit_rot_cascade_fixes.md` are explicitly named as overdue for merge in `2026-04-25-memory-effectiveness-review.md` and again in `2026-04-25-forensics-and-memory-authoring-conventions.md`. Three entries with redundant rules and subtle calibration divergences — high noise for the agents that load them.

4. **(Medium) Decide on the three worktree-contamination consolidation.** Same pattern — three entries with shared root cause; agent loading any one or two gets only a partial picture. Per the consolidation principle in `2026-04-25-memory-effectiveness-review.md`, candidate for merge into a single entry with named sub-blocks.

5. **(Low) Cross-link `docs/forensics/INDEX.md` + `README.md` from the newer architecture.** The PR-archaeology subsystem is older infrastructure with no obvious bridge to the prompt-fragment architecture. New operators reading `2026-04-25-forensics-as-prompt-fragments-architecture.md` may be confused by the older docs alongside. Either add a one-paragraph "this directory hosts two distinct subsystems" preface to the newer architecture doc, or move the older PR-archaeology files under a `docs/forensics/archaeology/` subdir.

6. **(Low) Update `project_v013_structural_gaps.md` in place.** Pragma tracker work landed; symbol visibility and parser error recovery remain. 5-minute update with current issue numbers; not retirement.

7. **(Low — defer until measurement protocols complete) Re-evaluate the three "fragments needing substrate review" entries** under "Findings". `feedback_codex_framework_hallucination.md`, `feedback_broad_scope_codex_stack_diversity.md`, `feedback_perl_feature_claims.md` may have shifted under the new upstream-research loop but cannot be confirmed without measurement.

---

## Counts summary

| Category | Count |
|---|---:|
| Fragments inventoried | 132 (47 forensics + 78 feedback + 7 project) |
| Fragments needing substrate-bound measurement | 5 |
| Fragments needing substrate review (no number, pattern shift possible) | 6 |
| Broken cross-references found | 0 |
| Dispatch-index entries verified | 21 / 21 |
| Mechanical fixes applied | 4 (3 missing MEMORY.md pointers, 3 substrate stamps in feedback files; both jobs done as one batch) |
| Consolidation candidates flagged | 5 |
| Retirement candidates | 0 (none warranted; dormant ≠ stale) |
| Operator decisions surfaced | 7 |

---

## Methodology notes (for future recalibration passes)

- **The agent could not re-measure calibration numbers** within the recalibration session — there is no live PR-sample tooling inside this agent's scope. Every numeric calibration carries a "needs measurement, here is the protocol" qualifier rather than a re-measured value. This is the agent definition's intended behavior (see "What you flag for operator decision" in the agent file).

- **Cross-reference integrity is mechanically verifiable** and should be the first pass on every recalibration. The dispatch-index check + MEMORY.md pointer check together took <2 minutes of agent time and surfaced 3 dead pointers.

- **Substrate stamps should be added in-place** to all calibration entries during this pass. Future passes will be cheaper because they can scan for the stamp's presence as the first staleness signal.

- **The previous "memory effectiveness review"** (`2026-04-25-memory-effectiveness-review.md`) is essentially a recalibration analysis from a different angle — it scored entries on whether they fired in a session, while this pass scored them on substrate alignment. Both lenses are valuable; future passes should reference the prior effectiveness review when prioritizing consolidations.

- **No agent-definition recalibration in scope.** `.claude/agents/memory-recalibrator.md` (this agent's own definition) and other agent files were not modified per the explicit out-of-scope rule. If an agent definition is found to reference a stale calibration in its prose, that is operator work, not recalibrator work.

---

## Cross-references

- `2026-04-25-substrate-shift-and-two-timescale-calibration.md` — the substrate-shift framing this pass operationalizes
- `2026-04-25-forensics-as-prompt-fragments-architecture.md` — why this agent class exists in the architecture
- `2026-04-25-forensics-and-memory-authoring-conventions.md` — the authoring style this pass enforces
- `2026-04-25-memory-effectiveness-review.md` — prior recalibration analysis from a different angle
- `2026-04-25-methodology-blind-spots-conways-law.md` — names memory-recalibrator as a previously-blind granularity now addressed
- `.claude/agents/memory-recalibrator.md` — the agent definition this pass executed against
- `docs/forensics/dispatch-index.toml` — the integrity-check target verified by this pass
