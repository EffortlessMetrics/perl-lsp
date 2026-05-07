# Droid Review Rules

## Review Standards

**No naked LGTM.** Every review comment must explain why the finding matters.

**Repair packets, not editorials.** Findings must include:
- Why here (the failure mode)
- Fix direction (what to change)
- Validation method (how to verify the fix works)

**Evidence provenance.** Clean reviews must categorize evidence:
- **Observed:** What Droid found by inspection
- **Reported:** Test output or CI signals
- **Not verified:** Things Droid cannot check (e.g., "no integration test exists for this path")

**No extra @mentions.** Droid should not mention reviewers, maintainers, or other people in its generated review body. Ping only in separate comments if the finding requires escalation.

**Same-repo guard.** Droid auto-review only runs on PRs where the head branch is in the same repository. This prevents secret execution on fork PRs.

**Trusted actor guard.** Manual `@droid` commands are gated by GitHub author association (OWNER, MEMBER, COLLABORATOR). Public comments cannot trigger secrets-backed jobs.

## Inspection Record

Clean reviews (no actionable findings) must include:

```text
No actionable findings emitted.

Inspected surfaces:
- [list of files, patterns, or subsystems checked]

Checks performed:
- [list of analysis steps: linting, semantic analysis, etc.]

Why no comments:
- [brief explanation: within guidelines, consistent with patterns, etc.]

Residual risk:
- [any areas not covered by Droid's analysis]

Validation signal:
  Observed:
    - [test signals, CI results]
  Reported:
    - [output from tools, linters, security scanners]
  Not verified:
    - [things Droid cannot check]
```

## Severity Tiers

- **P0 (Critical):** Security, data corruption, build breakage, merge conflicts
- **P1 (High):** Logic errors, performance regressions, API violations
- **P2 (Medium):** Style, naming, documentation gaps
- **P3 (Low):** Nice-to-haves, future improvements (rarely emitted)

## Model & Depth

Droid reviews perl-lsp with:
- **Model:** MiniMax-M2.7 (custom BYOK)
- **Depth:** shallow (fast, focused findings)
- **Security depth:** medium (balance coverage and noise)

Shallow review prioritizes correctness and safety. Deep review is triggered only on explicit request (not baseline).
