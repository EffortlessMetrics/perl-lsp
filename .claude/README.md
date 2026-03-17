# Claude Control Plane

This repo runs one swarm, not multiple parallel pack stories.

The canonical runtime surfaces are:

- `.claude/agents/` — who owns each lane
- `.claude/skills/swarm/` — canonical swarm control-plane skill
- `.claude/commands/` — compatibility slash-command layer and operator entrypoints
- `.claude/settings.json` — shared permissions and hook enforcement
- `.claude/swarm-state/` — durable queue and dedup state

The pack under `docs/handoff/swarm-pack/` is a derived export, not a co-equal
design source.

## Canonical Roster

Persistent coordinators:

- `scout`
- `builder`
- `reviewer`
- `ops`
- `improver`

Reusable workers:

- `bootstrapper`
- `fixer`
- `validator`
- `pr-responder`
- `research-web`
- `research-docs`
- `research-verify`

See [agents/README.md](./agents/README.md) for the roster contract.

## Operating Doctrine

- worktree = write boundary
- worker = context boundary
- skills = reusable procedure and durable instruction
- commands = thin operator entrypoints and compatibility shims
- hooks and settings = deterministic enforcement
- handoffs, receipts, worktrees, and PRs = volatile execution state

Every agent should keep a local todo list and name the slash entrypoint for
each todo item. That keeps procedure attached to the current slice instead of
floating in remembered context. In the live repo today, `/swarm` is the main
skill-backed control-plane entrypoint; many other reusable procedures are still
command-backed while they remain lightweight.

## Compatibility Note

Older `swarm-*` agent files remain as donor material during the transition, but
new docs, prompts, and commands should reference the canonical names first.
