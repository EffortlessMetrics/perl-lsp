---
description: Reviewer step 4 — improve and route to deep review, or send back to builder
user-invocable: false
---

# Reviewer Decide

Based on steps 1-3, make a decision.

## Decision tree

### Default path → Improve and route to deep review

Every PR has room for improvement. Check out the branch, push improvements (edge case tests, naming, simplification), then route to deep review. **Never approve directly** — the standards reviewer does NOT approve PRs.

```bash
gh pr checkout <number>
# ... make improvements, commit ...
git push
```

After pushing improvements, ALWAYS route to deep review:
```bash
gh pr edit <number> --add-label "needs-deep-review"
gh pr comment <number> --body "Standards review complete. Improved: <list of changes>. Deep reviewer: focus on <areas of concern>."
```

Then write a version-bound receipt:
```
/label-receipt-write pr <number> needs-deep-review reviewer
```

**Do NOT call `gh pr review --approve`.** The reviewer's job is the standards pass only. Deep review is the approval gate.

### Structural problems → Send back to builder

**Only send back when:**
- The approach is fundamentally wrong (wrong crate, wrong architecture)
- The issue has been flagged with critical review states in earlier pipeline stages
- The codebase has moved so much the PR can't be salvaged with local fixes

If you must send back:
1. Leave specific, actionable review comments
2. `SendMessage({to: "builder"})` with the blocker list

## Rules

- **Fix forward is the default.** If you can fix it where you are, fix it.
- **ALWAYS route to deep review.** Never approve directly. The reviewer does the standards pass; the deep reviewer does the correctness pass and approves.
- Never request changes for style preferences.
- "I would have done it differently" is not a blocker — make it how you'd do it and push.
- **Recommend next steps.** Typical recommendations:
  - "Improved and routed to deep review — pushed edge case tests and naming fixes"
  - "Routed to deep review — recommend focus on the regex logic in parse_heredoc"
  - "Sent back to builder — approach is structurally wrong, see review comments"
