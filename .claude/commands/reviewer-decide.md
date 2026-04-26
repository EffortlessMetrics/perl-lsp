---
description: Reviewer step 4 — improve and route to deep review, or send back to builder
user-invocable: false
---

# Reviewer Decide

Based on steps 1-3, make a decision.

## Operating principle (per the 2026-04-26 directive)

Sign-off IS one of the routing decisions. Each pass produces exactly ONE outcome:

- **Gate clean** → apply `review-reviewed` (and only `review-reviewed`)
- **Mechanical fix applied** → push the fix; the post-fix state is gate-clean → apply `review-reviewed`
- **Bounce back (blocker found)** → apply `needs-builder-fix` (and only `needs-builder-fix`); do **NOT** also apply `review-reviewed`

The contradictory state of `review-reviewed` AND `needs-builder-fix` simultaneously (the 2026-04-26 #6780 incident) is forbidden — it lets unfixed bugs ride the merge gate.

**Default posture**: every PR is potentially problematic until you've substantively cleared it. "Approved with no changes" is almost never right — find something concrete to flag, fix, or improve. Thin LGTM-shaped output without a single substantive observation is itself a signal you didn't look hard enough.

## Decision tree

### Docs-only PRs → Fast-track without `deep-reviewed`

If every changed file is documentation-only (`docs/**` or doc-text files such as `.md`, `.mdx`, `.txt`, `.rst`, `.adoc`), do the standards pass, push any doc fixes, and route straight to `/pr-ready`.

```bash
gh pr checkout <number>
# ... improve wording / links / receipts as needed ...
git push
gh pr comment <number> --body "Standards review complete. Docs-only fast-track used; no reviewer-deep pass required."
```

Then call:
```
/pr-ready <number>
```

**Do NOT add `deep-reviewed` yourself.** That label is reserved for the deep reviewer only.

### Default path → Improve and route to deep review

Every PR has room for improvement. Check out the branch, push improvements (edge case tests, naming, simplification), then route to deep review. **Never approve directly** — the standards reviewer does NOT approve PRs.

```bash
gh pr checkout <number>
# ... make improvements, commit ...
git push
```

After pushing improvements, set sign-off and route to deep review:
```bash
gh pr edit <number> --add-label "review-reviewed"
gh pr edit <number> --add-label "needs-deep-review"
gh pr comment <number> --body "Standards review complete. Improved: <list of changes>. Deep reviewer: focus on <areas of concern>."
```

Then write a version-bound receipt:
```
/label-receipt-write pr <number> needs-deep-review reviewer
```

**Do NOT call `gh pr review --approve`.** The reviewer's job is the standards pass only. Deep review is the approval gate.

### Blocker found → Send back WITHOUT sign-off

When you find substantive blocking issues that you cannot mechanically fix forward:
- Wrong language reference, hallucinated API call, scope mismatch between title and diff
- Missing required code that the title claims (e.g., title says "fix manifest" but diff is docs-only)
- Test regression a fix-forward would mask
- Cross-PR contamination in source/test files (not just `.hermes/`)
- Banned production patterns the builder must address

Apply ONLY the routing label, NOT the sign-off:
```bash
gh pr edit <number> --add-label "needs-builder-fix"
gh pr comment <number> --body "Standards review: NEEDS BUILDER — <specific blockers>.

Blockers:
1. <file:line> — <what's wrong> — <what to do>
2. ...

Not signing off (\`review-reviewed\` not applied) per the 2026-04-26 sign-off-as-routing rule. Will re-run after fix."
```

**DO NOT also apply `review-reviewed`.** Sign-off and bounce are mutually exclusive — they are the same routing decision with different outcomes. Applying both is the #6780 failure mode (let unfixed bugs ride to merge).

### Structural problems → Send back to builder

**Only send back at this severity when:**
- The approach is fundamentally wrong (wrong crate, wrong architecture)
- The issue has been flagged with critical review states in earlier pipeline stages
- The codebase has moved so much the PR can't be salvaged with local fixes

If you must send back at structural severity:
1. Leave specific, actionable review comments
2. Apply `needs-builder-fix` (NOT `review-reviewed`)
3. `SendMessage({to: "builder"})` with the blocker list

## Rules

- **Fix forward is the default.** If you can fix it where you are, fix it.
- **Route non-docs PRs to deep review.** Docs-only PRs may use the fast-track path above; everything else still requires reviewer-deep.
- Never request changes for style preferences.
- "I would have done it differently" is not a blocker — make it how you'd do it and push.
- **Recommend next steps.** Typical recommendations:
  - "Improved and routed to deep review — pushed edge case tests and naming fixes"
  - "Routed to deep review — recommend focus on the regex logic in parse_heredoc"
  - "Sent back to builder — approach is structurally wrong, see review comments"
