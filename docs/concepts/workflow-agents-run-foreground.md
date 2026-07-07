# Workflow agents run foreground; the orchestrator owns background work

**Pattern class**: Multi-agent orchestration / agent lifecycle

## The invariant

A **workflow subagent's execution ends when it emits its final message.** There
is no re-invocation of a finished subagent. Therefore a workflow agent must
never delegate the completion of its own work to a mechanism that notifies
*later* — `run_in_background` tasks, `Monitor` waits, or "I'll be notified when
this finishes." Those are **orchestrator** affordances: only the long-lived main
loop is re-invoked on task completion.

## Corollaries

1. **Workflow-agent prompts must mandate foreground execution** for any
   long-running command (builds, test suites). Give an explicit long timeout and
   instruct: on timeout-kill, re-run the same command (incremental state
   resumes). Never background.

2. **The orchestrator may freely use background tasks** — it *is* the loop that
   gets the completion event. Long verification builds, CI polling substitutes,
   and parallel salvage checks belong here.

3. **A "waiting for background…" result is recoverable, not lost.** The work
   lives in the worktree. The orchestrator salvages: read the diff, rebase onto
   fresh main (verify the tree touches only intended files — a stale base can
   show phantom re-adds), re-verify foreground, push + PR.

4. **Liveness ≠ silence.** Before salvaging a "stalled" worktree, confirm it is
   genuinely idle (no `cargo`/`rustc`/`node` process, worktree idle > 5 min).
   Salvaging a live agent's worktree races it. Use process inspection +
   `git rev-list --count origin/main..HEAD` (authoritative) rather than trusting
   a possibly-buffered `git status`.

## Why it recurs

The affordance names ("background", "monitor", "you'll be notified") read as
generally available, and a capable subagent reaches for them to avoid blocking.
The context boundary — who receives the completion event — is invisible in the
tool description. Make it explicit in the prompt every time.

## Related

- [cache-aware-agent-lanes](cache-aware-agent-lanes.md)
- [orchestrator-substrate-model](orchestrator-substrate-model.md)
- Incident: [2026-07-workflow-agent-background-wait-death](../learnings/2026-07-workflow-agent-background-wait-death.md)
