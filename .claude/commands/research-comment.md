---
description: Research verifier step 5 — post verification findings as a structured issue comment and add label
user-invocable: false
---

# Research: Comment

Post the verification findings to the GitHub issue as a structured comment,
then apply the `research-verified` label to signal the issue is ready for
plan-review.

## Steps

1. **Compile the findings** from steps 2-4 into one summary:
   - Count verified/false/unverified per category
   - Highlight any FALSE claims that will affect the plan-reviewer's work

2. **Post the comment:**

   ```bash
   gh issue comment <number> --body "$(cat <<'VERIFY_EOF'
   ## Research Verification

   **Summary:** X of Y claims verified (Z false, W unverified)

   ### Perl Claims

   | Claim | Status | Finding | Source |
   |-------|--------|---------|--------|
   | <claim> | VERIFIED / FALSE / UNVERIFIED | <finding> | [link](<url>) |

   ### LSP/DAP Spec Claims

   | Claim | Status | Finding | Source |
   |-------|--------|---------|--------|
   | <claim> | VERIFIED / FALSE / UNVERIFIED | <finding> | [link](<url>) |

   ### Crate API Claims

   | Claim | Status | Finding | Source |
   |-------|--------|---------|--------|
   | <claim> | VERIFIED / FALSE / UNVERIFIED | <finding> | [link](<url>) |

   ### Action Items for Plan-Reviewer

   <List any FALSE claims that need correction in the spec. Be specific:
   "P1 is FALSE — correct 'since 5.32' to 'since 5.10' in the spec body"
   If all claims verified: "All claims verified. No corrections needed.">

   ---
   _Verified by research-verifier agent. Ready for plan-review._
   VERIFY_EOF
   )"
   ```

3. **Ensure the `research-verified` label exists, then apply it:**

   ```bash
   # Create the label if it doesn\'t exist (idempotent)
   gh label create "research-verified" \
     --color "0075ca" \
     --description "Facts verified by research-verifier agent" \
     2>/dev/null || true

   # Apply the label to the issue
   gh issue edit <number> --add-label "research-verified"
   ```

4. **Remove `needs-research-verification` label if present:**

   ```bash
   gh issue edit <number> --remove-label "needs-research-verification" 2>/dev/null || true
   ```

## Rules

- Always post the comment BEFORE adding the label (label is the signal that work is done).
- If ALL claims were skipped (no verifiable external facts), post a brief comment saying so,
  then still add the label.
- If ANY claim is FALSE, make the action items section prominent — it's the most important
  output for the plan-reviewer.
- Do NOT suggest fix approaches in the comment. Just report what is true or false.
- Confirm the comment was posted by printing the issue URL.

## Output

```
Comment posted on issue #NNN: <URL>
Label added: research-verified
FALSE claims requiring plan-reviewer attention: <N>
```
