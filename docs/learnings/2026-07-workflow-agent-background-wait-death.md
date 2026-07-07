---
tags: [multi-agent, workflow, orchestration, background-tasks, agent-lifecycle, salvage, worktree, stochastic-pipeline]
repos: [perl-lsp-swarm]
related: ["#3355", "#3397", "#3404", "#3412", "#3413"]
portable: true
article_asset: true
search_terms: [workflow agent stalled, wait for monitor, background build lost, run_in_background workflow, agent ended turn waiting, salvage worktree, orchestrator salvage, Get-Process before salvage, background-wait death, monitor completion event]
---

# Workflow agents die when they wait on a background build; the orchestrator must salvage the worktree

**Date**: 2026-07-04
**Hazard class**: Multi-agent lifecycle / self-report reliability
**Portable lesson**: [docs/concepts/workflow-agents-run-foreground.md](../concepts/workflow-agents-run-foreground.md)

## What happened

During a single autonomous campaign, **five** workflow-spawned builder agents
ended their turn with a message like *"I'll wait for the monitor completion
event"* or *"the lib test suite is compiling in the background; I'll be notified
when it completes."* In every case the agent had done substantial, correct work
(one had captured a validated ~13× latency receipt; another had all four
CodeRabbit threads fixed), then **stalled and returned control** while a
`run_in_background` cargo build or a `Monitor` task was still pending.

A workflow subagent's lifecycle **ends when it stops its turn**. Background-task
and Monitor completion notifications are delivered to the *orchestrator* (the
main loop), not back into a finished subagent. So the subagent's `result` was
literally the string "waiting for the background build" — its real work sat
uncommitted in the worktree, unpushed, invisible to any PR.

Affected lanes this session: #3355 ripr fix (v1 died, v2 finished), the #3397
timing lane, the #3404 token/threads work, the Neovim quick-win (#3412), and the
VSIX lane (#3413). Every one was recovered from its worktree.

## Why it happens

`run_in_background: true` (and skill-level `Monitor` waits) are an
**orchestrator affordance**: the main loop is re-invoked when the task
completes. A workflow subagent has no such re-invocation — once it emits its
final message the run is over. Telling the subagent "you'll be notified" is
false for that execution context.

## The fix (two parts)

**1. Prompt rule (prevention).** Every workflow-agent prompt that compiles Rust
must state, verbatim:

> Run ALL cargo commands in the FOREGROUND with an explicit long timeout (up to
> 600000 ms) and wait inline. NEVER use `run_in_background`; NEVER set up a
> Monitor and end your turn. If a build is killed by the tool timeout, re-run
> the same command — incremental compilation resumes where it stopped. Prefix
> with `RUSTC_WRAPPER=""` under concurrent worktree builds (sccache returns bare
> exit-1 on unrelated crates otherwise).

**2. Orchestrator salvage (recovery).** When a workflow returns a
"waiting for background…" result, do **not** treat the work as lost. The
worktree diff is almost always intact:

- Inspect `.claude/worktrees/<runId>-1` — read the diff, the captured receipt,
  the test output the agent already ran.
- Commit the real files, `git fetch origin main` + rebase (branches cut earlier
  in a fast campaign go stale — a pre-merge base can show phantom re-adds of
  files a later PR deleted; **verify the rebased tree touches only the intended
  files** before pushing).
- Re-run the verification yourself (foreground, `RUSTC_WRAPPER=""`), then
  push + PR.

## The critical refinement: check liveness before salvaging

"Silent for a while" is **not** the same as "stalled." Twice this session a lane
looked idle (no recent worktree edit, empty `git log`) but was actually mid-build
in a long compile. Salvaging a *live* agent's worktree races it and corrupts
both. Before salvaging, confirm the lane is genuinely idle:

```powershell
(Get-Process -Name cargo,rustc,node -EA SilentlyContinue).Count   # 0 build procs?
git -C <worktree> rev-list --count origin/main..HEAD               # authoritative ahead-count
```

Only salvage when there are **no build processes AND the worktree has been idle
> 5 min**. (Bash `git -C <worktree> status` misreported clean/dirty state here;
PowerShell `git rev-list --count origin/main..HEAD` was authoritative.)

## Guardrail

A workflow that returns a "waiting…" result is an **incomplete-but-recoverable**
signal, never a failure and never a done. Read the worktree, verify liveness,
salvage or wait accordingly.
