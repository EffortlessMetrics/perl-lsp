# Hook Control Archaeology

## Question

When did the repo stop relying on prompt discipline alone and start enforcing
parts of the swarm mechanically?

## Short Answer

The shift is visible in two stages.

- `2025-09-06` introduces early `PreToolUse` hooks in
  [`.claude/settings.json`](../../../.claude/settings.json).
- `2026-03-15` to `2026-03-16` turns hooks into a first-class control surface
  for the current swarm, with lifecycle events, dangerous-command blocking,
  completion gates, and teardown metrics.

The repo therefore did not invent hooks in March 2026. What March adds is a
broader hook regime that becomes part of the live swarm operating system rather
than a narrow configuration trick.

## 1. Early Hook Evidence Exists Before The Current Swarm

The first strong committed marker is commit `32fc6074d` on `2025-09-06`,
`feat: add pre-tool use hooks for CI validation and command checks`.

That matters because it disproves an overly simple story where hooks are only
an Era 5 invention. Deterministic interception starts earlier than the modern
skill-and-state control plane.

The current history of
[`.claude/settings.json`](../../../.claude/settings.json) preserves that lineage:

- `32fc6074d` on `2025-09-06`
- `bda6c11bd` on `2025-09-24`
- `9cc2d3b9a` on `2026-03-15`
- `fd2356507`, `e4a089ef4`, `d17b84393`, and `bf7407d24` on `2026-03-16`

So the archaeology is not "hooks suddenly appear." It is "hooks start early,
then expand into the live swarm lifecycle."

## 2. Q3 Still Mostly Enforced By Instruction, Not Interception

The canonical Q3 swarm pack in
[`.claude/agents4/issue-to-draft.md`](../../../.claude/agents4/issue-to-draft.md)
already has quality and review discipline.

It tells agents to work from clear requirements, generate gated output, and set
check runs after non-trivial changes. That is real discipline, but it still
lives primarily in phase-pack Markdown and GitHub habits.

The distinction matters:

- Q3 instructions tell agents what they must do.
- hooks later intercept runtime events and reject or mutate behavior directly.

That is why Q3 can look disciplined while still feeling expensive in attention
terms. Much of the discipline still depended on agent compliance.

## 3. March 15-16, 2026 Turns Hooks Into A Live Control Plane

The current hook surface comes together across a tight two-day window:

1. `9cc2d3b9a` on `2026-03-15`
   `feat(swarm): continuous swarm infrastructure with agent teams (#1553)`
2. `fd2356507` on `2026-03-16`
   `chore: improve PostToolUse and TeammateIdle hooks (#1632)`
3. `e4a089ef4` on `2026-03-16`
   `feat(hooks): add SubagentStart, Stop, PreToolUse, SessionStart hooks`
4. `d17b84393` on `2026-03-16`
   `docs(swarm): codify worktree-first control plane (#1721)`
5. `bf7407d24` on `2026-03-16`
   `refactor(swarm): canonicalize agent roster and hook surfaces (#1723)`

The committed
[`.claude/settings.json`](../../../.claude/settings.json) now wires named hook
events to commands:

- `PostToolUse`
- `TeammateIdle`
- `TaskCompleted`
- `SubagentStart`
- `SubagentStop`
- `PreToolUse`
- `SessionStart`

That is a different operating model than "remember to follow the protocol."
The repo now owns lifecycle boundaries explicitly.

## 4. The Hook Surface Enforces Real Boundaries

The live hooks are not decorative.

### Completion gate

[`.claude/hooks/task-completed.sh`](../../../.claude/hooks/task-completed.sh)
rejects task completion when `cargo fmt --all -- --check` fails.

That is a direct runtime boundary: an agent can think it is done, but the hook
can still block completion.

### Dangerous command blocking

[`.claude/settings.json`](../../../.claude/settings.json) registers a
`PreToolUse` shell hook that rejects commands matching patterns like:

- `git push --force`
- `git reset --hard`
- `rm -rf /`
- `cargo publish`
- `git clean -fd`

That is stronger evidence than any prompt rule because the block happens before
the command runs.

### Teardown telemetry

[`.claude/hooks/subagent-stop.sh`](../../../.claude/hooks/subagent-stop.sh)
writes structured JSONL teardown events including:

- `event`
- `agent_name`
- `agent_type`
- `worktree_path`
- `session_id`

This is important archaeologically because it shows hooks doing bookkeeping and
measurement, not only gates.

### Idle suppression

