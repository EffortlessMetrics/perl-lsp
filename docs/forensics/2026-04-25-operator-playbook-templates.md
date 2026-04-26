# 2026-04-25 — Operator Playbook Templates

**Lens**: Reusable wave-design templates for different starting states
**Purpose**: Match wave shape to queue state — given X, dispatch Y

The 7 wave shapes documented in #6763 (orchestration anatomy) all worked, but the question of *which* shape to dispatch given a particular starting state hasn't been codified. This doc gives templates: "if your queue looks like X, here's the wave shape that fits".

---

## How to use this doc

At session start, run a state snapshot:

```bash
# Quick state read
gh pr list --state open --json number --limit 1 -L 1 -i 2>&1 | grep -i 'link:' | head -1   # total PR count
gh pr list --state open --search "label:merge-ready" --json number --limit 100 --jq 'length'
gh pr list --state open --search "label:ci-green -label:merge-ready" --json number --limit 100 --jq 'length'
gh pr list --state open --search "label:diff-audited -label:ci-green" --json number --limit 100 --jq 'length'
gh pr list --state open --search "label:needs-ci-fix" --json number --limit 100 --jq 'length'
gh pr list --state open --search "label:needs-builder-fix" --json number --limit 100 --jq 'length'
gh pr list --state open --search "label:needs-diff-fix" --json number --limit 100 --jq 'length'
git fetch origin master && git log origin/master --oneline -5
gh run list --branch master --limit 3 --json conclusion,headSha
```

Then match the bucket distribution to the templates below.

---

## Template 1 — "Fresh queue, lots of unreviewed PRs"

**Starting state**:
- 100+ open PRs
- Most have no review labels yet (raw inflow from Codex bursts)
- Master green

**Dominant problem**: discovery — what's actually in the queue?

**Wave shape**:
- 1 cluster-scout agent (general-purpose) to identify duplicate clusters in the inflow
- 4-6 standards-review agents in parallel (haiku) — each handles 5-7 PRs
- 4-6 maintainer-pr agents in parallel (haiku)
- 3-4 diff-audit agents
- 1 master-bit-rot scout to verify CI infrastructure is healthy

**Outcome**: builds queue understanding + initial sign-off coverage. Don't merge yet.

**Don't**: dispatch ops drain in this state. Nothing is ready to merge.

---

## Template 2 — "Lots of duplicate clusters from recent Codex burst"

**Starting state**:
- Recent burst created 20-50 PRs in ~1 hour
- Multiple PRs share `(#NNNN)` issue refs
- Many `task_e_<id>` references in PR bodies indicate same Codex task

**Dominant problem**: deduplication

**Wave shape**:
- 1 cluster-scout to map out clusters
- 4-8 ensemble-curator agents in parallel, one per cluster of size 3+
- Each curator MUST have explicit "skip already-covered: #N1, #N2" lists
- Each curator MUST have "DO NOT close where uncertain" instruction (load-bearing)
- 2-3 standards-review agents in parallel for any singleton PRs

**Outcome**: highest closure throughput. Saturday session demonstrated ~108 closures across 9 clusters.

**Don't**: review every PR individually before clustering. The clustering itself does the heaviest filtering.

---

## Template 3 — "Many PRs awaiting CI / merge"

**Starting state**:
- 50+ PRs labeled `diff-audited` but missing `ci-green`
- Master green
- No widespread CI failures

**Dominant problem**: throughput

**Wave shape**:
- 4-6 green-CI agents in parallel (haiku), each handling 8-10 PRs
- 1-2 promote-sweep agents (after first green-CI batches return)
- 1 ops-drain agent (after promote-sweep returns)
- Iterate: promote → drain → promote → drain in batches of 3 merges

**Outcome**: Saturday session merged 10 PRs across 4 ops drain passes using this pattern.

**Don't**: dispatch all green-CI batches at once and wait for all to return before promoting. Iterate — each batch's GREEN PRs feed the next promote sweep.

---

## Template 4 — "Master cascade suspected (3+ identical failures)"

**Starting state**:
- 3+ PRs failing identically on Compile + PR Smoke + Windows-something
- Pattern looks systematic

**Dominant problem**: distinguish real master cascade from per-PR fmt drift

**Wave shape**:
- 1 master-bit-rot scout (priority dispatch, blocks everything else)
- Wait for verdict before dispatching anything else

**Possible verdicts and follow-ups**:

a) **Real master cascade detected**:
   - 1 builder agent fixes master narrowly
   - 1 cascade-update agent runs `gh pr update-branch` across affected PRs after fix lands
   - Resume normal waves

b) **Per-PR fmt drift** (the most common false-cascade):
   - 1 fmt-fix agent runs `cargo xtask fmt` per affected PR and pushes
   - Be aware: 5/12 PRs flagged in this session as "needs fmt fix" had different root causes
   - For each PR where fmt produced no changes: investigate individually

c) **Stale-base cascade** (PRs old enough to predate a master fix):
   - 1 bulk-rebase agent runs `gh pr update-branch` across affected PRs
   - Affected PRs that conflict (no shared ancestor) may be pre-rebuild → cherry-pick

**Don't**: dispatch fmt-fix wave before master-bit-rot scout returns. You may waste tokens fixing the wrong thing.

---

## Template 5 — "Many hard merge conflicts"

**Starting state**:
- 10+ PRs labeled `needs-builder-fix`
- All have CONFLICTING/DIRTY merge state
- `gh pr update-branch` fails for all of them

**Dominant problem**: root-cause investigation, not bulk rebase

**Wave shape**:
- 1 common-blocker investigation agent (general-purpose) — finds the shared cause (e.g., master root rebuild)

**Possible findings**:

