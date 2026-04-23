---
description: Research verifier step 5 — post verification findings as a structured issue comment and add label
user-invocable: false
---

# Research: Comment

Post the verification findings to the GitHub issue as a structured comment,
then apply the `research-reviewed` label to signal the issue is ready for
plan-review.

## Steps

1. **Compile the findings** from steps 2-4 into one summary:
   - Count verified/false/unverified per category
   - Highlight any FALSE claims that will affect the plan-reviewer's work
   - Note which categories had zero claims (omit those sections entirely)

2. **Post the comment:**

   Only include sections that have at least one claim. Omit empty sections
   entirely — do not render a table with placeholder rows for categories
   that had no claims.

   ```bash
   gh issue comment <number> --body "$(cat <<'VERIFY_EOF'
   ## Research Verification

   **Summary:** X of Y claims verified (Z false, W unverified)

   ### Perl Claims
   _(include only if there were Perl claims to check)_

   | Claim | Status | Finding | Source |
   |-------|--------|---------|--------|
   | <claim> | VERIFIED / FALSE / UNVERIFIED | <finding> | [link](<url>) |

   ### LSP/DAP Spec Claims
   _(include only if there were LSP/DAP claims to check)_

   | Claim | Status | Finding | Source |
   |-------|--------|---------|--------|
   | <claim> | VERIFIED / FALSE / UNVERIFIED | <finding> | [link](<url>) |

   ### Crate API Claims
   _(include only if there were crate API claims to check)_

   | Claim | Status | Finding | Source |
   |-------|--------|---------|--------|
   | <claim> | VERIFIED / FALSE / UNVERIFIED | <finding> | [link](<url>) |

   ### Action Items for Plan-Reviewer

   <List any FALSE claims that need correction in the spec. Be specific:
   "P1 is FALSE — correct 'since 5.32' to 'since 5.10' in the spec body"
   If all claims verified: "All claims verified. No corrections needed."
   IMPORTANT: State only what is true or false. Do NOT suggest how to fix
   the spec or redesign the approach — that is the plan-reviewer's role.>

   ---
   _Verified by research-verifier agent. Ready for plan-review._
   VERIFY_EOF
   )"
   ```

3. **Ensure the `research-reviewed` label exists, then apply it:**

   ```bash
   # Create the label if it doesn\'t exist (idempotent)
   gh label create "research-reviewed" \
     --color "0075ca" \
     --description "Facts verified by research-verifier agent" \
     2>/dev/null || true

   # Apply the label to the issue
   gh issue edit <number> --add-label "research-reviewed"
   ```

4. **Remove `needs-research-verification` label if present:**

   ```bash
   gh issue edit <number> --remove-label "needs-research-verification" 2>/dev/null || true
   ```

## Rules

- Always post the comment BEFORE adding the label (label is the signal that work is done).
- **Omit sections with no claims.** A parser-only issue with no LSP or API claims should
  have only the Perl Claims table. Empty tables add noise and waste the plan-reviewer's time.
- If ALL claims were skipped (no verifiable external facts in any category), post a brief
  comment saying so, then still add the label to unblock the pipeline.
- If ANY claim is FALSE, make the action items section prominent — it's the most important
  output for the plan-reviewer.
- **Scope boundary:** The action items section names false claims and what the correct fact
  is. It does NOT propose design changes, suggest alternative implementations, or recommend
  how the spec should be restructured. That is the plan-reviewer's job.
- Do NOT suggest fix approaches in the comment. Just report what is true or false.
- Confirm the comment was posted by printing the issue URL.

## Output

```
Comment posted on issue #NNN: <URL>
Label added: research-reviewed
FALSE claims requiring plan-reviewer attention: <N>
```
