# Claude Control Plane

This repo runs one swarm, not multiple parallel pack stories.

The canonical runtime surfaces are:

- `.claude/agents/` — agent definitions and role-specific swarm instructions
- `.claude/commands/` — slash entrypoints (step skills, shared ops, domain ops)
- `.claude/settings.json` — shared permissions and hook enforcement

Legacy directories archived to `docs/reference/archive/` during architecture transition.

## Canonical Roster

See [agents/AGENT_CATALOG.md](./agents/AGENT_CATALOG.md) for the full inventory.

### Pipeline Agents
- `scout` (haiku) — investigate, file issues
- `plan-reviewer` (sonnet) — stress-test plans, mark builder-ready
- `builder` (sonnet) — implement from spec
- `reviewer` (haiku) — fast standards check
- `reviewer-deep` (sonnet) — deep correctness check
- `ops` (haiku) — merge queue, CI, post-merge

### Specialized Scouts
- `scout-parser` (haiku) — error buckets, corpus, parser
- `scout-lsp` (haiku) — features.toml, providers, LSP spec
- `scout-dap` (sonnet) — DAP protocol, bridge mode

### Utility
- `research-web` (sonnet) — web search, doc lookup
- `wisdom` (sonnet) — synthesize learnings from issue→PR→merge cycles

## Operating Doctrine

- worktree = write boundary
- worker = context boundary
- commands = slash entrypoints for all agent procedures
- hooks and settings = deterministic enforcement
- GitHub issues and PRs = persistent work state

Every agent should use the local todo or task tool and name the slash
entrypoint for each item. The orchestrator reads the agent catalog and
agent files to route work — no separate flow commands needed. Step skills
provide mechanical instructions for each todo step. Domain ops
(`/parser-fix`, `/verify`, `/corpus-ratchet`, etc.) handle specialized
procedures.

The roster mapping lives in
[agents/AGENT_CATALOG.md](./agents/AGENT_CATALOG.md). It records agent models,
step counts, and roles.

## State

Swarm state lives in GitHub (issues, PRs, labels) and `.ops-perl-lsp/swarm-metrics.jsonl`.
Use `gh issue list`, `gh pr list`, and `/swarm-status` to query current state.
