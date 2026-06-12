---
tags: [multi-agent, branch-management, re-create, ripr, dap]
repos: [perl-lsp-swarm]
related: ["#1309", "#1337", "#1336", "#1216", "#1325"]
portable: false
article_asset: true
search_terms: [claude-admiring-volta-uotucs, multi-agent-tangle, ripr-suppress-dap-stack-frame-lifecycle, one-owner-per-branch, tangled-branch]
---

# Multi-agent branch tangle: #1309 re-created fresh as #1337

**Date**: 2026-06
**Hazard class**: process-hazard (multi-agent branch ownership)
**Portable lesson**: [docs/concepts/re-create-over-untangle.md](../concepts/re-create-over-untangle.md)

## What happened

Branch claude/admiring-volta-uotucs (PR #1309) accreted commits from multiple agents
over multiple rounds: a DAP stack-frame clear (fix for #964), an xtask ripr evidence
parser fix (for the 0.9.x format change), an extracted gate-parser fix, and a stray xtask
command. The ripr+ gate failed due to the schema mismatch (see 2026-06-ripr-output-schema-break.md),
causing suppression entries to be added that then collided with parallel PRs editing
policy/ripr-suppressions.toml. Cherry-picking the correct commits required non-trivial
conflict resolution.

## Why

Multiple agents worked on the same branch without a clean handoff. Each agent added what
it needed; no agent owned the cumulative diff. The accumulation of unrelated commits made
it impossible to read the diff as a single coherent change.

## Fix

Extract each concern as a standalone PR from its own spec. PR #1336: xtask ripr fix,
touching only xtask/src/tasks/ripr_evidence.rs. PR #1337: DAP fix, touching only DAP
files and one policy entry, re-created from the original spec (#964/#933). Close the
tangled PR (#1309) with pointers to the replacements.

## Spec impact

Motivated the one-owner-per-branch principle and docs/concepts/re-create-over-untangle.md.
Added to docs/reference/MAINTAINER_AGENT_DOCTRINE.md: check for active builder before
spawning a second agent on the same branch.

## Portable lesson

Re-creating a change from its spec is often cheaper and safer than untangling a branch
that accreted changes from multiple agents. The signal: it is no longer clear which
commits represent the intended change and which are workaround-on-workaround.

- **Pattern**: [docs/concepts/re-create-over-untangle.md](../concepts/re-create-over-untangle.md)
- **Class**: Process hazard -- multi-agent branch ownership
- **Generalization**: One owner per branch; re-create from spec when the tangle is deep.

## Related PRs

- [#1309](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1309) -- tangled PR (closed, superseded)
- [#1337](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1337) -- clean re-creation of DAP fix
- [#1336](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1336) -- extracted xtask fix
- [#1216](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1216) -- earlier tangled attempt
