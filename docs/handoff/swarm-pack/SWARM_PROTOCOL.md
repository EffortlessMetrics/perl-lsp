
# Swarm Protocol

Shared behavioral rules for all swarm agents. Invoke `/swarm-protocol` to load these rules into your context. Core swarm agents include in subagent prompts: "Invoke /swarm-protocol for behavioral rules."

---

## 1. Autonomy: Fix What You See

You are empowered to fix problems you encounter, even outside your assigned slice.

**Same-PR fixes** (do immediately, within your current worktree):
- Formatting issues in files you're already editing
- Clippy warnings in your crate
- Obvious typos in comments or strings near your code
- Broken imports caused by your changes

**File an issue for everything else** (a fresh agent handles it):
Don't try to branch-switch or stash in your worktree. Just create a GitHub issue with enough context that a fresh agent can pick it up without re-investigating:

```bash
gh issue create --title "<type>: <description>" --label "swarm-discovered" \
  --body "Discovered by <agent-type> while working on <branch>.

## Context
<what you found, why it matters — enough that no one re-investigates>

## Files
<paths with line numbers>

## Suggested Approach
<if you have one>"
```

Create issues for: security vulnerabilities, design flaws, missing features, recurring patterns needing architectural decisions.

**Discovery log** (`.claude/swarm-state/discovered-issues.md`):
For smaller items not worth a full issue. Scouts read this as an input source.

**Findings ledger** (`.claude/swarm-state/findings.json`):
For durable control-plane conclusions that should change how the swarm
describes or operates itself.

## 2. Direct Communication

Message other teammates directly. Don't route through the lead.

- **Builder → Improver-docs**: "Found undocumented pattern in <crate>."
- **Builder → Improver-tests**: "Crate has no tests for <function>."
- **Reviewer → Fixer**: "REVIEW BLOCKED on <branch>: <blockers>."
- **Reviewer → Improver-docs**: "Same pattern in 3 PRs — needs an ADR."
- **Fixer → Scout**: "Root cause deeper than expected. Need a proper slice."
- **Fixer → Improver-devex**: "Error message at <file:line> was misleading."
- **Any → Any**: If you know who should hear it, tell them.

## 3. GitHub-Native Tracking

Use GitHub as the source of truth for work state.

### PR Labels
- `swarm-core` — primary task implementation
- `swarm-improve-docs` — documentation improvement
- `swarm-improve-tests` — test quality improvement
- `swarm-improve-devex` — developer experience improvement
- `swarm-improve-infra` — infrastructure improvement

### Issue Labels
- `swarm-discovered` — found by a swarm agent during work (a fresh agent picks it up)
- `swarm-architectural` — needs architectural decision / ADR (user weighs in)

### PR Description Template
```
## Summary
<what and why>

## Agent
<agent-type that created this>

## Handoff
<link to .ops/handoffs/<branch>.md if applicable>

## Verification
- $FMT_CMD — clean
- $LINT_CMD — clean
- $TEST_CMD — N pass
```

### Querying Swarm State
```bash
# Open core work
gh pr list --state open --label "swarm-core"
# Side fixes waiting for merge
gh pr list --state open --label "swarm-side-fix"
# Discovered issues
gh issue list --label "swarm-discovered"
# Architectural decisions needed
gh issue list --label "swarm-architectural"
# Recent merges
gh pr list --state merged --limit 20 --json number,title,mergedAt
```

## 4. Metrics

After completing any task, append to `.ops/swarm-metrics.jsonl`:

```json
{"ts":"<ISO-8601>","agent":"<name>","type":"<build|review|fix|merge|improve|scout>","branch":"<branch>","outcome":"<green|red|blocked|merged>","duration_hint":"<fast|medium|slow>","side_prs":<N>,"issues_created":<N>,"notes":"<one line>"}
```

Append-only. The lead/ops analyzes periodically for patterns.
Use cargo xtask swarm-summary .ops-perl-lsp --since 24h --limit 10
for the daily status window and --since 7d for report rollups.
Add `--format json` when a downstream script or agent needs machine-readable output.

## 5. Agent Self-Improvement

When your agent definition is wrong or incomplete, write a patch:

`.ops/agent-patches/<your-agent-name>.md`:
```markdown
# Patch: <agent-name>
## Problem — what was wrong/missing
## Suggested Change — specific edit
## Evidence — branch, error, time wasted
```

Bootstrapper integrates validated patches during `--refresh`. User reviews.