[`.claude/hooks/teammate-idle.sh`](../../../.claude/hooks/teammate-idle.sh)
tracks first-idle transitions and suppresses repeated notifications.

That is not a quality gate, but it is still deterministic control over swarm
behavior.

## 5. The Repo Explains The Philosophy Explicitly

The docs do not leave the design intent ambiguous.

[`docs/adr/0032-skill-scoping-and-hook-enforcement.md`](../../../docs/adr/0032-skill-scoping-and-hook-enforcement.md)
says prompt instructions are unreliable for behavioral enforcement and that
hooks execute unconditionally.

[`docs/adr/0033-worktree-first-disposable-workers.md`](../../../docs/adr/0033-worktree-first-disposable-workers.md)
says "hooks own guarantees; prompts own judgment" and lists lifecycle hooks as
the mechanical boundaries for provisioning, cleanup, metrics, and completion.

[`.claude/skills/swarm/SKILL.md`](../../../.claude/skills/swarm/SKILL.md) makes
the same control-plane claim in operational language: anything that must always
happen belongs in hooks, not in agent memory.

This is a rare case where the repo both does the thing and documents why it is
doing it.

## 6. The Live Hook Regime Also Shows Its Own Limits

One of the most useful findings here is that the live repo does not fully match
the most ambitious ADR language.

ADR `0032` describes a `TaskCompleted` regime that blocks completion unless
metrics entries exist in `swarm-metrics.jsonl` and a `SubagentStart` hook that
auto-injects standards and pitfalls.

But the current committed
[`.claude/hooks/task-completed.sh`](../../../.claude/hooks/task-completed.sh)
only enforces a formatting gate, and the current registered `SubagentStart`
behavior in
[`.claude/settings.json`](../../../.claude/settings.json) is a reminder string,
not a full state injection layer.

That makes the archaeology stronger, not weaker:

- it proves the repo was actively designing deterministic enforcement
- it proves some parts landed as narrower live implementations than the ADR
  language first imagined

This is exactly the kind of control-plane drift the repo later starts tracking
in [`.claude/swarm-state/findings.json`](../../../.claude/swarm-state/findings.json).

## 7. Hooks Also Define What The Repo Does Not Yet Own

The worktree-first ADR is explicit that `WorktreeCreate` and `WorktreeRemove`
exist as hook boundaries but are intentionally not registered in shared
settings until the repo deliberately wants to own that lifecycle.

That is a useful sign of maturity. The hook system is not only about adding
more automation. It is also about deciding which boundaries should remain under
tool-default behavior.

## 8. The Hook Layer Also Produces Its Own Audit Trail

The March 19, 2026 issue wave shows the swarm treating missing hook coverage as
first-class engineering debt.

Issue `#2154`, opened with the `swarm-discovered` label, is titled:

- `swarm-infra: Hook enforcement incomplete — verify-build and other skills can't gate completion`

That is historically useful evidence because it shows the repo is not merely
adding hooks. It is also using the swarm to discover where hook ownership is
still too narrow or too incomplete.

In other words, by March 2026 the hook layer is important enough that gaps in
hook enforcement are themselves routed through the same discovery-and-repair
system as product bugs.

## 9. Strongest Evidence-Backed Claims

1. Deterministic hook control starts earlier than the current swarm, with
   `PreToolUse` evidence on `2025-09-06`.
2. March 15-16, 2026 is the inflection where hooks become part of the live
   swarm operating system rather than a narrow settings feature.
3. The current hook layer enforces real boundaries: completion gating,
   dangerous-command blocking, teardown metrics, and idle suppression.
4. The repo explicitly documents the philosophy behind the shift: prompts carry
   judgment, hooks carry invariants.
5. The live implementation is intentionally narrower in places than the ADR
   language, which makes hook evolution itself an archaeology subject.
6. By March 19, 2026, missing hook coverage is already being surfaced as
   `swarm-discovered` infrastructure debt rather than being left implicit.

## See Also

- [CONTROL_PLANE_ARCHAEOLOGY.md](CONTROL_PLANE_ARCHAEOLOGY.md)
- [HYBRID_CONTROL_PLANE_ARCHAEOLOGY.md](HYBRID_CONTROL_PLANE_ARCHAEOLOGY.md)
- [SWARM_SURFACE_EVOLUTION.md](SWARM_SURFACE_EVOLUTION.md)
- [SWARM_STATE_ARCHAEOLOGY.md](SWARM_STATE_ARCHAEOLOGY.md)
- [OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md](OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md)
