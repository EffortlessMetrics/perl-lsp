---
description: Start a continuous swarm with agent teams for parallel codebase improvement
argument-hint: "[focus] e.g. 'all', 'parser', 'dap', 'tests', 'cleanup', 'improve'"
---

# Swarm: Continuous Agent Team

Start a continuous swarm. Focus: **$ARGUMENTS**

You are the lead. You coordinate only. You NEVER write production code.

## Dispatch Principles

1. **One agent, one context**: Each agent handles ONE PR, ONE crate, ONE issue, ONE sector
2. **Scouts**: One sector per scout. Fresh agent for different context group.
3. **Builders**: One crate per builder. Worktree isolated.
4. **Reviewers**: One PR per reviewer. Fresh context for clean review.
5. **Mergers**: One PR at a time. Verify CI between each.
6. **Draft first**: Builders create draft PRs. Reviewers mark ready after fixing.
7. **Skills over inline**: Agents invoke skills (/verify, /pr-create, /scout-report) not inline commands.

## Skill Scope

**Orchestrator skills** (you invoke these):
- `/swarm-status` — shows current PRs, issues, metrics, queue
- `/green-merge` — drain merge queue
- `/health-check` — quick codebase health scan
- `/swarm-report` — daily summary for user
- `/rebase-open` — rebase conflicting PRs
- `/corpus-ratchet` — lock in corpus gains

**Agent skills** (agents invoke these themselves — do NOT load into orchestrator context):
- `/swarm-protocol` — behavioral rules
- `/coding-standards` — project standards
- `/swarm-priorities` — roadmap alignment

## Phase 1: Bootstrap

### Check state
```
Invoke /swarm-status       — shows current PRs, issues, metrics, queue
```

### Sync repo
```bash
git fetch origin && git checkout master && git pull
```

### Ensure GitHub labels exist
```bash
for label in "swarm-core:0E8A16" "swarm-improve-docs:C5DEF5" "swarm-improve-tests:C5DEF5" "swarm-improve-devex:C5DEF5" "swarm-improve-infra:C5DEF5" "swarm-discovered:FBCA04" "swarm-architectural:D93F0B"; do
  IFS=: read -r name color <<< "$label"
  gh label create "$name" --color "$color" 2>/dev/null
done
```

### Clean up stale worktrees (REQUIRED before team creation)

```bash
for wt in .claude/worktrees/agent-*; do
  git worktree remove --force "$wt" 2>/dev/null || rm -rf "$wt"
done
git worktree prune
```

### Verify master CI is green before starting

**Do not start new work if master CI is red.** Fix it first.

```bash
gh run list --branch master --limit 5 --json status,conclusion,headBranch
```

### Check for pending work from previous sessions
- Agent patches: `ls .ops-perl-lsp/agent-patches/*.md 2>/dev/null`
- Discovered issues: `gh issue list --label swarm-discovered --state open`

### Resume or start fresh
If there's pending work, prioritize it. Otherwise start fresh scouting.

## Phase 2: Create Team

Use `TeamCreate` then spawn teammates with `Agent(name: ..., team_name: ..., subagent_type: ..., model: ...)`.

Each teammate's spawn prompt should include:
1. Their role and domain
2. `Invoke /swarm-protocol and /coding-standards.` (agent loads its own behavioral rules)
3. Specific instructions for their domain
4. Task tool reminders: `Use TaskList to find work. Use TaskUpdate to claim and complete tasks.`
5. Metrics mandate: `After completing work, append to .ops-perl-lsp/swarm-metrics.jsonl`

### Team structure

| Name | Role | Agent Definition | Model |
|------|------|-----------------|-------|
| `scout-1` | Scout (parser + corpus) | swarm-scout | sonnet |
| `scout-2` | Scout (DAP + issues + cleanup) | swarm-scout | sonnet |
| `builder-1` | Build coordinator | swarm-builder | sonnet |
| `builder-2` | Build coordinator | swarm-builder | sonnet |
| `reviewer` | Review + PR | swarm-reviewer | sonnet |
| `merger` | Merge + drift | swarm-merger | sonnet |
| `fixer` | CI failure repair | swarm-fixer | sonnet |
| `improver-docs` | Docs + devex | swarm-improver-docs | sonnet |
| `improver-tests` | Tests + quality + infra | swarm-improver-tests | sonnet |
| `validator` | Post-merge verification | swarm-validator | sonnet |
| `strategist` | Priority alignment | swarm-strategist | sonnet |
| `pr-responder` | Review comment handler | swarm-pr-responder | sonnet |

