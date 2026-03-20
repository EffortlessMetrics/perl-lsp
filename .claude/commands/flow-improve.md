---
description: "Flow: scan recent merges for quality improvements"
---

# Flow: Improve

Scan recently merged PRs for quality gaps and file follow-ups.

## Steps

1. Spawn the improver agent:
   ```
   Agent(
     subagent_type: "improver",
     prompt: "Scan recent merges for quality gaps. Follow your todo list.",
     model: "sonnet",
     name: "improver"
   )
   ```

2. Improver follows its 4-step todo:
   scan → classify → act → health-check

3. For small fixes (<10 lines): creates PRs directly
   For larger gaps: files well-specified issues via `/scout-report`

## What a successful flow produces

- Follow-up issues for quality gaps
- Small fix PRs for trivial improvements
- Health check results
- Clear "what's next" for each finding