a) **Master root rebuild detected** (no shared ancestry):
   - 1 cherry-pick agent per 3-5 PRs (in worktree isolation)
   - Each cherry-pick produces a new PR with cross-ref + closes original
   - Some cherry-picks will fail (deep conflicts) → leave as `needs-builder-fix` for manual

b) **Common blocker file** (e.g., scope_analyzer.rs):
   - 1 builder agent fixes the file's API
   - 1 cascade-update agent handles the rebase wave

c) **PRs already on master via different routes**:
   - 1 close-as-superseded agent with cross-refs

**Don't**: dispatch many bulk-rebase agents in parallel. They will all fail the same way and waste tokens.

---

## Template 6 — "End of session, lots of pending labels from earlier waves"

**Starting state**:
- Many PRs have most sign-offs but missing one or two specific labels
- Earlier agents reported "verdicts confirmed but rate-limited before label apply"
- A list of pending mutations exists

**Dominant problem**: mechanical label catchup

**Wave shape**:
- **Don't dispatch sub-agents.** Use direct orchestrator API ops via `gh` CLI.
- 16 calls in 4 minutes accomplishes more than a sub-agent wave would
- See #6763 for the specific pattern

**Outcome**: highest tokens-per-action efficiency of the session.

**Don't**: dispatch a "label catchup agent" — it's overkill for mechanical work.

---

## Template 7 — "Critical bug found by reviewer-deep, needs immediate fix"

**Starting state**:
- reviewer-deep just sent SEND-BACK with a concrete bug list
- Bug is 1-line or small surgical fix

**Dominant problem**: surgical fix-forward

**Wave shape**:
- 1 pr-responder agent in worktree isolation
- Prompt MUST include exact file path + line number + correct vs wrong code
- Build + run relevant test before pushing
- Strip `needs-builder-fix` after push

**Don't**: route back through the full pipeline if the fix is mechanical. Memory entry `feedback_reviewer_deep_proactive_fixes` says deep-reviewer should push 1-line fixes directly.

---

## Template 8 — "Cluster of PRs share a single root cause"

**Starting state**:
- 5-10 PRs all fail the same way
- Each PR addresses a different aspect but they all hit the same downstream symptom (e.g., perl-dap UX cluster #6715)

**Dominant problem**: cluster-fix, not per-PR fix

**Wave shape**:
- 1 root-cause investigation agent
- File 1 tracking issue listing affected PRs and shared signature
- Cross-reference all affected PRs with "Tracked in #<issue>"
- After 1 upstream fix lands: 1 cascade-update agent runs `gh pr update-branch` across the cluster

**Outcome**: Saturday filed #6715 for the perl-dap UX cluster. Avoided 8 conflicting per-PR fixes.

**Don't**: route each PR to its own builder agent. They'll produce 8 different fixes for the same root cause and conflict with each other.

---

## Template 9 — "Quota exhausted (sub-agent or org-level)"

**Starting state**:
- Sub-agent dispatch returns BLOCKED-RATE-LIMIT or "monthly limit hit"
- Orchestrator's own tool use still works

**Dominant problem**: extract maximum value from orchestrator's direct execution

**Wave shape**:
- **Don't dispatch any sub-agents.**
- Direct orchestrator API ops for: label catchups, closures with predetermined rationale, mechanical verifications
- Local-only work: writing forensics docs, memory updates, code reading
- If GitHub also throttled: switch to local-only mode

**Outcome**: Saturday session produced 1 merge + 9 label changes + 3 closures via direct ops after sub-agents were blocked.

**Don't**: keep dispatching sub-agents that return BLOCKED in seconds. They consume token budget for no work output.

---

## Template 10 — "Session-end retrospective"

**Starting state**:
- Session goals largely achieved
- Want to capture learnings before quota wraps

**Wave shape**:
- 1 forensics-writer agent (or orchestrator direct) — quantitative session report
- 1 memory-update agent (or orchestrator direct) — new entries + updates to existing
- 1 next-session-priorities agent — what should next session do first?
- Optional: 1 process-patterns agent for meta-learnings

**Outcome**: Saturday produced 4 forensics docs (this set, #6757, #6761, #6763, #6764).

**Don't**: skip the retrospective just because the session is "over". The next operator (or future-you) needs the handoff.

---

## Template combinations across a 5h session

A typical 5h session passes through several templates as queue state evolves:

```
Hour 0:   Template 1 (discovery) OR Template 2 (deduplication) — depending on queue state
Hour 1-2: Template 3 (throughput) — once sign-offs are accumulating
Hour 2-3: Templates 4, 5, or 7 — handling whatever specific failure modes surfaced
Hour 3-4: Template 3 again (more drains as more PRs become merge-ready)
Hour 4:   Template 6 (label catchup) + Template 10 (retrospective)
```

If the session encounters Template 9 (quota exhaustion) mid-flight: pivot to direct ops + local work for the remainder.

---

## What's NOT in this doc (and why)

- **"Build a new feature" template**: out of scope; this is a queue-management playbook
- **"Implement a v0.13.0 milestone" template**: out of scope; that's project-management, not session-orchestration
- **"Investigate a single complex bug" template**: covered by Template 7
- **"Onboarding new editor" template**: not enough data yet; the editor-registry RFC (per #6764) might give the right shape later

---

## Cross-references

- #6757 (economics) — quantitative validation of the playbook patterns
- #6761 (process meta-learnings) — the principles these templates implement
- #6763 (orchestration anatomy) — concrete wave shapes from this session that map to these templates
- `2026-04-25-failure-mode-catalog.md` (sibling) — what to defensively check before dispatching a template
- `2026-04-25-subagent-roi-rankings.md` (sibling) — which agent type to choose within a template
