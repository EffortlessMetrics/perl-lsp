---
description: Reviewer step 4 — approve, apply trivial fix, or send back to builder
user-invocable: false
---

# Reviewer Decide

Based on steps 1-3, make a decision.

## Decision tree

### Default path → Improve and approve

Every PR has room for improvement. Check out the branch, push improvements (edge case tests, naming, simplification), then approve with a summary of what you changed.

```bash
gh pr checkout <number>
# ... make improvements, commit ...
git push
gh pr review <number> --approve --body "Improved: <list>. Verified: <what you checked>."
gh pr ready <number>  # if still draft
gh pr edit <number> --add-label "in-review"
```

Note: `in-review` was already set in step 1 (/reviewer-read-handoff). No label change needed here.

### Trivial issues only (typos, formatting, <5 lines) → Fix in place
1. Check out the branch:
   ```bash
   gh pr checkout <number>
   ```
2. Apply the fixes
3. Commit with: `fix(review): <what you fixed>`
4. Push
5. Approve and mark ready:
   ```bash
   gh pr review <number> --approve
   gh pr ready <number>
   ```

### Complex logic requires deep review → Mark for deep-review pass

If the PR logic is complex and would benefit from a second set of eyes before merge:
```bash
gh pr edit <number> --add-label "needs-deep-review"
```

Post a comment explaining what you'd like the deep-reviewer to focus on, then route to the deep-review agent. Do not approve yet.

After adding `needs-deep-review`, write a version-bound receipt:
```
/label-receipt-write pr <number> needs-deep-review reviewer
```

### Non-trivial issues → Fix forward if you can
Most "non-trivial" issues are still fixable on the branch. You're a sonnet-grade agent on an isolated branch — fix it, don't send it back. Bumping back is a full round trip through the queue.

**Only send back when:**
- The approach is fundamentally wrong (wrong crate, wrong architecture)
- The issue has been flagged with critical review states in earlier pipeline stages
- The codebase has moved so much the PR can't be salvaged with local fixes

If you must send back:
1. Leave specific, actionable review comments
2. `SendMessage({to: "builder"})` with the blocker list

## Rules

- **Fix forward is the default.** If you can fix it where you are, fix it.
- Never request changes for style preferences.
- "I would have done it differently" is not a blocker — make it how you'd do it and push.
- Approve unless there's a structural defect you can't resolve locally.
- **Recommend next steps.** Typical recommendations:
  - "Approved with improvements — ready for CI and merge"
  - "Approved — recommend a follow-up improvement pass on the test coverage"
  - "Marked for deep-review — complex logic, want a second set of eyes on edge cases"
