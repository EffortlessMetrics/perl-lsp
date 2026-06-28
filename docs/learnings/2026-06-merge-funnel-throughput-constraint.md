---
tags: [ci, workflow, throughput, merge-velocity, bottleneck, economics]
repos: [perl-lsp-swarm]
related: ["#1578", "#1574", "#1556", "#1555", "#1512", "#1511", "#1558", "#1583"]
portable: true
article_asset: true
search_terms: [throughput, merge funnel, bottleneck, draft PRs, red checks, merge velocity, CI waste, discovery vs merge, binding constraint, economics, cycle time]
---

# Merge funnel, not discovery, is the binding throughput constraint

**Date**: 2026-06
**Hazard class**: ci / workflow / economics
**Portable lesson**: [docs/concepts/orchestrator-substrate-model.md](../concepts/orchestrator-substrate-model.md)

## What happened

PR-cleanup sweep (haiku scouts over ~18 open PRs) revealed a pattern: ~50% of open PRs are stuck in draft or red-check state. The issues are not discovery (specs are sound, builders know what to do) or code (implementation is locally correct). The issues are: (a) ripr gate evaluation malfunction (draft PRs return ROUTE_RESULT=skipped, evaluator FAILS them), (b) irrelevant CI (docs-only PRs run Rust matrix and fail), (c) pre-open validation (agent-generated titles missing issue refs, validate-title fails). All of these are CI/routing/validation substrate issues, not product code issues.

The observation: **the merge funnel (PR→draft→red→fix) is where PRs are stuck, not the discovery or implementation layer**. The actual builders have work ready; the gates are false-positives or misdirected. This is an infrastructure throughput problem, not a discovery or build problem.

## Why

The pipeline is designed with gates as quality checkpoints: discover → spec → build → review → CI → merge. When gates are working correctly, they catch real bugs and enforce invariants. When gates are broken or misdirected, they become funnel bottlenecks.

**Key insight**: A broken gate is worse than no gate. When a gate is:
- Triggered on non-applicable PRs (docs-only gets Rust matrix)
- Configured incorrectly (ripr skip → fail)
- Lacks a pre-open guard (titles missing issue refs)

...it creates red herring failures that slow the funnel without catching real bugs. The team then spends cycles *fixing the gate itself* instead of doing the work the gate was meant to protect.

The economic dynamics:
- **Discovery cost**: Haiku scouts, plan review → relatively cheap (token-bound)
- **Build cost**: Sonnet builders, tests → moderate (token-bound, but 1-per-issue)
- **Review cost**: Haiku reviewers, deep-review sonnet → moderate
- **CI cost**: Gates running on every PR → multiplied by total PRs open × CI cost per gate
- **Merge cost**: Batch serialization, one merge at a time → bottleneck

When gates are broken, the **CI cost per PR inflates asymptotically** (re-run the gate 5+ times waiting for a fix that doesn't exist). The merge funnel gets clogged, and the team shifts to "unblock CI" instead of "build features."

## Fix

**Partially applied.** The fixes are infrastructure/routing/validation, not code fixes:

1. **ripr gate fix**: Router should emit PASS when ROUTE_RESULT=skipped (draft), not FAIL. Fixes #1578, #1574, #1556, #1555, #1512, #1511, #1558.
2. **Docs-only routing fix**: Workflow should detect .md-only PRs and skip Rust matrix. Fixes #1558, #1512.
3. **Pre-open title guard**: Agent PR-creation should verify title format before opening. Fixes #1583 and future agent-generated PRs.

Each fix is a one-time substrate change that unblocks multiple PRs. The ROI on fixing the gate is **high**: blocks 8-10 PRs today, saves time on all future PRs of that class.

## Spec impact

This incident motivates a new section in `docs/agents/SPEC_UPDATE_CHECKLIST.md`:

> **§9. Throughput & funnel health**
> - Is this change a code feature, or a gate fix? If gate fix: measure the impact (how many stuck PRs will this unblock?).
> - If a gate fix: update docs/concepts/, docs/reference/, and MAINTAINER_AGENT_DOCTRINE as appropriate.
> - Before merging a gate fix, verify that it unblocks at least one real PR without regression.

Also: quarterly audit of open-PR distribution (how many are in draft, red, green, merge-ready?). If >30% are stuck in draft/red for reasons other than awaiting review, there's a gate/routing issue.

## Portable lesson

The merge funnel has three strata: discovery (identifying work), build (implementing work), and CI/merge (validating + landing work). When the funnel stalls, diagnose which stratum has the bottleneck. If the issue is not "discovery is slow" or "build is slow," it's a CI/routing/validation substrate issue. Substrate issues have **leverage**: fixing one broken gate unblocks 5-10 PRs and saves time on all future PRs of that class.

- **Pattern**: [docs/concepts/orchestrator-substrate-model.md](../concepts/orchestrator-substrate-model.md)
- **Class**: Pipeline throughput; economic leverage of substrate fixes
- **Generalization**: When many PRs are stuck red (not just one), suspect the gate, not the code. Broken gates have high leverage for fixes: one fix unblocks many PRs. Prioritize substrate health.

## Related PRs & Issues

### Stuck PRs (unblocked by substrate fixes once they land)
- [#1578](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1578) — test(lsp-folding)
- [#1574](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1574) — test(document-links)
- [#1556](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1556) — feat(lsp)
- [#1555](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1555) — feat(lsp)
- [#1512](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1512) — chore(branding)
- [#1511](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1511) — docs(roadmap)
- [#1558](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1558) — docs(NODEKIND)
- [#1583](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1583) — fix-forward for #1519

### Substrate issues (separate PRs to fix gates)
- Gate fix for ripr draft-skip-fails: See 2026-06-ripr-draft-skip-fails-gate.md
- Gate fix for docs-only Rust matrix: See 2026-06-docs-only-runs-rust-matrix.md
- Pre-open guard for titles: See 2026-06-validate-title-issue-ref-gap.md
