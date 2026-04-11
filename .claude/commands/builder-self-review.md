---
description: Builder step 5 — re-read your own diff before publishing
user-invocable: false
---

# Builder Self-Review

Before creating the PR, re-read your own diff with fresh eyes.
This catches dumb mistakes before a reviewer has to.

## Steps

1. Read your diff:
   ```bash
   git diff HEAD~1
   ```

2. Check each changed file:
   - Does this change match what the spec asked for?
   - Did I accidentally include debug code, extra files, or unrelated changes?
   - Are my test names descriptive?
   - Is the diff minimal — no unnecessary whitespace, reformatting, or refactoring?

3. Check your test:
   - Does it test behavior, not implementation details?
   - Would it fail before the fix and pass after?
   - Are edge cases from the plan-reviewer's comments covered?

4. Quick sanity:
   - Any `unwrap()`, `expect()`, `panic!()`, `todo!()`, `dbg!()` that slipped in?
   - Any `.clone()` on Copy types?
   - Any commented-out code?

5. **Fix everything you find** — don't just note it, fix it now. Re-run `/verify`, then continue.

6. **Look for improvements** beyond just correctness:
   - Can any code be simplified?
   - Are test names clear and descriptive?
   - Are there edge cases worth one more test?

## Research Verification

Before publishing, check whether your diff makes any external claims. A PR is **claim-heavy** if it asserts ANY of the following:

- Perl language semantics (`our`, `my`, `local`, pragma behavior, signature semantics, regex flags)
- LSP 3.17/3.18 protocol behavior
- DAP protocol behavior
- External crate API behavior (tower-lsp, lsp-types, tree-sitter, etc.)
- “PR #NNNN closed this” or “this is fixed by commit SHA”
- Standard library function behavior that the fix depends on

**If ANY claim-heavy criterion is met:**
1. Dispatch the `research-verifier` agent on the issue (not the PR) before creating the PR.
2. Wait for the `research-verified` label or a verification comment.
3. **Fallback — if network is unavailable:** add the `needs-research-verification` label to the PR and note it in the PR description. Do not merge blind.

**If no external claims are made:** skip this step — no dispatch needed.

## Output

Record in your task:
```
Self-review: CLEAN / FIXED <what>
Diff size: <lines added/removed>
Files changed: <count>
Research verification: SKIPPED (no external claims) / DISPATCHED / FALLBACK LABEL SET
Recommend: <next step, e.g.:
  - "Ready for review — clean implementation"
  - "Needs a follow-up builder for edge case X I discovered but is out of scope"
  - "Recommend accuracy scout — the spec's root cause was wrong, I adapted but want verification"
>
```
