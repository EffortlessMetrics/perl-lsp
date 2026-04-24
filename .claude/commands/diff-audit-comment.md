---
description: Diff auditor step 2 — post findings and set label
user-invocable: false
---

# Diff Audit: Comment

Post your findings and set the appropriate label.

## Steps

1. Post comment:
   ```bash
   gh pr comment <number> --body "$(cat <<'EOF'
   ## Diff Audit

   **Files changed:** <count>
   **Lines:** +<added> -<removed>
   **Commits:** <count> (<list>)

   ### Spec alignment: [COMPLETE | PARTIAL | DRIFT]
   <acceptance criteria coverage>

   ### Cleanliness: [CLEAN | ARTIFACTS FOUND]
   <leftover TODOs, debug code, out-of-scope files>

   ### Commit coherence: [CLEAN | MESSY]
   <commit history quality>

   ### Verdict: [CLEAN | ARTIFACTS | REGRESSION | SCOPE DRIFT]
   <one sentence>

   ---
   *Diff auditor — final coherence check before merge.*
   EOF
   )"
   ```

2. If CLEAN:
   ```bash
   gh pr edit <number> --add-label "diff-audited"
   ```

3. If not CLEAN:
   ```bash
   gh pr edit <number> --add-label "needs-diff-fix"
   ```
