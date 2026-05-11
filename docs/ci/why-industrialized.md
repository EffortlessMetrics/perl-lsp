# Why this CI architecture exists

perl-lsp operates in **Industrialized AI mode** (the fourth step of the
four-mode AI-development progression).

| Mode | Who writes | Who reviews | PR size | Daily volume |
|---|---|---|---|---|
| Suggestions | Human | Human | ~76 lines | A few |
| Assisted | AI | Human | ~500 lines | Dozens |
| Native | AI | AI | 2,000–20,000 lines | Hundreds |
| **Industrialized** | AI | AI | Continuous | **1,000+ PRs** |

Reference: [Assisted, Native, Industrialized](https://effortlesssteven.com/assisted-native-industrialized/).

At ~1000+ PRs/day with agents on both sides, verification cost is
already higher than LLM cost. The CI architecture in this repo is a
direct response to that operating mode:

| Choice | What it's responding to |
|---|---|
| Scoped per-crate / per-domain gates | Full-workspace builds × 1000 PRs/day are infeasible |
| LEM (Learned Estimate Model) budgeting | Predictive cost-per-PR is how the CI budget gets spent intelligently at scale |
| ripr as static mutation-exposure analysis | Shift-left the same signal mutation testing catches; mutation as runtime backstop on the residual |
| Claim boundaries on every evidence lane | Ambiguous claims compound into systemic mistrust faster at industrialized volume |
| Rail-burndown docs + builder-ready issue ladders | Coworker agents need specs, not chat history; specs-for-agents > prose-for-humans |
| Coworker-agent lanes (codex / factory-droid / claude / aider) | Orchestration depends on each agent having queued builder-ready work |

Without the framework, these choices look like over-engineering. With it,
they're the only sustainable architecture for this throughput.

## Related

- [`ripr.md`](ripr.md) — static mutation-exposure analysis doctrine
- [`codecov-rollout.md`](codecov-rollout.md) — coverage claim boundaries
- [`gate-policy-economics.md`](gate-policy-economics.md) — scoped gate policy
- [`lem-budgeting.md`](lem-budgeting.md) — Learned Estimate Model
