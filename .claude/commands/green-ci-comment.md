---
description: Green CI agent step 2 — post verdict as PR comment and set label
user-invocable: false
---

# Green CI: Comment

Post the CI verdict and set the sign-off label if green.

## Steps

1. Post comment:
   ```bash
   gh pr comment <number> --body "$(cat <<'EOF'
   ## CI Verification

   **HEAD SHA:** `<sha>`
   **Verdict:** [GREEN | RED | STALE | BLOCKED]

   | Check | Status | SHA |
   |-------|--------|-----|
   | <name> | <pass/fail> | <sha[0:8]> |
   | ... | ... | ... |

   <if RED: list failures>
   <if STALE: note which checks need re-run>
   <if BLOCKED: list blockers>

   ---
   *Green CI — SHA-verified CI freshness check.*
   EOF
   )"
   ```

2. If GREEN, set sign-off label:
   ```bash
   gh pr edit <number> --add-label "ci-green"
   ```

3. If RED or STALE, do NOT set label. Flag for pr-responder:
   ```bash
   gh pr edit <number> --add-label "needs-ci-fix"
   ```
