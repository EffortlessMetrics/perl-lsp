# Claude Control Plane

This repo runs one swarm, not multiple parallel pack stories.

The canonical runtime surfaces are:

- `.claude/agents/` — agent definitions and role-specific swarm instructions
- `.claude/commands/` — slash entrypoints (step skills, shared ops, domain ops)
- `.claude/settings.json` — shared permissions and hook enforcement

Legacy directories archived to `docs/reference/archive/` during architecture transition.

## Agent Roster

See [agents/AGENT_CATALOG.md](./agents/AGENT_CATALOG.md) for the full inventory.

### Team Agents (TeamCreate — long-running coordinators)
- `sector-lead` (sonnet) — manages a sector, spawns workers, tracks progress

### Worker Agents (Agent() — worktree-isolated, one task, exit)
- `scout` (haiku), `scout-parser` (haiku), `scout-lsp` (haiku), `scout-dap` (sonnet)
- `plan-reviewer` (sonnet), `builder` (sonnet)
- `reviewer` (haiku), `reviewer-deep` (sonnet)
- `ops` (haiku), `research-web` (sonnet), `wisdom` (sonnet)

## Operating Doctrine

- worktree = write boundary
- worker = context boundary
- commands = slash entrypoints for all agent procedures
- hooks and settings = deterministic enforcement
- GitHub issues and PRs = persistent work state

Two interfaces for two scales: Agent() spawns worktree-isolated workers
for individual tasks. TeamCreate with sector leads coordinates workers at
scale (10+ tasks). Step skills provide mechanical instructions for each
todo step. Domain ops (`/parser-fix`, `/verify`, `/corpus-ratchet`, etc.)
handle specialized procedures.

The roster mapping lives in
[agents/AGENT_CATALOG.md](./agents/AGENT_CATALOG.md). It records agent models,
step counts, and roles.

## State

Swarm state lives in GitHub (issues, PRs, labels) and `.ops-perl-lsp/swarm-metrics.jsonl`.
Use `gh issue list`, `gh pr list`, and `/swarm-status` to query current state.
