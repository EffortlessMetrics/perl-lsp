---
description: Plan reviewer step 4 — improve the spec and mark builder-ready
user-invocable: false
---

# Plan Review: Improve

Write your findings as an issue comment and label the issue as builder-ready.

## Steps

1. Write a review comment on the issue:
   ```bash
   gh issue comment <number> --body "$(cat <<'COMMENT_EOF'
   ## Plan Review

   **File references:** ✅ Current / ⚠️ Updated: <corrections>

   **Root cause:** ✅ Confirmed / ⚠️ Refined: <corrections>

   **Approach assessment:** <your analysis>
   - Risk: LOW/MEDIUM/HIGH
   - Simpler alternative: <if found>

   **Edge cases to cover:**
   - <edge case 1>
   - <edge case 2>

   **Test spec refinements:**
   - <any improvements to the test>

   **Verdict:** READY FOR BUILDER / ALREADY FIXED (with evidence)

   ---
   _Plan reviewed by plan-reviewer agent._
   COMMENT_EOF
   )"
   ```

2. If ready for builder, add both labels in a single call:
   ```bash
   gh issue edit <number> --add-label "plan-reviewed" --add-label "builder-ready"
   ```
   Both labels in one call is atomic — either both are set or neither is, preventing
   the partial state where `plan-reviewed` exists without `builder-ready`.
   `plan-reviewed` records that the spec passed review; `builder-ready` gates builder pickup.

3. If the spec is incomplete or wrong (root cause was wrong, file references stale, approach flawed):
   - **Do the investigation yourself.** Find the real root cause, correct the file references, design the fix. You have sonnet — use it.
   - Update the issue with the corrected spec: exact files, functions, lines, test cases, verify commands.
   - Then add both `plan-reviewed` and `builder-ready` labels in a single call. The output is always a builder-ready issue.

## Rules

- Always leave a comment, even if the plan is perfect — "Confirmed, no changes needed" is useful signal.
- Be specific about improvements, not vague ("needs work").
- Add edge case tests to the comment so the builder knows to include them.
- "Approved with suggestions" is the ideal outcome — approve and improve.
- **Recommend next steps.** Typical recommendations:
  - "Builder-ready — spec is solid, route to builder"
  - "Already fixed — close the issue, recommend regression tests via a test builder"
  - "Split into 2 issues — sub-pattern A is builder-ready, sub-pattern B needs a follow-up scout"
