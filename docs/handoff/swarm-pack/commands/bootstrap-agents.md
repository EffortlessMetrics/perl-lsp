---
description: Discover codebase and generate domain-specific swarm agents
argument-hint: "[--dry-run] [--domain <name>] [--refresh]"
---

# Bootstrap Agents

Discover the codebase structure and generate domain-specific agent definitions. Context: **$ARGUMENTS**

## When to Use
- **First time after import**: once the derived `swarm-pack` starter is copied into a new repo and you want repo-specific domain agents
- **In perl-lsp itself**: prefer the tracked repo `.claude/` surfaces; this pack is a derived export
- **Refresh**: when the codebase structure changed (new packages, reorganization)
- **Single domain**: `--domain <name>` to regenerate agents for one domain only

## What It Does

1. **Discovers** your repo: packages, tests, errors, standards, CI, docs
2. **Identifies** natural domains (package families, layers, feature areas)
3. **Generates** 3-5 agent files per domain: fix, test, scout, explorer
4. **Customizes** the imported starter agents with repo-specific details
5. **Creates** `.claude/agents/AGENT_CATALOG.md` and archived agent roster files for orchestrator reference

## Process

Launch the `bootstrapper` agent:

```
Agent(
  subagent_type: "bootstrapper",
  prompt: "Discover this codebase and generate domain-specific agents. $ARGUMENTS.
Write agents to .claude/agents/.
Update the repo-local coordinator and worker roster when the pattern is reusable.
Create .claude/agents/AGENT_CATALOG.md and .claude/agents/agent-roster.json.
Keep role framing and todo structure in the agent files; keep the mechanical
substep instructions in skills or commands so agents can load them when those
substeps become relevant.
Target ~25-35 domain agents.",
  mode: "auto"
)
```

## After Bootstrap

1. Review generated agents in `.claude/agents/`
2. Check `AGENT_CATALOG.md` for the full inventory
3. Verify any `$PLACEHOLDER` values were filled in
4. Test with `/swarm all` to start the swarm

## Modes

### `--dry-run`
Discover and report what would be generated, but don't create files.

### `--domain <name>`
Only generate/refresh agents for a specific domain.

### `--refresh`
Re-discover and update existing agents. Won't overwrite manual customizations (checks for `# CUSTOMIZED` marker at top of file).
