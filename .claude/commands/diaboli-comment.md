---
description: Advocatus diaboli step 3 — post verdict as a structured issue comment
user-invocable: false
---

# Advocatus Diaboli: Post Comment

Post your verdict to the issue. Keep it sharp — the plan-reviewer reads
this to decide whether to invest sonnet-grade time.

## Steps

1. Format your findings from `/diaboli-challenge` into a comment.

2. Post the comment:

   ```bash
   gh issue comment <number> --body "$(cat <<'EOF'
   ## Advocatus Diaboli Review

   ### Case against building

   <numbered arguments against, with evidence>

   ### Strongest counter-argument

   <the best reason TO build this — be fair>

   ### Verdict: [BUILD | DEFER | CLOSE]

   <1-2 sentence justification>

   ---
   *Advocatus diaboli pass — challenges whether this should exist, not how to build it.*
   EOF
   )"
   ```

3. Add the label:

   ```bash
   gh issue edit <number> --add-label "diaboli-reviewed"
   ```

## Verdicts

- **BUILD**: "I tried to argue against this and couldn't. The user need is real, the scope is right, the timing is right. Proceed."
- **DEFER**: "Valid work, wrong time. Build X first, then revisit." Must name the prerequisite.
- **CLOSE**: "This should not be built. Here's why." Must have concrete evidence (no users asking for it, ecosystem already handles it, maintenance cost exceeds value, N degrees from user value).

## Notes

- A BUILD verdict is the most common and most valuable outcome — it means the issue survived adversarial review.
- Don't be contrarian for its own sake. If the issue is obviously valuable, say so quickly and move on.
- CLOSE is rare and should be reserved for issues that genuinely waste pipeline resources.
- If you recommend CLOSE, the plan-reviewer still makes the final call.
