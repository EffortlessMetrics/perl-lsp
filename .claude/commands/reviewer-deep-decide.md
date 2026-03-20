---
description: Deep reviewer step 4 — approve or send back with analysis
user-invocable: false
---

# Deep Reviewer Decide

Make the final call based on your analysis.

## Decision tree

### Logic correct + tests good + low regression risk → Approve
```bash
gh pr review <number> --approve --body "Deep review: logic correct, tests verify behavior, low regression risk."
```
Then: `SendMessage({to: "ops"})` that PR is merge-ready.

If you found non-blocking edge cases, file them as follow-up:
```bash
gh issue create --title "follow-up: edge case for #<PR-issue>" --body "<edge case details>"
```

### Logic issues or high regression risk → Send back
```bash
gh pr review <number> --request-changes --body "<specific analysis of what's wrong>"
```
Each comment must explain:
- What's wrong with the logic
- What the correct behavior should be
- Suggested approach to fix

Then: `SendMessage({to: "builder"})` with the analysis.

### Weak tests but logic correct → Approve with follow-up
Approve the PR but file a follow-up issue for better tests:
```bash
gh pr review <number> --approve --body "Logic correct. Tests are minimal — filed follow-up for better coverage."
gh issue create --title "test: improve coverage for #<PR-issue>" --body "<what tests to add>"
```

## Rules

- Block only for correctness issues, not style
- "I would have done it differently" is not a blocker
- Edge cases that don't cause crashes → follow-up issue, not a block
- When in doubt, approve and file follow-up
