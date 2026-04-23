---
description: Accuracy-scout step 5 — draft accuracy comment, update issue, add label
user-invocable: false
---

# Accuracy: Comment

Post a structured accuracy comment to the issue, update or correct the issue
body if needed, and attach the `accuracy-reviewed` label.

## Steps

1. **Classify the outcome** based on steps 2-4:

   - **CLEAN** — all file paths, functions, and claims verified. No corrections needed.
   - **CORRECTED** — some paths/functions were stale; corrections noted in comment.
   - **LIKELY FIXED** — recent merged PR covers this issue; recommend closure.
   - **DUPLICATE** — open issue already tracks this; link it.
   - **UNVERIFIABLE** — claimed file/function never existed in any recent history; issue may be hallucinated.

2. **Post the accuracy comment:**

   ```bash
   gh issue comment <number> --body "$(cat <<'ACCURACY_EOF'
   ## Accuracy Review

   **Outcome:** CLEAN / CORRECTED / LIKELY FIXED / DUPLICATE / UNVERIFIABLE

   ### File Path Checks

   | Claimed | Status | Correction |
   |---------|--------|-----------|
   | `crates/perl-parser/src/foo.rs` | VERIFIED | — |
   | `crates/perl-parser/src/bar.rs` | STALE PATH | `crates/perl-parser/src/core/bar.rs` |

   ### Function/Symbol Checks

   | Claimed | Status | Correction |
   |---------|--------|-----------|
   | `fn parse_hash_or_block` | VERIFIED at expressions.rs:417 | — |
   | `fn parse_method_call` | STALE FUNCTION | renamed to `parse_method_invocation` at expressions.rs:382 |

   ### Status Checks

   | Check | Result |
   |-------|--------|
   | Already fixed? | PR #2528 merged 2026-03-15 — covers this area |
   | Duplicate? | No open duplicate found |
   | Corpus examples exist? | VERIFIED |

   ### Action Items for Plan-Reviewer

   <List only what a plan-reviewer must know before writing a spec:
   "F2 is STALE PATH — update references to crates/perl-parser/src/core/bar.rs"
   "S2 RENAMED — issue body references parse_method_call; correct name is parse_method_invocation"
   If CLEAN: "All facts verified. No corrections needed."
   If LIKELY FIXED: "Recommend closing — PR #2528 already addresses this. Verify before closing."
   DO NOT suggest implementation approaches — that is the plan-reviewer's role.>

   ---
   _Reviewed by accuracy-scout. Mechanical facts verified against current master._
   ACCURACY_EOF
   )"
   ```

3. **Ensure the `accuracy-reviewed` label exists, then apply it:**

   ```bash
   gh label create "accuracy-reviewed" \
     --color "e4e669" \
     --description "Mechanical facts verified against master by accuracy-scout" \
     2>/dev/null || true

   gh issue edit <number> --add-label "accuracy-reviewed"
   ```

4. **Remove `needs-accuracy-scout` label if present:**

   ```bash
   gh issue edit <number> --remove-label "needs-accuracy-scout" 2>/dev/null || true
   ```

5. **If LIKELY FIXED or DUPLICATE, add a close recommendation** by adding
   label `needs-triage` so a human or ops agent can review and close:

   ```bash
   # Only if LIKELY FIXED or DUPLICATE:
   gh issue edit <number> --add-label "needs-triage"
   ```

## Rules

- Always post the comment BEFORE adding the label (label is the completion signal).
- **Scope boundary:** Report what is correct or incorrect. Do NOT redesign the spec.
  Say "F2 path is wrong" not "you should restructure the module."
- If ALL checks passed (CLEAN), say so clearly. Do not manufacture corrections.
- If UNVERIFIABLE (function never existed in any recent git history), say so honestly.
  Do not guess at what the scout meant.
- Distinguish between "can't verify" (outside git history / corpus not built) and
  "doesn't exist" (searched broadly, nothing found).

## Output

```
Accuracy comment posted on issue #NNN: <URL>
Label added: accuracy-reviewed
Outcome: CLEAN / CORRECTED / LIKELY FIXED / DUPLICATE / UNVERIFIABLE
Corrections for plan-reviewer: <N>
```
