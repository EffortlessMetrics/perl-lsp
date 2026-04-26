# 2026-04-25 — Open Questions & Research Backlog

**Lens**: Things this session surfaced as unknowns worth tracking — questions that came up during execution but didn't get answered in the moment
**Purpose**: Standing research backlog for future sessions or dedicated investigation

These aren't action items (those are in `2026-04-26-session-priorities.md`) and they aren't bugs (those are in their own issues). They're **open questions** about the project, the orchestration model, and the operating environment.

---

## Repo-state questions

### Q1: What's the actual current rate of new PR creation from upstream Codex bursts?

The session closed ~111 PRs and merged 14, but 410 → 327 = -83 net. The discrepancy means roughly 40 new PRs were created during the 5-hour session window. Is that the steady-state rate, a Saturday-only spike, or burst-pattern?

**Why it matters**: determines whether queue can be drained to zero or only kept in steady-state. Steady-state-low (200-300) is a different operating mode than queue-zero.

**How to investigate**: query `gh search prs --owner EffortlessMetrics --created:>=2026-04-20 --json number,createdAt,author --limit 1000` and bucket by hour-of-day across the past 7 days. Identify Codex thread cadence pattern.

---

### Q2: Are the 173+ locked worktrees actually serving running agents, or are they orphaned from killed agents?

`git worktree list` shows 173 entries marked locked. Some are clearly running (saw the lock acquired by agent PID 164956). But many were likely from agents killed by quota limits before their wrap-up step ran.

**Why it matters**: 10-30 GB of disk overhead. Also, future agents might fail to acquire worktree slots because the named slot already exists.

**How to investigate**: cross-reference `.claude/worktrees/agent-XXX/.git` lock-file PIDs against currently running processes. Anything with a PID that doesn't exist anymore is orphaned and safe to clean.

---

### Q3: Does the perl-dap UX scenario_01 hang affect production users or just CI?

The 8-PR perl-dap perf cluster all hit `UxHarness::new → UxClient::spawn → handshake → wait_for_response` timeout. Reviewer concluded it's a real master regression from `7943a13` (perl-dap bridge adapter lifecycle fix). Filed as tracking issue #6715.

**Why it matters**: if it affects production users, it's a hot fix priority. If it's only CI (test harness initialization), it's a tracking-issue priority.

**How to investigate**: try `perl-dap` from a real DAP client (e.g., VS Code Perl debug session) on master. If it hangs the same way, production-affecting. If not, CI-harness-only.

---

### Q4: What's the cost-per-merge across the verification ladder in dollars?

Tokens-per-merge order-of-magnitude estimates exist in #6763, but actual dollar cost depends on Claude Sonnet 4.7 pricing × token count.

**Why it matters**: budget planning, ROI justification for the orchestration investment.

**How to investigate**: track Anthropic API spend per session and divide by merge count. Need at least 3-5 sessions of data to get a stable estimate. May want to track separately by tier (sonnet vs haiku) since the per-token costs differ.

---

### Q5: What's the post-merge bug-find rate?

Reviewer-deep catches bugs at ~100% rate before merge. But that's the bugs we *find*. What about bugs that escape into master and only surface days/weeks later?

**Why it matters**: validates whether the verification ladder is the right depth, or whether more (or less) review would change the outcome.

**How to investigate**: triage closed issues filed against master in the past 30 days. For each, identify which PR introduced the bug and which review stage missed it. Compute escape rate per stage.

---

### Q6: How much of the 134-workspace-member crate count is essential vs. transitional?

CLAUDE.md target for v0.13.0 is 135→30 published crates. The non-published ones (workspace-internal helpers, test-support, dev-only) presumably stay. Of the 30 publishable, how many are "would still exist post-collapse" vs "exist only because of historical microcrate splitting"?

**Why it matters**: informs the v0.13.0 collapse plan's scope.

**How to investigate**: walk `cargo metadata` output, check each crate's `[package].publish` setting, then triage which non-publish crates are truly internal-helpers vs vestigial.

---

### Q7: Is the legacy `tree-sitter-perl/` C tree still providing benchmark value?

CLAUDE.md notes it's excluded from workspace and "kept for benchmarking". Does anyone actually run the benchmarks against it currently? If not, ~7000 files of disk overhead per worktree for nothing.

**Why it matters**: simplifies the worktree footprint dramatically if removable.

**How to investigate**: grep recent commit log for any reference to `tree-sitter-perl/` benchmark targets; check `just benchmarks` output to see if any benchmarks reference it; ask the maintainer when it last provided actual data.

---

## Orchestration-model questions

### Q8: Is a "policy bus" mechanism feasible — running agents check for policy updates before destructive actions?

The .hermes attribution policy was elicited mid-execution after the wrong policy had already executed. A "before destructive action, check the policy bus" mechanism would prevent this class of post-hoc correction.

**Why it matters**: reduces the cost of mid-wave policy updates from "corrective wave required" to "agents self-correct".

**How to investigate**: design sketch for a per-repo policy file (e.g., `.claude/orchestration-policies.toml`) that running agents poll before executing high-blast-radius actions. Specify which policy types belong (.hermes attribution, scope-drift threshold, conservative-close criterion, etc.). Determine how often agents poll (start of session? per-action?).

