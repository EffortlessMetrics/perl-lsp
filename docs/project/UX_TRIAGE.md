# UX Triage Guide

This document defines what counts as a UX bug, how to label and prioritize UX issues, the triage cadence, and how to close UX issues correctly.

## What counts as a UX bug

A UX bug is anything a first-time user can see, click, or get stuck on. This includes:

- Error messages that appear in the VSCode output panel or notifications
- Missing or confusing completion, hover, or diagnostics results
- Extension startup failures (silent failures, unhelpful messages)
- Broken or absent commands in the command palette
- Configuration options that don't work or are undocumented
- Missing feedback while background operations run (indexing, formatting)
- Any behavior that would cause a first-time user to file an issue, open Stack Overflow, or uninstall

It does not include:

- Internal refactors with no user-visible behavior change
- Parser correctness bugs with no LSP surface (file those as `parser` bugs)
- Performance issues that are imperceptible to users (file those as `performance`)

## Labels and when to use them

| Label | When to apply |
|-------|--------------|
| `ux` | Any user-visible behavior issue. Apply this first — always. |
| `ux:p0-blocker` | Would cause a first-time user to give up or uninstall. Blocks a release milestone. Examples: extension fails to start, no completions at all, crashes on open. |
| `ux:p1-friction` | Annoying but users can work around it. Examples: completions delayed, error messages not actionable, formatting requires restart. |
| `ux:p2-polish` | Minor UX improvement. Does not block any milestone. Examples: icon missing, tooltip copy wrong, command palette label inconsistency. |
| `ux:regression` | Something that worked before and no longer works. Always check for a regression test gap. Apply in addition to a priority label. |
| `ux:fix-forward` | The fix is merged. Applied to closed issues to signal that a regression test was added and is actively guarding the fix. |

All UX issues should carry the `ux` label plus exactly one priority label (`ux:p0-blocker`, `ux:p1-friction`, or `ux:p2-polish`). When in doubt, assign the higher severity — downgrade is cheaper than an unnoticed blocker.

## Filing a UX issue

Use the **UX Bug Report** issue template at `.github/ISSUE_TEMPLATE/ux-bug.yml`. It automatically applies the `ux` label and includes:

- Expected vs. actual behavior
- Reproduction steps
- Environment details (OS, VSCode version, extension version, Perl version)
- Optional: logs from "Perl LSP: Show Logs" command
- Severity self-assessment (the reporter's view — triager sets the final label)

## Triage cadence

**Weekly sweep** (Mondays, ~15 minutes):
1. `gh issue list --label "ux" --state open --json number,title,labels | jq` — check for new issues missing a priority label
2. Assign `ux:p0-blocker`, `ux:p1-friction`, or `ux:p2-polish` to any unlabeled `ux` issues
3. Look for `ux:p0-blocker` issues without an assignee — escalate immediately

**Pre-release sweep** (before any release candidate):
1. All `ux:p0-blocker` issues must be closed or explicitly deferred before release
2. All `ux:regression` issues need a regression test or documented justification

**Ad-hoc**:
- Any issue with "ux" in the title filed without the template — manually apply `ux` label and priority
- Any issue that arrives from a real user (not swarm-discovered) — bump to `ux:p0-blocker` pending triage

## Closing a UX issue

A UX issue may be closed when **one** of these conditions is met:

1. **Test added**: A test was added to the UX test harness that would fail if the regression returned. Apply `ux:fix-forward` label. Reference the test in the closing comment.
2. **Won't fix**: The behavior is intentional or the fix cost exceeds the value. Document the decision in the issue with the keyword `WONTFIX:` so it's searchable.
3. **Duplicate**: Reference the canonical issue.
4. **Already fixed**: The behavior changed in a prior PR. Reference the PR and add `already-fixed` label.
5. **Not a UX bug**: The behavior is not user-visible. Remove the `ux` label and reclassify.

Do not close a `ux:p0-blocker` without a fix or explicit deferral decision. Deferral requires a milestone target in the issue body.

## Escalation path for UX blockers on master

If a `ux:p0-blocker` is filed against master (not a pre-release branch):

1. **Within 1 hour**: Assign to a builder or self-assign
2. **Within 4 hours**: Draft PR must exist, even if not reviewed
3. **Within 24 hours**: Merged or formally deferred with a milestone date

For regressions (`ux:regression` + `ux:p0-blocker`), additionally:
1. Identify the commit that introduced the regression with `git bisect` or `git log --oneline`
2. File a note in `.ci/blockers.yaml` if it affects the CPAN corpus gate

Escalate to the repo owner (@EffortlessMetrics) if no builder is available within 4 hours.

## UX test harness

The UX test harness catches regressions in user-visible behavior automatically. It tests:

- Extension startup behavior (health check passes, server connects)
- Error message text (actionable, not panic traces)
- Completion triggers (keywords, method calls, package names)
- Diagnostic message formatting
- Graceful degradation when tools (perltidy, perlcritic) are missing

### Running the harness

```bash
# Run UX-specific tests
cargo test --workspace -- ux

# Check for UX regressions in the LSP surface
cargo test -p perl-lsp -- ux

# Check extension UX tests (requires Node.js)
cd vscode-extension && pnpm test
```

When adding a UX fix, add a test that would fail without the fix. Name the test with a `ux_` prefix so it's discoverable.

### Backfilling tests for closed UX issues

When closing a `ux:fix-forward` issue, verify the test exists:

```bash
# Find existing ux_ prefixed tests
grep -r "fn ux_" crates/
```

If no test exists for a fix, add one before applying `ux:fix-forward`.

## Links

- Issue template: `.github/ISSUE_TEMPLATE/ux-bug.yml`
- PR template UX checkbox: `.github/PULL_REQUEST_TEMPLATE.md`
- Labels: `ux`, `ux:p0-blocker`, `ux:p1-friction`, `ux:p2-polish`, `ux:regression`, `ux:fix-forward`
- UX-related issues: [filter on GitHub](https://github.com/EffortlessMetrics/perl-lsp/issues?q=label%3Aux)
