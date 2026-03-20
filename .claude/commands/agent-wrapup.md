---
description: Final step for every agent — retrospective, documentation, and clean handoff
---

# Agent Wrapup

Every agent invokes this as their last action before stopping.
Captures what was learned, documents the ending state, and ensures
clean handoff to whoever picks up next.

## Steps

1. **Summarize what you did.** One paragraph. What changed, what was created,
   what was decided.

2. **Document the ending state.** Where did things land?
   - For scouts: issue URL, what's ready, what needs plan review
   - For builders: PR URL, what passes, what doesn't yet
   - For reviewers: approval status, follow-up issues filed
   - For ops: what merged, what's still in queue, master status

3. **Retrospective — what did you learn?** This is the most valuable part.
   Write 2-3 sentences about:
   - What was harder or easier than expected?
   - What would you do differently next time?
   - What surprised you about the code or the problem?
   - What context would have helped you work faster?

4. **Breadcrumbs for the next agent.** What should whoever picks this up
   next know?
   - What's the logical next step?
   - Are there related issues that should be tackled together?
   - Any gotchas or traps to watch out for?

5. **Update task status.** Mark your tasks as completed with the summary
   from step 1.

## Where to write this

- **Scouts:** Add retrospective to the issue as a closing comment
- **Builders:** Add retrospective to the PR description under "What's next"
- **Reviewers:** Add retrospective to the review comment
- **Ops:** Add retrospective to a brief merge summary comment
- **Improver:** Add retrospective to follow-up issues

## Why this matters

Each agent's retrospective makes the NEXT agent faster. If a scout notes
"the dispatch table in statements.rs is ordered by token kind, not by
frequency — check this first next time," the next scout saves 10 minutes.

These observations compound across cycles. They're the swarm's learning
mechanism.
