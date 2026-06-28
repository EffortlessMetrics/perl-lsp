---
tags: [tag1, tag2]
repos: [repo-name]
related: ["#NNN", "#MMM"]
portable: true
article_asset: true
search_terms: [symbol-or-function-name, error-string, "PR title fragment"]
---

# [Short incident title, present tense]

**Date**: YYYY-MM
**Hazard class**: [ID/ref collision | bounds/overflow | protocol-safety | scanner-blindness | test-encodes-bug | coverage-integrity | other]
**Portable lesson**: [docs/concepts/filename.md](../concepts/filename.md)

## What happened

[2-5 sentences. Describe the observable failure: what broke, what the symptom was, which
check or reviewer caught it. Be specific about the component, the behavior, and the impact.]

## Why

[2-4 sentences. Root cause: what assumption was wrong, what check was missing, what design
decision made the failure possible. Distinguish from the fix -- this is why the failure
was possible, not how it was corrected.]

## Fix

[2-4 sentences. What was changed and in which files. Include the PR number and key
function/constant names so a future grep lands here.]

## Spec impact

[What spec, acceptance criterion, or checklist item this incident motivated or updated.
Reference the specific file and section. If no spec was updated, note "None -- follow-up
tracked in #NNN" or "None -- one-off, no recurring class identified."]

## Portable lesson

Link to the relevant [docs/concepts/](../concepts/) pattern. One sentence on how this
incident is a concrete instance of that pattern.

- **Pattern**: [docs/concepts/filename.md](../concepts/filename.md)
- **Class**: [from hazard-class-invariants.md, or N/A]
- **Generalization**: [one sentence on what the abstract lesson is]

## Related PRs

- [#NNN](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/NNN) -- [what it did]
- [#MMM](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/MMM) -- [what it reported]
