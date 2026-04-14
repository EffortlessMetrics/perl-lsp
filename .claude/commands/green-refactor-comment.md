---
description: Green refactor step 4 — commit, push, comment on PR with what changed
user-invocable: false
---

# Green Refactor: Comment

Commit refactoring, push, set sign-off label, comment on PR.

## Steps

1. Push refactoring commits (should already be committed during simplify step):
   ```bash
   git push
   ```

2. Set sign-off label:
   ```bash
   gh pr edit <number> --add-label "green-refactor-reviewed"
   ```

3. Comment on PR:
   ```bash
   gh pr comment <number> --body "$(cat <<'EOF'
   ## Green Refactor

   ### Changes made
   <list of refactoring changes with commit refs>

   ### Tests: all passing
   `cargo test -p <crate>` — <N> tests pass

   ### What improved
   <brief summary: reduced complexity, better naming, idiomatic patterns, etc.>

   ---
   *Green refactor — simplify while green, the R in red-green-refactor.*
   EOF
   )"
   ```
