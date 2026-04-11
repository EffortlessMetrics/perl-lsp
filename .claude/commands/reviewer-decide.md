---
description: Reviewer step 4 — improve and route to deep review, or send back to builder
user-invocable: false
---

# Reviewer Decide

Based on steps 1-3, make a decision.

## Decision tree

### Docs-only PRs → Fast-track without `reviewed-deep`

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

**Do NOT add `reviewed-deep` yourself.** That label is reserved for the deep reviewer only.

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
- **Route non-docs PRs to deep review.** Docs-only PRs may use the fast-track path above; everything else still requires reviewer-deep.
- Never request changes for style preferences.
- "I would have done it differently" is not a blocker — make it how you'd do it and push.
- **Recommend next steps.** Typical recommendations:
  - "Improved and routed to deep review — pushed edge case tests and naming fixes"
  - "Routed to deep review — recommend focus on the regex logic in parse_heredoc"
  - "Sent back to builder — approach is structurally wrong, see review comments"
