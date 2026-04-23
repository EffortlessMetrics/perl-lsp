---
description: Write scout findings as a builder-ready GitHub issue
argument-hint: "<one-line title of the finding>"
user-invocable: false
---

# Scout Report

File a GitHub issue that a builder can implement WITHOUT re-researching.
This is the scout's primary deliverable. Invoke ONLY after completing all
7 steps of the scout checklist.

## Pre-flight Check

Before filing, verify you have ALL of these. If any are missing, go back
and complete the scout checklist step that produces it:

- [ ] **File:line locations** — exact paths to every relevant code location
- [ ] **Reproduction** — minimal example that triggers the bug/gap
- [ ] **Root cause** — one sentence explaining WHY it fails
- [ ] **Fix options** — 2-3 approaches with tradeoffs
- [ ] **Recommendation** — which option and why
- [ ] **Test spec** — exact test code or command that proves the fix works
- [ ] **Dedup confirmed** — no existing issue or PR covers this

## Template

Use the **Full Scout Report** variant from `/scout-issue`. Do NOT hand-roll the issue body.

```
Invoke /scout-issue for the canonical issue template.
Fill all sections: Problem, Root Cause, Options, Recommendation, Builder Spec, Acceptance Criteria, Scope.
```

## Rules

- ONE issue per distinct finding. Do not bundle.
- Fill in ALL sections. No placeholders. No "TBD" or "needs investigation."
- **Root Cause** must name a specific function and file:line.
- **Builder Spec** must be copy-paste implementable.
- **Test to add** must be actual code, not a description of what to test.
- If you can't fill in the Builder Spec completely, **fill in what you can and note your uncertainty.** A plan-reviewer will verify and improve. A roughly-right spec that a plan-reviewer can correct is more valuable than no spec at all.
- Label `swarm-discovered` for bugs/improvements, `swarm-architectural`
  for design decisions that need human input.
- After creating the issue, **add the pipeline label:**
  ```bash
  gh issue edit <number> --add-label "needs-plan-review"
  ```
  This is the entry point for the verification pipeline. Without it, the
  issue is invisible to accuracy-scouts, research-verifiers, and plan-reviewers.
- Print the URL.
- **Recommend next steps.** Typical recommendations:
  - "Ready for plan-review — spec is complete but I'm uncertain about the root cause in X"
  - "Recommend a follow-up scout on the Y subsystem — I found related issues there"
  - "High confidence on root cause — plan-review should focus on edge cases and test coverage"