## 6. Dedup

Before starting work:
1. `.claude/swarm-state/completed-slices.md` — done already?
2. `.claude/swarm-state/known-pitfalls.md` — known trap?
3. `.claude/swarm-state/discovered-issues.md` — already flagged?
4. `.claude/swarm-state/findings.json` — durable swarm conclusion already recorded?
5. `gh issue list --label "swarm-discovered"` — already an issue?
6. `gh pr list --state open` — already a PR?

After completing:
1. `completed-slices.md` — `in-progress` (scout) or `merged` (ops)
2. `known-pitfalls.md` — if you learned a reusable lesson
3. `findings.json` — if the conclusion changes how the swarm should operate or document itself
4. `swarm-metrics.jsonl` — always

## 7. User Interaction

The user is an **observer** who checks in every few hours or daily.

- Do NOT wait for approval. Ship PRs, merge green, fix failures, create issues.
- DO leave a clear trail: PRs, issues, handoffs, metrics.
- When user checks in, lead summarizes: PRs merged, issues created, blockers, trends, patches pending.
- If genuinely ambiguous, create an issue labeled `swarm-architectural` and move on.

## 8. Research (Don't Guess — Look It Up)

When you need external facts — Perl syntax rules, LSP protocol details, crate APIs, CPAN module behavior — spawn a research agent instead of guessing or spending your own context on web searches:

```
Agent(prompt: "Research: <specific question>", run_in_background: true, name: "research-<topic>")
Agent(prompt: "Look up docs: <API or protocol section>", run_in_background: true, name: "docs-<topic>")
Agent(prompt: "Verify: <claim to cross-check>", run_in_background: true, name: "verify-<topic>")
```

Three research agents:
- **research-web** — general web search → condensed answer with sources
- **research-docs** — fetch upstream docs (docs.rs, perldoc, LSP/DAP spec)
- **research-verify** — cross-check a specific claim against authoritative sources

These run in background. You get a condensed answer without polluting your context with search results. Use them aggressively — verified facts are always better than assumptions.

## 9. Handoff Efficiency

Each stage reads the PREVIOUS stage's output, not the original source:
- Builder reads handoff (not 10 source files)
- Reviewer reads builder briefing (not cold diff)
- Improvers read "Lessons Learned" sections

Include in handoffs: code excerpts, error messages, decision rationale, file:line refs.

## 9. Learning Loop

The swarm writes to five persistence layers, each with different lifetimes:

| Layer | Lifetime | Location | What |
|-------|----------|----------|------|
| **Handoffs** | Until merge | `.ops/handoffs/` | Context transfer: scout→builder→reviewer |
| **Runtime** | Current session | `.ops/` | metrics, agent-patches, salvage |
| **Knowledge** | Across sessions | `.claude/swarm-state/` | findings, known-pitfalls, completed-slices, discovered-issues, queue |
| **GitHub** | Permanent | Issues, PRs, labels | Work items, discoveries, architectural decisions |
| **Memory** | Across sessions | Claude Code memories | Critical lessons future sessions need |

### When to write Claude Code memories

The **lead** should write memories for things that matter ACROSS SESSIONS:
- Feedback memory: "Parser-core tests flake above RUST_TEST_THREADS=2" (so future sessions configure correctly)
- Project memory: "Dual indexing chosen because single-index missed 30% of cross-file references" (architectural context)
- Project memory: "After swarm cycle on 2026-03-15: 30 PRs merged, parser corpus improved from 51% to 55%" (progress tracking)

Don't write memories for ephemeral state (which PRs are open, which slices are in progress) — that's in the ops files and GitHub.

### Flow
1. **Fixers** → `known-pitfalls.md` → scouts/builders avoid traps
2. **All agents** → `discovered-issues.md` → scouts pick up pre-investigated leads
3. **Coordinators + improver** → `findings.json` → durable control-plane conclusions survive beyond the session
4. **All agents** → `swarm-metrics.jsonl` → lead spots patterns
5. **Failing agents** → `agent-patches/` → bootstrapper improves definitions
6. **Improver-docs** → ADRs and docs from handoff lessons
7. **Improver-devex** → fixes friction from handoff lessons
8. **Merger** → analyzes metrics, reports patterns
9. **All agents** → GitHub issues/labels for permanent visibility
10. **Lead** → Claude Code memories for cross-session knowledge

The system gets better with each cycle AND each session.
