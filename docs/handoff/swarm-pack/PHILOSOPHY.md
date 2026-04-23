# The Swarm Philosophy

Why continuous agent swarms work, what they change about software development, and how to think about them.

## Core Essays

| Essay | Key Insight |
|-------|------------|
| [Cheap Passes Beat Expensive Passes](philosophy/cheap-passes.md) | Five 30-second specialized reviews > one 20-minute general review. Make quality checks abundant, not expensive. |
| [Every Agent Is a Scout](philosophy/every-agent-is-a-scout.md) | Every agent reports what it sees. Codebases become self-aware of their own gaps as a side effect of normal work. |
| [Context Efficiency](philosophy/context-efficiency.md) | Handoff files, skills over reads, minimal prompts. The #1 performance lever: don't re-read what the previous agent already condensed. |
| [Compound Improvement](philosophy/compound-improvement.md) | ~20% of capacity always goes to improvement. Each cycle is cheap; the compound effect is transformative. |
| [Self-Improving Systems](philosophy/self-improving-systems.md) | Five learning loops: pitfalls, discoveries, metrics, agent patches, ADRs. The swarm gets better at getting better. |
| [GitHub-Native Operations](philosophy/github-native-swarm.md) | Issues for discoveries, PRs for work products, labels for categorization, auto-merge for throughput. GitHub is the dashboard. |

## The One-Sentence Version

**Throw many cheap passes of improvement at a codebase continuously, with agents that learn from each other and leave the codebase better than they found it.**

## The Core Principles

1. **Cheap passes > expensive passes.** Five specialized 30-second checks beat one 20-minute general review.
2. **Every agent is a scout.** Discoveries outside scope become GitHub issues for fresh agents.
3. **Handoffs carry context.** Next agent reads the summary, not the source.
4. **Improvement is continuous.** ~20% always goes to docs, tests, devex, infra.
5. **The system learns.** Pitfalls, metrics, patches, ADRs — each cycle is smarter than the last.
6. **GitHub is the dashboard.** Visible, persistent, searchable. No hidden state.
7. **Ship small, ship often.** 5-50 line PRs, validated and merged continuously.
8. **Validate what you ship.** Post-merge checks catch regressions before they compound.
9. **Leave it better than you found it.** Every session: better documented, better tested, cleaner, more observable.
10. **Trust the compound effect.** Each cycle is small. The trajectory is transformative.
