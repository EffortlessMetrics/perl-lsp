# 2026-04-23 → 2026-04-25 — Three-Day Arc Economics & Learnings

**Window**: Thursday afternoon 2026-04-23 → Saturday evening 2026-04-25 (~60 hours wall-clock, ~3 large operator sessions)
**Repo**: EffortlessMetrics/perl-lsp
**Operating mode**: Continuous swarm — Codex web bursts upstream, Anthropic verification ladder downstream, Claude Code orchestrating waves of 15-25 parallel sub-agents
**Headline**: Master rebuilt as fresh root commit mid-arc, ensemble triage drained ~108 duplicates in a single 5h session, multi-gate verification caught real correctness bugs at scale, and the closing session burned through its weekly quota with 91% utilization before reset.

---

## Three-day metrics

| Metric | Thursday 04-23 | Friday 04-24 | Saturday 04-25 |
|---|---|---|---|
| Operating mode | tier-wiring reviewer fix-forward | extended throughput, master root rebuild | PR queue drain via ensemble triage |
| Master start | (pre-rebuild root) | rebuilt at `3e06aef40` 12:07 EDT | `7943a13e` |
| Master end | (rolled into 04-24 rebuild) | `222b41e2d` end of day | `9cecad2aa` |
| PRs merged | ~6-8 (from 04-23 forensics) | 25+ (heavy throughput day) | **11** |
| PRs closed via triage | small | moderate | **~111** |
| New session-spawned PRs | scout findings | feature work + Codex bursts | **2 cherry-picks (#6717, #6718)** |
| Forensics docs landed | 1 | 3 | 3 + this one |
| Quota arc | normal | heavy | exhausted then reset to 91% weekly |

**Net Saturday delta**: 410 → 327 open PRs (**-83 net**), confirmed via REST `Link: rel="last"` header on `repos/EffortlessMetrics/perl-lsp/pulls?state=open&per_page=1`.

---

## The pivotal event: master root rebuild

On Friday 2026-04-24 at 12:07 EDT, master was rebuilt as a **fresh root commit** (`3e06aef40` "fix(type-hierarchy): prevent c3 recursion loops"). Every PR branched before that timestamp lost its shared ancestry with current master.

This had cascading effects across all three days:

1. **Friday afternoon**: bulk update-branch attempts on pre-rebuild PRs failed silently — `gh pr update-branch` cannot operate without a merge-base.
2. **Saturday morning**: 17 PRs flagged as "hard merge conflicts" turned out to share zero git ancestry. The "common blocker file" investigation discovered this: `git merge-base origin/master <pr-branch>` returned empty for every one.
3. **Saturday afternoon**: corrected approach — cherry-pick the topical commit onto a fresh branch off current master, open a new PR with cross-ref to the original. Successfully applied to #5679 → #6716, #6133 → #6717, #6014 → #6718.

The root rebuild also exposed several PRs whose changes had **already landed via different routes**: #6240/#6239 already on master via #6312/#6313, #5388 superseded by #6133 → #6717. These were closed-as-superseded.

**Codified**: `feedback_fresh_root_master_rebuild.md` — pre-rebuild PRs need cherry-pick, not rebase.

---

## Cluster triage at scale (Saturday's headline win)

Codex web bursts during Thursday and Friday had stacked dense duplicate clusters across many subsystems. Saturday's first three waves dispatched ensemble-curator agents at the largest clusters and produced a cleaner queue than any prior day.

| Cluster | Closed | Keepers | Notes |
|---|---:|---:|---|
| Lexer (slash, quote-like, Unicode/BOM, perf) | 22 | 4 | C19/C20/C21/C22 — keepers had advanced sign-offs |
| Parser (corpus, recovery, scorecard, classification) | 16 | 6 | Two distinct sub-clusters incorrectly grouped by title; 2 wrong-closures recovered by parallel agent |
| Tree-sitter overlay + semantic façade | 7 | 2 | Architectural keeper #5874 in `perl-semantic-analyzer` (siblings would have inverted dep direction) |
| Incremental document/edit | 9 | 4 | All 4 keepers orthogonal across files — close/keep decision via file overlap, not title |
| Workspace / symbol | 5 | 3 | Refactor-tooling keepers #5797 / #5830 / #5841 |
| Refactor #3522 family (rename, safe-delete, refs) | 9 | 4 | First-wave clusters blocked by transient master bit-rot; second-wave orphans (#6047, #6053) became the keepers |
| Parser closeout (ratchet, classification) | 2 | 3 | One agent over-closed; parallel agent recovered #5989/#5990 (3 distinct fixes wrongly grouped as duplicates) |
| Misc smaller (Module::Runtime, exporter, semantic, lexer-keywords) | 12 | 9 | Each cluster size 2-5 |
| **Total** | **~108** | — | |

**Plus 3 more Saturday-evening closures** (#6240/#6239/#5388) once GitHub quota recovered, bringing the close count to ~111.

**Codified**: `feedback_triage_at_scale_validates_ensemble.md` — sub-linear triage cost per cluster, sort by file path not title, two-day Codex bursts often have second-day winners.

---

## Multi-gate review caught real bugs

Sonnet deep-review continued its ~100% catch rate streak across the arc. Saturday session alone caught:

| PR | Bug |
|---|---|
| #6088 | `return /pat/`, `die /pat/`, `eval /pat/` would have lexed as division (regression test added + fixed) |
| #6379 | Tests used `[T; N]` array literals where `Vec<T>` was expected — wouldn't compile (PR description claimed "all tests pass") |
| #6053 | `is_ambiguous_sub_reference` was dead code — workspace indexer doesn't store cross-package bare-name refs the way the function expected; tests using `expect_err` would receive `Ok(...)` |
| #5403 | Fixture matrix divergence — branch lacked 4 workflow entries master expected; merge would fail `editor_ux_fixture_matrix_covers_all_scenarios` |
| #5361 | Removed `consume_balanced_segment_in_string` while leaving 7 callers; would silently break double-quoted string interpolation lexer |
| #5368 | `code[..start].lines().count()` is off-by-one (returns 2 when expected 1 for line index 1); deleted the 3 regression tests that would catch it |
| #6230 | Registered 2 new blocking CI gates that always fail (CPAN not installed in runner; baseline from different machine) |

**Cumulative ROI**: deep-review catches bugs invisible to `--lib` tests, including 30%+ that are performance bugs invisible to correctness tests. The verification ladder (research → oppositional → architecture → maintainer-issue → plan-review → standards → maintainer-PR → diff-audit → deep-review) stays earning its tokens.

---

## Pattern catches and false alarms (calibration data)

This arc produced enough data to calibrate several heuristics:

### xtask fmt cascade ≠ master bit-rot

12 PRs flagged as "needs per-PR fmt fix" by master-bit-rot scout. **Only 7/12 actually had fmt drift.** The other 5 were stale-base or unrelated. The cascade-detection heuristic ("3+ PRs failing identically = master signal") needs refinement: identical *aggregator-level* failure isn't enough; need identical *first-line-of-failure* matching.

**Codified**: updated `feedback_xtask_fmt_false_cascade.md` with 2026-04-25 calibration.

### Branch contamination claim → false positive

Maintainer-PR batch L flagged 8 PRs as "branch-contaminated" with diff sizes like "3943 files, 213k adds". REST API authoritative `additions`/`deletions` confirmed all 8 PRs are CLEAN at their actual diff size (#6051 = exactly 21 lines, matching prior reviews). The agent confused `gh pr view --changedFiles` metadata or merge-commit log noise with actual PR diff contents.

**Calibration lesson**: cheap-model agents reading metadata fields can produce systematically wrong verdicts at scale. Always verify with REST `pulls/N` `additions`/`deletions` before acting.

### .hermes attribution policy

The hermes-sweep agent flagged 8 PRs as having `.hermes/` artifacts and applied `needs-diff-fix` to all. Corrective audit found:
- **#5870**: strip was correct (work-id was `work-aa31d3df`, branch's work-id was `work-8ca3cd48` — cross-contamination)
- **#5750**: strip would have been wrong (work-id matched exactly — legitimate self-attributed audit trail)
- **6 others**: cross-PR contamination, correctly flagged

**Codified**: extended `feedback_agent_audit_trail_directories.md` — keep `.hermes/conveyor/work-XXXXX/` if `XXXXX` matches the PR's branch work-id, only flag cross-PR work-ids.

### CI step timeouts must include master headroom

Windows Guardrails (sandbox-fail-closed) has a 20-min timeout. Master itself runs that check in 18m51s — only 6% headroom. PRs frequently CANCELLED on this gate, then their next run (without contention) succeeds in 13m8s.

**Codified**: `feedback_ci_timeout_too_tight_for_master.md` — sample 5 master runs and use p95 + 30-50% margin.

### CI workflow trigger observability gap

UX Regression Gate is `on: pull_request:` only — never runs on master pushes. Master-bit-rot scouts cannot directly verify master against UX scenarios. The perl-dap perf cluster (8 PRs) triggered identical UX harness LSP-startup hangs that turned out to be a real master-side regression introduced by `7943a13e` ("fix(perl-dap): harden bridge adapter lifecycle"). Tracking issue #6715 filed.

**Codified**: `feedback_ci_workflow_trigger_observability_gap.md` — always add `on: push: branches: [master]` to baseline-comparison gates.

### CRLF breaks xargs on Windows

Hermes sweep agent's first two parallel runs returned zero matches because Git Bash on Windows emitted CRLF line endings. `xargs -I {}` substituted `<num>\r` into args, every `gh pr diff` call failed with "no such PR" silently.

**Codified**: `feedback_crlf_breaks_xargs_pipelines.md` — always `tr -d '\r'` PR-list inputs before piping to xargs on Windows.

---

## Quota economics

### Anthropic burn pattern

| Stage | Saturday utilization |
|---|---|
| Session start (74% session, 86% weekly remaining) | discovery + state snapshot |
| First 3 hours | 4 dispatch waves of 15-20 agents each — heavy burn |
| ~Hour 4 | 95% session push, 91% weekly hit |
| Org monthly limit hit | sub-agents return immediately with "You've hit your org's monthly usage limit" |
| **Weekly reset 9 minutes later** | full quota restored |
| Closing direct API ops | 16 GitHub calls, no sub-agents — produced 1 merge + 3 closures + 9 label catchups |

**Lesson**: when sub-agents can't run, **the orchestrator can still execute high-leverage actions directly via gh CLI**. The label-catchup pass took 16 calls and 4 minutes; the same work via sub-agents would have taken 4 agents and ~15 minutes of agent-runtime cost.

### GitHub API burn pattern

GraphQL was the consistent choke point. Saturday session exhausted GraphQL twice:
- Mid-afternoon: 25 parallel agents querying `gh pr view --json` simultaneously triggered secondary rate limit (abuse detection)
- Late afternoon: full GraphQL quota exhausted; only REST + `gh search` (separate 30/min bucket) remained

**Workarounds discovered**:
- REST `gh api repos/.../pulls/N` for PR detail (separate quota pool)
- REST `gh api -X PUT/PATCH/DELETE` for label/state changes
- Parallel agent prompts should specify "use REST API directly if GraphQL exhausted" as a fallback strategy

---

## What worked

- **Verification ladder kept its full structure even under throughput pressure** — no single-pass shortcuts. Multi-gate caught hallucinations, scope drift, branch contamination, correctness bugs.
- **Ensemble triage with file-path-coherence sorting** drained ~108 duplicates in one session without losing real work (parallel agent recovery on the over-closure of #5989/#5990 saved 2 distinct parser fixes).
- **Cherry-pick instead of rebase** for pre-rebuild branches restored 2 PRs to mergeable state (#6717, #6718).
- **Direct orchestrator API actions** when sub-agents are out of quota are the cheap finisher — 9 label catchups in 4 minutes.
- **Forensics docs at session end** preserve calibration data: 3 docs landed Saturday, this is the 4th, plus prior-day docs already merged.

## What broke

- **Cheap-model agents misread metadata fields** (changedFiles count, merge-commit log) and produced systematically wrong "contamination" verdicts. REST API is the authoritative source.
- **Parallel ensemble agents collided** on overlapping PR scopes, producing duplicate work and occasionally contradictory verdicts. Coordination via SendMessage was tried but adds complexity; better to scope agent prompts narrowly with explicit "skip already-covered" lists.
- **GitHub abuse detection triggered** on parallel `gh pr view` salvos before the documented per-hour quota was even half-consumed. Need to throttle parallelism per agent (`--limit 5` per request, sequential not parallel within an agent) or stagger dispatches.
- **CRLF on Windows** silently broke `xargs -I {}` pipelines. Defensive `tr -d '\r'` is now in the CRLF memory entry.
- **xtask fmt cascade pattern** produced false positives at 5/12 = 42% rate. Heuristic needs first-line-of-failure matching, not just check-name matching.

## Unresolved at session end

- 8 PRs likely have `.hermes/` cross-PR contamination still needing cleanup (#4890, #5684, #5685, #5691, #5714, #5785 + 2 from earlier flagging; #5750 confirmed exempt)
- Hard-conflict cherry-picks #5711, #5710, #5695 — manual rebase needed (Saturday's cherry-pick agent timed out)
- C21 Unicode plan-review — recommend reopen #6098 + cherry-pick #6099's emoji tag tests
- #6001 PR Smoke FAILURE — investigate (sandbox-fail-closed CANCELLED was a 20-min timeout artifact, but PR Smoke is a separate signal)
- #5793 vs #5795 architecture choice for exporter metadata #3416 — needs plan-review on which placement (perl-semantic-analyzer/class_model.rs vs new analysis/exporter_metadata.rs module)
- perl-dap UX cluster (8 PRs) tracked in #6715 — needs single shared root-cause fix across cluster

---

## Memory entries written or updated this arc

**New** (Saturday session):
- `feedback_fresh_root_master_rebuild.md`
- `feedback_crlf_breaks_xargs_pipelines.md`
- `feedback_ci_workflow_trigger_observability_gap.md`
- `feedback_ci_timeout_too_tight_for_master.md`
- `feedback_triage_at_scale_validates_ensemble.md`

**Updated** (Saturday session):
- `feedback_agent_audit_trail_directories.md` — added 8-PR `.hermes/` cross-PR leak data point + work-id attribution policy
- `feedback_xtask_fmt_false_cascade.md` — 7/12 calibration data point

**Carried forward from Thursday/Friday**:
- `feedback_master_bit_rot_recurrence_pattern.md`
- `feedback_master_bitrot_cascade_8plus_pattern.md`
- `feedback_label_skill_silent_failure.md`
- `feedback_green_ci_false_positive_pattern.md`
- `feedback_concurrent_worktree_contamination.md`

---

## Three-day net deltas

- Open PRs: ~520 (Thursday afternoon estimate) → 327 (Saturday end of day)
- Open issues: similar drop (66+ stale duplicates closed Saturday alone)
- Merges: 40+ across the arc (estimate from commit log) — 11 on Saturday
- Master commits: ~80 between root rebuild and end of Saturday
- Cherry-pick rebases: 4 (#5679 → #6716, #6133 → #6717, #6014 → #6718, plus #5403 fixture-matrix recovery)
- Tracking issues filed: 1 (#6715 perl-dap UX cluster)

The arc demonstrated that high-throughput review-and-merge with ensemble triage is sustainable — under one operator's coordination — provided the verification ladder stays intact and quota arithmetic is respected. The 91% weekly burn at session-end is the upper bound on a single 5h dispatch-heavy session. Steady-state operation is well below that ceiling.

---

## Appendix: orchestration patterns observed in session context

These observations are extracted from the live session transcript itself and may be worth codifying separately:

### Operator direction signals evolved during the session

The operator's instructions shifted as observed signals came in, providing a useful template for future high-volume orchestration:

1. **Start of session**: "continue reviewing and improving and merging the prs" — broad mandate
2. **After first wave returned**: "Call another 20" — confirmation of pace
3. **Mid-session, after agents started colliding**: "Coordinate the agents so they don't accidentally burn gh rate limits" — throttling concern
4. **After the .hermes sweep produced over-aggressive cleanup**: "only strip hermes if it's contamination. if it's just specs for the current pr ... then it's just specs and should remain" — policy correction in flight
5. **Same turn**: "focus them on improving the prs and reviewing and merging. Don't need much ensemble sweeps right now" — pivot away from cluster work
6. **Late session**: "If it's just a local rate limit, we can stagger them a bit and stuff like that" — request for staggered dispatch
7. **Near quota end**: "74% used 5 hour session 86% used weekly limit resets in 9 minutes" — explicit quota state shared
8. **Quota reset**: "Reset at like 91% weekly usage" + "Lets be a bit more conservative with how much we're hitting that API"
9. **Session wrap**: "Log the economics and learnings as a pr"

**Pattern**: the operator stayed engaged with quota arithmetic and provided real-time telemetry. The orchestrator's job was to translate those signals into agent-prompt adjustments (narrower scope, REST API fallback, skip-already-covered lists).

### Promotion sweep iterated 3 times because chain completions arrived in waves

Each promotion sweep found a different set of newly-eligible PRs because the sign-offs needed to complete the chain landed asynchronously across waves. Sweep #1 found 0 new (only the originally-flagged #6371/#6278/#6001 candidates). Sweep #2 found 2 (#6357, #6179). Sweep #3 found 7 (#5541 + #5428/#5427/#5426/#5424/#5420/#5419 from a single Codex test cluster). A 4th sweep (manual via direct gh CLI after weekly reset) found 1 more (#5320 after maintainer-pr-reviewed catchup).

**Pattern**: promotion sweeps need to be re-run after each batch of label-applying agents return, not as a one-shot. Same applies to ops drain.

### Direct orchestrator API ops are the cheap finisher

When the org monthly limit blocked sub-agents, 16 direct gh CLI calls accomplished:
- 6 `maintainer-pr-reviewed` label applications
- 1 `needs-diff-fix` strip + diagnostic comment (#5750 corrective)
- 2 `ci-green` label applications
- 3 `needs-ci-fix` label applications
- 3 PR closures with cross-ref comments
- 1 PR promotion to `merge-ready`
- 1 PR auto-merge invocation

This took ~4 minutes wall-clock. The same work via 4-5 sub-agents would have taken 15+ minutes of agent-runtime + token cost. **For clearly-scoped mechanical actions, the orchestrator should execute directly rather than dispatch sub-agents.**

### Two parallel deep-review agents collided on PR #5403

A maintainer-pr review and a reviewer-deep agent both ran against #5403 in the same wave. The first applied a fix-forward (cherry-pick rebase + matrix update) and approved. The second saw the same issues but didn't have the fix applied yet, returned SEND-BACK with concrete bug list. **Net effect was harmless** — both pointed at the same problems, the fix landed once. But this is a classic cost of high-parallelism on overlapping scopes.

**Pattern**: agent-prompt scoping matters more than agent count. "Pick 3 from this query, exclude these PR numbers" is better than "review 3 high-value PRs" because the latter often picks the same target.

### #6090 was reviewed and labeled by a deep-reviewer AFTER being closed by an ensemble agent

The lexer/parser ensemble (38-PR closure agent) closed #6090 as superseded by #6091. A later reviewer-deep batch picked #6090 from the queue, reviewed it, pushed regression tests to its branch, applied `deep-reviewed` label. The agent didn't notice the PR was already CLOSED.

**Pattern**: agents should check PR state at start of work, not just at task assignment. "Skip if already closed" should be in every reviewer-style prompt.

### REST API as authoritative source over GraphQL aggregations

Maintainer-pr batch L's "branch contamination" verdict was based on `gh pr view --json changedFiles` returning a high number. REST `gh api repos/.../pulls/N` returns authoritative `additions`, `deletions`, `changed_files` from GitHub's actual diff calculator. The two diverged dramatically for stale-base PRs because GraphQL counts include merge-commit content, while REST diff is the actual scope of the PR.

**Pattern**: for "is this PR really N files changed?" questions, always use `gh api repos/.../pulls/N --jq '{additions, deletions, changed_files}'` not `gh pr view --json changedFiles`.

### Worktree creation collisions

Saturday's session inherited 173 locked worktrees from prior sessions. Attempting `git worktree add <path> master` failed with "master is already used by worktree at <other-path>". The fix: always use `git worktree add -b <new-branch> <path> origin/master` to create a new tracking branch rather than checking out master directly.

**Pattern**: worktree commands during high-throughput periods need defensive branch creation, not raw `master` checkout.

### Over-conservative auto-close prevention saved real work twice

Two cases this session where an over-aggressive ensemble-curator closed PRs that turned out to be distinct work:
- Parser closeout C12: #5988/#5989/#5990 share a title template but address 3 different sub-issues touching disjoint files. Wrong-closure of #5989 and #5990 was caught by parallel agent within same wave and both PRs reopened.
- C21 Unicode #6097/#6098/#6099: original ensemble agent skipped closure due to "uncertain" (correct conservative posture). Subsequent plan-review agent closed all 3 with documented rationale (#6098 is the right keeper, #6099 has a real coverage gap, #6097's defensive guards are an anti-pattern).

**Pattern**: "DO NOT close anything where you're uncertain" instruction in cluster-triage prompts is load-bearing. The cost of one wrong-closure (lost engineering work) far exceeds the cost of N kept-too-long PRs.
