---
description: Wisdom step 3 — write findings to the right place
user-invocable: false
---

# Wisdom: Document

Put your findings where they'll have the most impact.

## Before editing control-plane files

If your findings require editing `.claude/agents/`, `.claude/commands/`, or `CLAUDE.md`, acquire the lock first:

```bash
AGENT_ID="wisdom-<issue-number>"   # e.g., wisdom-2566
scripts/control-plane-lock.sh acquire "$AGENT_ID"
```

If acquire fails (another agent holds the lock), do NOT retry in a loop. File your `gh issue comment` and `crates/*/CLAUDE.md` updates without the lock — those are safe. Report the contention to the orchestrator for resolution.

Note: Edits to `crates/*/CLAUDE.md` do NOT need the lock — each crate is isolated. Only `.claude/agents/`, `.claude/commands/`, and the root `CLAUDE.md` require coordination.

## Where findings go

**Process improvements** → comment on the issue or PR:
```bash
gh issue comment <number> --body "## Wisdom Review
<your process findings — what worked, what to change>"
```

**Code patterns** → update the crate's CLAUDE.md if relevant:
If you found something about how a crate works that future agents
should know, add it to the crate's CLAUDE.md.

**Recurring patterns** → file a swarm improvement issue:
If the same kind of fix keeps coming up, or the same pipeline step
keeps being a bottleneck:
```bash
gh issue create --title "swarm: <pattern observed>" --body "<analysis>" --label "infrastructure"
```

**Agent skill improvements** → suggest updates:
If a step skill is missing guidance that would have helped, note the
specific skill and what to add.

## Rules

- Be specific. "Process could be better" is useless. "The scout's test
  spec didn't account for nested ternary which cost the builder 20 min"
  is actionable.
- Write for the next agent, not for a report. What would help them?
- One finding per location. Don't dump everything into one comment.
- **Close the learning loop.** Process insights should flow back to agent
  definitions and skill files, not just GitHub comments. If you found that
  scouts keep missing X, recommend updating scout-locate to check for X.
