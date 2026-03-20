---
description: Reviewer step 4 — approve, apply trivial fix, or send back to builder
---

# Reviewer Decide

Based on steps 1-3, make a decision.

## Decision tree

### No issues found → Approve
```bash
gh pr review <number> --approve --body "LGTM. Verified: <what you checked>."
gh pr ready <number>  # if still draft
```
Then: `SendMessage({to: "ops"})` that PR is merge-ready.

### Trivial issues only (typos, formatting, <5 lines) → Fix in place
1. Check out the branch:
   ```bash
   gh pr checkout <number>
   ```
2. Apply the fixes
3. Commit with: `fix(review): <what you fixed>`
4. Push
5. Approve and mark ready

### Non-trivial issues → Send back
1. Leave specific review comments on the PR:
   ```bash
   gh pr review <number> --request-changes --body "<specific blocker description>"
   ```
2. Each comment must be actionable — the builder should know exactly what to change
3. `SendMessage({to: "builder"})` with the blocker list

## Rules

- Never request changes for style preferences. Only for bugs, banned patterns, or missing tests.
- "I would have done it differently" is not a blocker.
- Approve unless there's a concrete defect.
