# Hook Reliability Archaeology

## Question

Did hooks arrive as a clean deterministic control layer, or did the repo have
to debug hooks as their own fragile subsystem?

## Short Answer

The committed history shows both breakthrough and fragility.

Hooks were a major control-plane upgrade, but they were not magically perfect.
The repo had to repair:

- payload handling
- file permissions
- scope mismatches between ADRs and live behavior
- incomplete enforcement coverage

That makes hooks historically important in two different ways:

1. they are the clearest move from advisory prompt prose to deterministic control
2. they themselves became a reliability surface that needed auditing and repair

## 1. The Hook Lineage Includes Explicit Repairs

The hook history is not only additive.

Relevant commits in order:

- `32fc6074d`
  `feat: add pre-tool use hooks for CI validation and command checks`
- `fd2356507`
  `chore: improve PostToolUse and TeammateIdle hooks (#1632)`
- `7f1ca606e`
  `fix(hooks): read teammate idle payload from stdin`
- `e4a089ef4`
  `feat(hooks): add SubagentStart, Stop, PreToolUse, SessionStart hooks`
- `097ae545b`
  `fix(hooks): make subagent-stop.sh executable`

That sequence matters. It shows the repo not only inventing hooks, but also
debugging the mechanics of how hooks actually fire and run.

## 2. The First Fragility Class Is Transport / Wiring

The clearest early repair is commit `7f1ca606e`,
`fix(hooks): read teammate idle payload from stdin`.

That is a small but revealing bug class:

- the hook existed
- the event fired
- but the payload plumbing was wrong enough that the hook needed a direct fix

This is a classic control-plane reliability story. Deterministic boundaries are
only useful if the event data actually arrives in the right shape.

The current
[`.claude/hooks/teammate-idle.sh`](../../../.claude/hooks/teammate-idle.sh)
still makes that transport assumption visible in the implementation: it reads
JSON from stdin and derives teammate identity from it.

## 3. The Second Fragility Class Is Execution Readiness

PR `#1900`,
`fix(hooks): make subagent-stop.sh executable`,
is one of the strongest concrete hook-reliability artifacts.

Its entire purpose is simple:

- `subagent-stop.sh` existed
- but it was committed without executable permission

That is a historically useful finding because it shows a deterministic control
surface failing at the most operational level possible: the hook file was there,
but the system could not reliably execute it as intended.

In other words, the control plane did not fail philosophically. It failed like
real systems fail: on wiring and deployability details.

## 4. The Third Fragility Class Is ADR Drift

The repo's own documentation shows another reliability seam: accepted design
does not always equal fully landed behavior.

[`docs/adr/0032-skill-scoping-and-hook-enforcement.md`](../../../docs/adr/0032-skill-scoping-and-hook-enforcement.md)
describes a stronger hook regime than the current live implementation:

- `TaskCompleted` should enforce metrics compliance
- `SubagentStart` should auto-inject coding standards and pitfalls
- `TeammateIdle` should block idle state with unfinished work

But the current committed runtime is narrower:

- [`.claude/hooks/task-completed.sh`](../../../.claude/hooks/task-completed.sh)
  only checks `cargo fmt --all -- --check`
- [`.claude/settings.json`](../../../.claude/settings.json)
  registers `SubagentStart` as a reminder echo, not a full state-injection hook
- [`.claude/hooks/teammate-idle.sh`](../../../.claude/hooks/teammate-idle.sh)
  deduplicates idle notifications rather than enforcing queue ownership

This is not a flaw in the archaeology. It is one of the main findings: the hook
system had to live with partial implementation and documentation drift while it
was still maturing.

## 5. The Repo Later Opens Infrastructure Debt About Hook Coverage

By March 19, 2026, missing hook coverage is itself tracked as swarm-infra debt.

Issue `#2154` is titled:

- `swarm-infra: Hook enforcement incomplete — verify-build and other skills can't gate completion`

The body is unusually explicit:

- `task-completed.sh` only checks formatting
- it does not verify `/verify-build`
- it does not enforce clippy or tests
- agents can still mark tasks complete without full verification

That issue is historically important because it proves the repo understood the
difference between:

- hooks existing
- hooks being sufficient

By this point, hook incompleteness is not hidden technical debt. It is a named,
discoverable, routed control-plane problem.

## 6. Hook Reliability Also Connects To Ownership Boundaries

The hook story is not only "make hooks stronger."

PR `#1723`,
`refactor(swarm): canonicalize agent roster and hook surfaces`,
removes live `WorktreeCreate` and `WorktreeRemove` registrations and documents
that those hooks should only be wired when they own the full lifecycle
contract.

That is another form of reliability work:

- not every available hook should be live
- a partially-owned lifecycle boundary is itself a risk

So the repo improves hook reliability both by fixing broken hooks and by
refusing to over-own boundaries it cannot yet control safely.

## 7. What Makes This Distinctive

Many projects would tell a cleaner story:

- hooks landed
- hooks solved prompt drift

This repo preserves the messier and more believable one:

- hooks landed
- hooks improved control dramatically
- hook payload handling had bugs
- hook files had deployability mistakes
- accepted ADRs outran the live implementation
- the swarm later opened explicit debt about incomplete enforcement

That is a better early-AI-age artifact because it preserves the engineering
cost of making "deterministic control" actually reliable.

## 8. Strongest Evidence-Backed Claims

1. Hook reliability is part of the repo's history, not an incidental footnote.
2. The hook timeline includes explicit fixes for payload handling and execution
   readiness, not only feature additions.
3. PR `#1900` is a sharp proof that even the deterministic control plane can
   fail on mundane operational details like executable bits.
4. ADR `0032` and the live hook implementations do not fully match, which makes
   hook drift itself an archaeology subject.
5. Issue `#2154` proves the repo later treated incomplete hook coverage as
   explicit swarm-infrastructure debt.
6. Hook reliability also includes restraint: the repo deliberately avoids
   wiring some lifecycle hooks until it can own them fully.

## See Also

- [HOOK_CONTROL_ARCHAEOLOGY.md](HOOK_CONTROL_ARCHAEOLOGY.md)
- [CONTROL_PLANE_SELF_REPAIR_ARCHAEOLOGY.md](CONTROL_PLANE_SELF_REPAIR_ARCHAEOLOGY.md)
- [CONTROL_PLANE_REPAIR_CHAIN_ARCHAEOLOGY.md](CONTROL_PLANE_REPAIR_CHAIN_ARCHAEOLOGY.md)
- [HYBRID_CONTROL_PLANE_ARCHAEOLOGY.md](HYBRID_CONTROL_PLANE_ARCHAEOLOGY.md)
- [OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md](OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md)
