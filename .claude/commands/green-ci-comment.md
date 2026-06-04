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
   > **MCP alternative (web/no-gh sessions):** `mcp__github__add_issue_comment(owner, repo, issue_number:<number>, body:"...")`.

2. If GREEN, set sign-off label:
   ```bash
   gh pr edit <number> --add-label "ci-green"
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__pull_request_read(method:"get", pullNumber:<number>)` to read current labels, then `mcp__github__issue_write(method:"update", issue_number:<number>, labels:[...current + "ci-green"])`. Note: `issue_write` labels field replaces the full list — always read current labels first before writing.

3. If RED or STALE, do NOT set label. Flag for pr-responder:
   ```bash
   gh pr edit <number> --add-label "needs-ci-fix"
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__pull_request_read(method:"get", pullNumber:<number>)` to read current labels, then `mcp__github__issue_write(method:"update", issue_number:<number>, labels:[...current + "needs-ci-fix"])`. Note: `issue_write` labels field replaces the full list — always read current labels first before writing.
