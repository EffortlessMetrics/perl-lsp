---
tags: [ci, routing, workflow-dispatch, build-matrix, documentation]
repos: [perl-lsp-swarm]
related: ["#1558", "#1512"]
portable: true
article_asset: false
search_terms: [docs-only, build matrix, Rust matrix, workflow dispatch, CI routing, documentation PR, .md only, irrelevant failure, no code path filter]
---

# Docs-only PRs run full Rust build matrix; pure .md changes fail irrelevantly

**Date**: 2026-06
**Hazard class**: ci / routing
**Portable lesson**: [docs/concepts/orchestrator-substrate-model.md](../concepts/orchestrator-substrate-model.md)

## What happened

PRs that modify only documentation files (pure `.md` changes) trigger the full Rust build and test matrix, including compilation, clippy, and test execution. The Rust matrix then fails on these PRs even though no Rust code changed. Observable on #1558 (docs/design/NODEKIND_CLASSIFICATION_DIVERGENCE.md) and #1512 (chore: branding, documentation label). Both PRs have red check status due to irrelevant Rust build failures; the fixes required are not code-side.

The CI dispatch routing has no path-based filter to detect documentation-only PRs and skip the Rust matrix. When a PR modifies only `.md` files, the workflow still dispatches the full build. This is a substrate waste (unnecessary CI cycles) and a signal-to-noise problem (red checks on code-correct PRs).

## Why

The GitHub Actions workflow dispatch is routed by branch name, PR state, and label presence — but NOT by file-path delta. The routing logic has no place to check "does this PR touch `crates/` or any `.rs` file?" before dispatching the Rust build step.

This is a **CI routing gap**: the dispatcher does not observe the change scope. It unconditionally dispatches the full matrix when a PR exists, regardless of the files touched.

The architectural assumption — "CI dispatch is PR-state driven, not change-driven" — is appropriate for most gates (which need to run on every code PR), but creates a waste when applied to docs-only PRs without a filtering layer.

## Fix

**Not yet applied.** The fix is a workflow routing logic change. The dispatcher must:

1. Detect documentation-only PRs by checking the file delta (using `git diff origin/main...HEAD -- crates/ xtask/ .github/workflows/` or similar scope guard)
2. If only `.md` files are touched and no Rust files, emit `SKIP_RUST_MATRIX=true` or skip the dispatch entirely
3. If Rust files are touched (or the check is ambiguous), dispatch the full matrix as before

Likely location: `.github/workflows/*.yml` dispatch trigger, before the matrix job step. A pre-flight job could compute the file scope and conditionally skip downstream jobs via `if: always()` guards.

## Spec impact

This incident motivates updates to:

1. **docs/agents/SPEC_UPDATE_CHECKLIST.md** (section 5, "Agent / workflow behavior"):
   > When adding a workflow dispatch step, include a path-based routing filter. Documentation-only PRs should skip expensive compute jobs (Rust matrix, coverage gates). Test your skip logic on a docs-only branch before landing.

2. **docs/reference/CI_ARCHITECTURE.md** (if it exists; or create a section):
   > Dispatch routers must be change-aware. A skipped job is not a failed job. Use GitHub Actions conditional `if:` guards and file-delta detection to route expensive jobs only when relevant.

## Portable lesson

Workflow dispatch routers that lack change-awareness become funnel amplifiers: they dispatch expensive jobs on irrelevant PRs, inflating CI time and creating red herring failures. Path-based routing (git diff guards) is a cheap shift-left control that prevents signal-to-noise pollution without slowing relevant PRs.

- **Pattern**: [docs/concepts/orchestrator-substrate-model.md](../concepts/orchestrator-substrate-model.md)
- **Class**: CI routing; workflow dispatch efficiency
- **Generalization**: Dispatch routers should be change-aware; gate scope and file delta are orthogonal concerns but must be co-observed.

## Related PRs

- [#1558](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1558) — docs(design): NODEKIND_CLASSIFICATION_DIVERGENCE.md; pure .md; Rust matrix failed irrelevantly
- [#1512](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1512) — chore(branding): logo and docs; documentation label; Rust matrix failed irrelevantly
