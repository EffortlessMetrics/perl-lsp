---
description: Wisdom step 3 — write findings to the right place
---

# Wisdom: Document

Put your findings where they'll have the most impact.

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