---

### Q9: What's the optimal parallelism per wave, given 4-6% collision rate observed?

The 4-6% collision rate is from a sample of ~80 agents in one session. Is it stable, or does it scale super-linearly with parallelism (10-agent waves: 4%, 25-agent waves: 8%, 50-agent waves: ?). 

**Why it matters**: determines whether "go wider" is a free win or whether parallelism has a knee in the curve.

**How to investigate**: track collision rate per wave size across multiple sessions. If linear, max parallelism is operator-controlled. If super-linear, find the knee and don't dispatch beyond it.

---

### Q10: Should sub-agents share state via a "session journal" rather than independently re-discovering it?

Multiple agents this session re-discovered the same facts (master SHA, open PR count, which clusters exist). A shared session-journal file (e.g., `.claude/session-journal.md` updated by each agent) could eliminate redundant queries.

**Why it matters**: reduces GitHub API consumption per wave, reduces the chance of stale-state collisions.

**How to investigate**: prototype a journal-write step in agent wrap-up; measure whether subsequent agents' query count drops.

---

### Q11: When does the verifier-of-verifier pattern not pay off?

It paid off twice this session (maintainer-pr-L false positive, .hermes attribution gap). But each verifier-of-verifier run costs additional tokens. At what point is it cheaper to just trust the first verdict?

**Why it matters**: avoid runaway "verify everything twice" pattern that quadruples token cost.

**How to investigate**: track verifier-of-verifier outcomes across sessions. If it catches false positives <5% of the time, the cost-benefit may flip toward trusting first verdict.

---

### Q12: Can the orchestrator detect "stale state" warnings before dispatching agents?

Examples: "you're operating on an open PR that was closed 5 minutes ago", "this label was just stripped by another agent", "this branch was just rebased". Currently agents discover staleness on their own.

**Why it matters**: reduces wasted agent dispatch on stale targets.

**How to investigate**: orchestrator-side state cache with TTL; query API once at orchestrator level and pass relevant snippets into agent prompts.

---

## Process and tooling questions

### Q13: Is the editor-registry consolidation (per #6764 recommendation) worth the up-front cost?

The current "1 PR per editor with hand-edited matrix entry" pattern has visible scaling problems (cross-PR contamination, destructive scope drift). A registry approach would solve them but requires up-front design work.

**Why it matters**: 22+ editors today, growing. If the project hits 40+, the manual pattern fully breaks down.

**How to investigate**: pilot the registry pattern for 3-5 new editors and measure: (a) PR coordination overhead vs current, (b) auto-generation accuracy, (c) reviewer time per registry-update PR vs per docs PR.

---

### Q14: Can the master-bit-rot-detection heuristic be improved with first-line-of-failure matching?

Current heuristic ("3+ PRs failing identically = master signal") had 5/12 false positive rate this session. First-line-of-failure matching would distinguish per-PR fmt issues from real upstream regressions.

**Why it matters**: reduces wasted "fix master" dispatch when actual fix is per-PR.

**How to investigate**: implement first-line extraction in green-CI agent prompts; A/B against current heuristic across a few sessions; measure false-positive rate change.

---

### Q15: Is there a batched-fix workflow for cluster-of-PRs-share-root-cause situations?

The perl-dap UX cluster (#6715) needs ONE upstream fix + cascade-update across 8 PRs. Currently this requires manual coordination. Could the orchestrator have a "cluster-fix" agent type that handles this pattern end-to-end?

**Why it matters**: would eliminate the per-PR-fix-conflict-cascade pattern that wastes builder time.

**How to investigate**: design an agent prompt that takes (a) tracking issue, (b) list of affected PRs, (c) upstream fix description; dispatches: one builder for the fix, one cascade-update for the affected PRs after the fix lands.

---

## Notes on prioritization

These are questions, not action items. Most don't need answers tomorrow. Prioritization order based on cost-of-not-knowing:

**High** (answer next 1-2 sessions):
- Q3 (perl-dap hang production impact?) — affects user-facing severity
- Q1 (PR creation rate?) — affects whether to plan for queue-zero or steady-state-low
- Q14 (improve master-bit-rot heuristic?) — directly reduces false-positive cost

**Medium** (answer when convenient):
- Q2 (orphaned worktrees?) — affects disk overhead but not correctness
- Q5 (post-merge bug-find rate?) — validates verification ladder depth
- Q9 (collision rate scaling?) — informs parallelism limits

**Low** (answer if specifically blocked):
- Q4 (dollar cost-per-merge?) — useful for budget but not blocking
- Q6, Q7 (collapse / legacy crate questions) — relevant for v0.13.0 planning
- Q8, Q10, Q11, Q12 (orchestration-model improvements) — interesting but not blocking
- Q13, Q15 (workflow improvements) — interesting design questions

---

## Adding to this list

If future sessions surface questions worth tracking, append to this doc. The format: `Q<N>: <one-line question>` + `**Why it matters**:` + `**How to investigate**:`. Keep entries short — this is a backlog, not a research log.

When a question gets answered, leave the entry but add `**Resolved YYYY-MM-DD**:` with the answer summary. Don't delete — the question's history is part of the project's understanding of itself.