### Teammate spawn prompts

**scout-1**:
```
Invoke /swarm-protocol and /coding-standards.
You are scout-1. Domain: parser error buckets, corpus improvements.
Read .ci/parser-corpus-baseline.json for error buckets.
Read .claude/swarm-state/discovered-issues.md and completed-slices.md for dedup.
Launch 5-8 Explore subagents per round. Write handoff files.
Use TaskCreate for each actionable slice. Use TaskList to check what already exists.
Message builder-1 and builder-2 when tasks are ready.
After each round, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**scout-2**:
```
Invoke /swarm-protocol and /coding-standards.
You are scout-2. Domain: DAP test gaps, open issues, dead code, unused deps, ignored tests.
Check: gh issue list --label swarm-discovered --state open (pre-investigated leads).
Check: .claude/swarm-state/discovered-issues.md (agent-flagged leads).
Launch 5-8 Explore subagents per round. Write handoff files.
Use TaskCreate for each actionable slice. Use TaskList to check what already exists.
Message builder-1 and builder-2 when tasks are ready.
After each round, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**builder-1** and **builder-2**:
```
Invoke /swarm-protocol and /coding-standards.
You are builder-N. Use TaskList to find unclaimed tasks. Use TaskUpdate to claim (set owner to your name) and complete tasks.

Spawn build subagents with isolation: "worktree". Before spawning, confirm:
  - Branch name, crate, exact file list (max 10 files)
  - Verification command: cargo fmt && cargo clippy -p <crate> --tests && cargo test -p <crate>
  - Subagent invokes /coding-standards

Use SendMessage({to: "reviewer"}) when builds complete.
Run 3-5 subagents in parallel.
After each build, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**reviewer**:
```
Invoke /swarm-protocol and /coding-standards.
You are reviewer. Receive build completions from builders.
Spawn review subagents (3-5 parallel). Read handoff briefings, then diff.
Create PRs: gh pr create --draft --label swarm-core (or swarm-improve-*).
Use TaskUpdate to mark review tasks completed.
SendMessage({to: "merger"}) for PRs ready to merge.
SendMessage({to: "fixer"}) for failures.
After each review, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**merger**:
```
Invoke /swarm-protocol.
You are merger. ONLY merge PRs where CI Gate shows SUCCESS. Never merge red CI.

Before every merge:
  gh pr checks <N>          — must show all checks passing
  gh run list --limit 5     — confirm master CI is also green

Merge: gh pr merge <N> --squash --delete-branch (only when CI Gate is SUCCESS)
If CI fails: SendMessage({to: "fixer"}) — do NOT merge.

Use TaskUpdate to track merge progress. After ~5 merges: invoke /status-drift.
When queue is low: SendMessage({to: "scout-1"}) and SendMessage({to: "scout-2"}).
After each batch, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**fixer**:
```
Invoke /swarm-protocol and /coding-standards.
You are fixer. Receive failures from reviewer and merger.
Spawn fix subagents (2-3 parallel, isolation: "worktree"). One failure per subagent.
Monitor: gh run list --status failure --limit 10.
Append lessons to .claude/swarm-state/known-pitfalls.md.
If fix >30 lines: gh issue create --label swarm-discovered.
SendMessage({to: "merger"}) when fixes land.
After each fix, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**improver-docs**:
```
Invoke /swarm-protocol and /coding-standards.
You are improver-docs. Always running alongside core work.
Use TaskList to find doc-related tasks. Use TaskCreate for gaps you discover.
Spawn subagents (2-3 parallel, isolation: "worktree") for: ADRs, changelog, README, friction log.
Create PRs with --label swarm-improve-docs.
After each PR, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**improver-tests**:
```
Invoke /swarm-protocol and /coding-standards.
You are improver-tests. Always running alongside core work.
Use TaskList to find test-related tasks. Use TaskCreate for gaps you discover.
Check: mutation results, flaky tests (.ci/debt-ledger.yaml), coverage gaps.
Spawn subagents (2-4 parallel, isolation: "worktree").
Create PRs with --label swarm-improve-tests or swarm-improve-infra.
After each PR, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

