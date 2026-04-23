# Agentic Development Landscape (2026)

*Explore agent research report. Data current as of March 2026.*

---

## Summary

The agentic coding tool market consolidated significantly in 2025. Every major AI lab and IDE vendor now ships an agent mode. Cost-per-PR benchmarks are not publicly available from any vendor — perl-lsp would be the first project to publish them.

---

## Tool Inventory

### Claude Agent SDK (Anthropic)

- **Default model**: Sonnet 4.5
- **Key capabilities**: Subagents, hooks, checkpoints, worktree isolation
- **Architecture**: Agents can spawn subagents; hooks enforce pipeline gates; checkpoints enable resumable long tasks
- **Relevance to perl-lsp**: Current swarm infrastructure — the SDK powering this development model

### Codex CLI (OpenAI)

- **Models**: o3, o4-mini
- **Pricing**: $1.50 / $6.00 per million tokens (input/output, approximate)
- **Positioning**: Lightweight CLI agent; no IDE dependency
- **Architecture**: Terminal-native; designed for scripted, non-interactive workflows
- **Status**: GA as of 2025

### Copilot Coding Agent (GitHub/Microsoft)

- **GA date**: September 2025
- **Key feature**: Mission Control dashboard for managing parallel agent tasks
- **Architecture**: GitHub-native; integrates directly with Issues and PRs
- **Positioning**: Developer workflow integration over raw coding capability
- **Status**: Generally available

### Windsurf (Codeium)

- **Agent**: Cascade
- **Model**: SWE-1.5 (proprietary)
- **Performance**: 950 tokens/sec inference
- **Pricing**: $15/seat/month
- **Architecture**: IDE-native agent with context-aware file editing
- **Positioning**: Speed-focused; targets developers who find Cursor/Copilot too slow

### Cursor

- **Agent**: Agent mode
- **Model**: Claude 3.5 Sonnet (default as of research date)
- **Pricing**: $20/seat/month
- **Architecture**: IDE fork; deep editor integration
- **Positioning**: Power-user focused; most complete context window usage of IDE tools

---

## Market Observations

### Cost-per-PR Benchmarks

No vendor publishes cost-per-PR data. This is a significant gap:

- Vendors optimize for capability claims, not efficiency metrics
- "Agentic" is used as a marketing term without standardized measurement
- perl-lsp's swarm model tracks cost-per-PR internally and would be the first to publish verified numbers

### Architectural Patterns

| Pattern | Tools Using It |
|---------|---------------|
| IDE-native agent | Cursor, Windsurf, Copilot |
| CLI agent | Codex CLI, Claude Agent SDK |
| Worktree isolation | Claude Agent SDK (perl-lsp swarm) |
| Subagent spawning | Claude Agent SDK |
| Mission Control / dashboard | Copilot Coding Agent |

### Pricing Landscape

| Tool | Price |
|------|-------|
| Copilot Coding Agent | Included in GitHub Copilot subscription |
| Cursor | $20/seat/month |
| Windsurf | $15/seat/month |
| Codex CLI | Pay-per-token ($1.50–$6.00/M) |
| Claude Agent SDK | Pay-per-token (API pricing) |

---

## Differentiation of perl-lsp Swarm Model

The perl-lsp swarm does not map neatly to any single product:

- **vs. Copilot/Cursor/Windsurf**: Those are single-agent IDE tools. perl-lsp runs 50-100 parallel agents with worktree isolation and a pipeline (Scout → Plan-Review → Build → Review → Green → Merge).
- **vs. Codex CLI**: Codex is single-task CLI. perl-lsp uses multi-stage quality gates with plan-reviewers improving scout specs before builders execute.
- **vs. Claude Agent SDK alone**: perl-lsp adds the swarm operating model on top — agent definitions, catalog routing, corpus ratchets, and CI integration.

The perl-lsp swarm is closer to an agentic CI/CD system than an IDE assistant.

---

## What Does Not Exist (as of March 2026)

- No vendor publishes cost-per-PR benchmarks
- No tool demonstrates 90%+ CPAN corpus coverage with a native parser
- No open-source project has documented a 100-agent parallel swarm development model with full receipts
- No Perl-specific agentic tooling from any vendor

---

*Source: Explore agent findings, March 2026.*
