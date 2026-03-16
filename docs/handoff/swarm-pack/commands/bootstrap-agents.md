---
description: Discover codebase and generate domain-specific swarm agents
argument-hint: "[--dry-run] [--domain <name>] [--refresh]"
---

# Bootstrap Agents

Discover the codebase structure and generate domain-specific agent definitions. Context: **$ARGUMENTS**

## When to Use
- **First time**: after `swarm-pack/setup.sh` installs portable agents — run this to add domain agents
- **Refresh**: when the codebase structure changed (new packages, reorganization)
- **Single domain**: `--domain <name>` to regenerate agents for one domain only

## What It Does

1. **Discovers** your repo: packages, tests, errors, standards, CI, docs
2. **Identifies** natural domains (package families, layers, feature areas)
3. **Generates** 3-5 agent files per domain: fix, test, scout, explorer
4. **Customizes** the portable agents with repo-specific details
5. **Creates** `.claude/agents/AGENT_CATALOG.md` for orchestrator reference

## Process

Launch the `bootstrapper` agent:

```
Agent(
  subagent_type: "bootstrapper",
  prompt: "Discover this codebase and generate domain-specific agents. $ARGUMENTS.
Write agents to .claude/agents/.
Update portable agents with repo-specific details.
Create .claude/agents/AGENT_CATALOG.md.
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
