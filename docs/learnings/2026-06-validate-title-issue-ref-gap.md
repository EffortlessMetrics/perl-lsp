---
tags: [ci, pr-metadata, validation, agent-generated, title-check]
repos: [perl-lsp-swarm]
related: ["#1583", "#1519"]
portable: true
article_asset: false
search_terms: [validate-title, missing issue ref, agent-generated PR, PR title, no issue link, title check failure, pre-open guard, pr body, title format]
---

# validate-title fails on agent-generated PRs lacking issue reference in title

**Date**: 2026-06
**Hazard class**: ci / pr-metadata / validation
**Portable lesson**: [docs/concepts/enforcement-over-doctrine.md](../concepts/enforcement-over-doctrine.md)

## What happened

Agent-generated PRs with titles that lack an issue reference (e.g., `#1583` with title "fix(dap): correct frame ref bias" but no "#1519" or similar in the title) fail the `validate-title` check. The failure is non-blocking (does not prevent merge, only sets a red status), but it appears on 6+ PRs in recent cycles. Example: PR #1583 (fix-forward for #1519) has the issue reference in the PR body and labels, but not in the title string, and the check fails.

The check enforces a convention: PR titles must include a GitHub issue reference (e.g., "#1519") for traceability. The enforcement is sound, but the **gap is that the check does not know about PR body references or the distinction between code PRs (which inherit issue context) and agent-generated fix-forwards (which may be issue-addressing but lack the reference syntax in the title)**.

## Why

The `validate-title` check is a static string scan: it looks for the pattern `#<digits>` anywhere in the PR title. It does not look at the PR body, linked issues, or commit messages.

Agent-generated PRs (fix-forwards, follow-ups, refactor PRs) often lack an explicit issue reference in the title because:
1. They are generated programmatically and the generating agent did not include the issue ref in the title template
2. The issue context is known to the agent (it generated the PR to address #1519), but the title string does not encode it
3. The convention was designed for human-authored PRs, where the author naturally includes the issue ref in the title

This is a **validation scope gap**: the check is correct in principle but incomplete in practice for agent-driven workflows. A pre-open guard (in the PR-creation agent) could catch this before the PR is opened, preventing the red status altogether.

## Fix

**Partially observed.** The check is mechanical and working as designed. Two improvements are possible:

1. **Pre-open guard (shift-left):** Before an agent generates a PR, verify that the title includes an issue reference (or generate a title that does). This is a 1-line check in the PR-creation step and prevents the red check from appearing.

2. **Fallback in validate-title (robustness):** Enhance the check to accept a PR with an issue reference in the PR body OR commit message if the title lacks it. This is a secondary net.

Currently, neither is in place. The first (pre-open guard) is the more valuable shift-left control. Likely location: the agent's `pr-create` skill or the PR-creation template in `.claude/agents/`.

## Spec impact

This incident motivates updates to:

1. **docs/agents/** (PR-creation agent spec or template):
   > When generating a PR title, include an issue reference (`#<digits>`) if the PR addresses a tracked issue. The reference may be the original issue (#1519) or the fix-forward issue (#1520). If no issue reference exists, use a descriptive title and add a comment linking the related context.

2. **docs/agents/SPEC_UPDATE_CHECKLIST.md** (section 5, "Agent / workflow behavior"):
   > Agent-generated PRs should have a pre-open guard that verifies the title format before opening. For fix-forwards and follow-ups, the related-issue context should be encoded in the title or PR body (but title is preferred for backward compatibility with existing checks).

3. **docs/reference/MAINTAINER_AGENT_DOCTRINE.md**:
   > Title conventions are enforced by CI; agents must respect them. If a convention is violated routinely by a generated PR class, add a pre-open guard in the agent, not an exception in the check.

## Portable lesson

Conventions that are enforced by CI should have a pre-open guard in the code generation path. When a check fails predictably on a class of legitimate PRs (agent-generated fix-forwards), the cost of the false-positive (red check, signal-to-noise) exceeds the cost of moving the validation shift-left. Pre-open guards are cheaper than post-open exceptions.

- **Pattern**: [docs/concepts/enforcement-over-doctrine.md](../concepts/enforcement-over-doctrine.md)
- **Class**: CI validation; agent-driven PR generation
- **Generalization**: Enforce conventions at generation time (pre-open guard) rather than at dispatch time (post-open exception). If a check fails on every PR of a specific class, add the check to the generation agent, not an exception in the gate.

## Related PRs

- [#1583](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1583) — fix-forward for #1519; title lacks issue ref; validate-title FAILED
- [#1519](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1519) — original PR; referenced by fix-forward #1583
