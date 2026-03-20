---
description: Plan reviewer step 4 — improve the spec and mark builder-ready
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

   **Verdict:** READY FOR BUILDER / NEEDS MORE SCOUT WORK

   ---
   _Plan reviewed by plan-reviewer agent._
   COMMENT_EOF
   )"
   ```

2. If ready for builder, add the label:
   ```bash
   gh issue edit <number> --add-label "builder-ready"
   ```

3. If NOT ready (root cause was wrong, file references stale, approach flawed):
   - Explain specifically what needs more investigation
   - Don't add the builder-ready label
   - The orchestrator will route it back to a scout

## Rules

- Always leave a comment, even if the plan is perfect — "Confirmed, no changes needed" is useful signal.
- Be specific about improvements, not vague ("needs work").
- Add edge case tests to the comment so the builder knows to include them.
- "Approved with suggestions" is the ideal outcome — approve and improve.
