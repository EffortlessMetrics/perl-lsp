---
description: Deep reviewer step 4 — approve or send back with analysis
user-invocable: false
---

# Deep Reviewer Decide

Make the final call based on your analysis.

## Decision tree

### Default → Fix forward and approve
Push improvements directly: add edge case tests, fix logic issues, simplify code. Then approve with a summary of what you changed.
```bash
gh pr checkout <number>
# ... make improvements, commit ...
git push
gh pr review <number> --approve --body "Deep review: <what you improved>. Logic verified, low regression risk."
```

### Logic issues → Fix them on the branch
You're a sonnet agent on an isolated branch. If the logic is wrong but the approach is right, fix the logic yourself. Only send back if the approach is fundamentally wrong.

### Structural problems → Send back (rare)
Only when the approach is wrong, wrong crate, or the codebase moved too far:
```bash
gh pr review <number> --request-changes --body "<what's structurally wrong and why it can't be fixed locally>"
```

## Rules

- **Fix forward is the default.** If you can fix it, fix it.
- "I would have done it differently" → make it how you'd do it and push.
- Edge cases → add the test yourself, don't file a follow-up.
- Send back only for structural issues you can't resolve on the branch.
- **Recommend next steps.** Typical recommendations:
  - "Approved with improvements — ready for merge"
  - "Approved — recommend a follow-up builder for the related edge case I found in X"
  - "Fixed logic bug on branch — recommend a second deep-review to verify my fix"
