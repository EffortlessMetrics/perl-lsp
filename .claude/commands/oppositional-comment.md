---
description: Oppositional planner step 3 — post challenges as a structured issue comment
user-invocable: false
---

# Oppositional Planner: Post Comment

Post your challenges to the issue as a structured comment. The plan-reviewer
will read this alongside the scout spec and research verification.

## Steps

1. Format your findings from `/oppositional-challenge` into a comment.

2. Post the comment:

   ```bash
   gh issue comment <number> --body "$(cat <<'EOF'
   ## Oppositional Review

   ### Objections

   <numbered objections with evidence>

   ### Alternatives not considered

   <concrete alternatives with tradeoffs>

   ### Risk flags

   <interaction / performance / maintenance risks>

   ### Verdict

   **APPROACH IS: [SOUND | QUESTIONABLE | NEEDS RETHINK]**

   **Key question for plan-reviewer:** <the one question that matters most>

   ---
   *Oppositional planner pass — challenges the approach, not the problem.*
   EOF
   )"
   ```

3. Add the label:

   ```bash
   gh issue edit <number> --add-label "oppositional-reviewed"
   ```

## Notes

- If you have zero substantive objections, say so explicitly:
  "No material objections. The proposed approach is straightforward and
  the alternatives are clearly worse. Recommend proceeding to plan-review."
- Don't fabricate objections for the sake of having objections.
- Your comment should help the plan-reviewer, not slow them down.
