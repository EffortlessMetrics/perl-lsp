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

   _(The Verdict line is mandatory. Do not submit this comment without declaring a terminal state: READY FOR BUILDER or ALREADY FIXED.)_

   ---
   _Plan reviewed by plan-reviewer agent._
   COMMENT_EOF
   )"
   ```

2. If ready for builder, set final labels in a single call:
   ```bash
   gh issue edit <number> --add-label "plan-reviewed" --add-label "builder-ready" --remove-label "needs-plan-review"
   ```
   All three label changes in one call: `plan-reviewed` and `builder-ready` are added atomically,
   and `needs-plan-review` is removed so the orchestrator does not re-route this issue to another
   plan-reviewer on the next swarm pass. (`--remove-label` is a no-op if the label is absent.)

   After setting labels, write version-bound receipts for both:
   ```
   /label-receipt-write issue <number> plan-reviewed plan-reviewer
   /label-receipt-write issue <number> builder-ready plan-reviewer
   ```

3. If the spec is incomplete or wrong (root cause was wrong, file references stale, approach flawed):
   - **Do the investigation yourself.** Find the real root cause, correct the file references, design the fix. You have sonnet — use it.
   - Update the issue with the corrected spec: exact files, functions, lines, test cases, verify commands.
   - Then apply the same label transition. The output is always a builder-ready issue.

## Rules

- Always leave a comment, even if the plan is perfect — "Confirmed, no changes needed" is useful signal.
- Be specific about improvements, not vague ("needs work").
- Add edge case tests to the comment so the builder knows to include them.
- "Approved with suggestions" is the ideal outcome — approve and improve.
- **Recommend next steps.** Typical recommendations:
  - "Builder-ready — spec is solid, route to builder"
  - "Already fixed — close the issue, recommend regression tests via a test builder"
  - "Split into 2 issues — sub-pattern A is builder-ready, sub-pattern B needs a follow-up scout"
