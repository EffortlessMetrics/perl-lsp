---
description: Refactor planner step 3 — post the refactoring plan as a PR comment
user-invocable: false
---

# Refactor Planner: Comment

Post your analysis and set the sign-off label.

## Steps

1. Post the refactoring plan as a PR comment (from analyze step output).

2. Set sign-off label:
   ```bash
   gh pr edit <number> --add-label "refactor-planner-reviewed"
   ```

3. If no refactoring opportunities found, say so explicitly:
   "No material simplification opportunities. Builder's implementation is
   already clean and idiomatic. Recommend skipping green-refactor for this PR."