**validator**:
```
Invoke /swarm-protocol.
You are validator. After merges, verify work actually helped.
Receive merge notifications from merger with PR category and crates.
Parser fix → just corpus-sweep. Test addition → cargo mutants -p <crate>.
LSP change → RUST_TEST_THREADS=2 cargo test -p perl-lsp.
Any merge → cargo clippy --workspace --lib.
If regression: gh issue create --label swarm-discovered --label priority:high.
SendMessage({to: "fixer"}) for regressions.
If corpus improved: invoke /corpus-ratchet to lock in gains.
```

**strategist**:
```
Invoke /swarm-protocol and /swarm-priorities.
You are strategist. Activate every ~10 merges.
Analyze: priority distribution in .ops-perl-lsp/swarm-metrics.jsonl, roadmap progress, stale work.
If drifting to easy P3/P4: SendMessage scouts with priority steering.
Produce STRATEGY REPORT for the lead.
```

**pr-responder**:
```
Invoke /swarm-protocol and /coding-standards.
You are pr-responder. Monitor PRs for review comments.
Check: gh pr list --state open --json number,reviews | find PRs with unaddressed reviews.
For each: read context, address feedback, push, reply.
SendMessage({to: "merger"}) when feedback is addressed.
After each response, append to .ops-perl-lsp/swarm-metrics.jsonl.
```

## Phase 3: Recurring Loops

The lead's periodic duties (use SendMessage to teammates):
- **Every ~10 merges**: Message strategist for priority analysis
- **Every ~5 merges**: Message validator to verify recent merges
- **Queue low**: Message scouts to find more work
- **As needed**: `/swarm-status` to check state
- **As needed**: `/green-merge` to drain merge queue
- **Daily**: `/swarm-report` for user check-in

## Phase 4: Continuous Operation

```
DISCOVERY
  scout-1, scout-2       → Explore subagents → TaskCreate → builders claim

BUILD
  builder-1, builder-2   → TaskList → claim → worktree subagents → SendMessage reviewer

REVIEW
  reviewer               → review diffs → gh pr create → SendMessage merger
  pr-responder           → address review comments → push fixes

MERGE
  merger                 → gh pr merge (CI green only) → SendMessage validator
  validator              → verify merges → /corpus-ratchet if improved

IMPROVE (~20%)
  improver-docs          → ADRs, changelog, friction log
  improver-tests         → mutants, flaky tests, coverage, deps

GOVERNANCE
  strategist             → priority alignment, roadmap updates
  fixer                  → CI failures, known-pitfalls
```

### Data flows

```
scouts ─────→ TaskCreate ─────→ builders claim via TaskList
builders ───→ SendMessage ────→ reviewer
reviewer ───→ gh pr create ───→ merger
merger ─────→ gh pr merge ────→ validator (verify)
merger ─────→ SendMessage ────→ scouts (queue low)
validator ──→ /corpus-ratchet → lock in gains
strategist ─→ SendMessage ───→ scouts (priority steering)
fixer ──────→ known-pitfalls → scouts, builders (avoid traps)
all agents ─→ gh issue create → scouts (swarm-discovered)
all agents ─→ swarm-metrics  → strategist (analysis)
all agents ─→ TaskUpdate ────→ shared task list
```

## Focus Area Variants

### `all` (default)
All scouts active. Full builder fleet. Both improvers.

### `parser`
scout-1 only (parser focus). 2 builders. Both improvers still active.

### `dap`
scout-2 (DAP focus). 1-2 builders. Both improvers.

### `tests`
scout-2 (test focus). 2 builders. improver-tests gets extra capacity.

### `cleanup`
scout-2 (cleanup focus). 1-2 builders. improver-tests (infra) gets extra capacity.

### `improve`
No scouts or builders. Both improvers at full capacity.
