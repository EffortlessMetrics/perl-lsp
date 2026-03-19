# Claude Control Plane

This repo runs one swarm, not multiple parallel pack stories.

The canonical runtime surfaces are:

- `.claude/agents/` — agent definitions, roster data, and role-specific swarm instructions
- `.claude/skills/` — canonical skill layer for swarm control and core worker procedures
- `.claude/commands/` — slash entrypoints that currently live as command files
- `.claude/settings.json` — shared permissions and hook enforcement
- `.claude/swarm-state/` — durable queue, dedup, pitfalls, and findings state

The pack under `docs/handoff/swarm-pack/` is a derived export, not a co-equal
design source.

Historical directories such as `.claude/agents-compat/`, `.claude/agents2/`,
`.claude/agents3/`, `.claude/agents3 - to update/`, `.claude/agents4/`,
`.claude/agents5/`, and `.claude/agents6/` are intentionally retained as
archival evolution and analysis material. Do not remove them during cleanup
unless a human explicitly asks for that archival content to be deleted.

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

Tracked specialist workers live under `.claude/agents/` for documentation,
quality, review, research, and domain-specific execution. See
[agents/README.md](./agents/README.md) for the role/invocation split and
[agents/AGENT_CATALOG.md](./agents/AGENT_CATALOG.md) for the swarm inventory
summary. Those agent files name the skills/commands to invoke, while the
mechanical substep instructions stay in the skills so they can be loaded when
they become relevant during the run.

## Operating Doctrine

- worktree = write boundary
- worker = context boundary
- skills and commands = interchangeable slash entrypoints unless frontmatter says otherwise
- hooks and settings = deterministic enforcement
- handoffs, receipts, worktrees, and PRs = volatile execution state

Every agent should use the local todo or task tool and name the slash
entrypoint for each item. That keeps procedure attached to the current slice
instead of floating in remembered context. In the live repo today, `/swarm`,
`/swarm-protocol`, `/coding-standards`, `/swarm-priorities`, `/plan-fix`,
`/parser-fix`, and `/verify-build` ship from `.claude/skills/`. Other
operator procedures currently live under `.claude/commands/`. Agents invoke
both the same way unless frontmatter intentionally changes who can call them or
how they run.

The roster mapping lives in
[agents/AGENT_CATALOG.md](./agents/AGENT_CATALOG.md). It records who usually
spawns each tracked agent, where it hands work next, and which slash
entrypoints it should invoke first. The machine-readable roster contract is
kept alongside the agent definitions.

Validate the roster contract with:

```bash
python3 scripts/validate_swarm_agent_roster.py
```

The tracked swarm-state contract lives in
[swarm-state/README.md](./swarm-state/README.md). Use `findings.json` there for
durable control-plane conclusions, `discovered-issues.md` for product leads,
`known-pitfalls.md` for reusable failure lessons, and `completed-slices.md` for
dedup and lifecycle tracking.

## Compatibility Note

Compatibility donor material now lives under
[`.claude/agents-compat/`](./agents-compat/). New docs, prompts, and commands
should treat `.claude/agents/` as the current swarm surface and the older
sibling directories as donor/historical material.
