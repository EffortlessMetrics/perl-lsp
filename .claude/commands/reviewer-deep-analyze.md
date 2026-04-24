---
description: Deep reviewer step 2 — analyze if the diff logic is correct
user-invocable: false
---

# Deep Reviewer Analyze

Read the diff carefully and verify the logic matches the issue's intent.

## Steps

1. Read the full diff:
   ```bash
   gh pr diff <number>
   ```

2. For each changed file, ask:
   - Does this change address the root cause from the issue?
   - Is the approach the one recommended in the issue, or different?
   - If different, is the alternative approach sound?

3. Check the test:
   - Does the test input match the reproduction from the issue?
   - Does it assert the right behavior (not just "no crash")?
   - Would the test have failed BEFORE the fix?

4. Check for **vacuous assertions** — assertions on hardcoded data that prove nothing about the code under test:
   - `assert!(vec.len() > 0)` where the Vec was constructed from a hardcoded non-empty list
   - `assert!(!s.is_empty())` on a string literal
   - `assert_eq!(result.len(), N)` when result was built directly from N hardcoded items
   - `assert!(result.is_some())` when result is `Some(hardcoded_value)`
   - The litmus test: would this assertion still pass if the feature code were commented out? If yes, the test is vacuous.

5. Check for regressions:
   - Does the change affect any other code paths?
   - Could existing callers break?
   - Are there related tests that might need updating?

6. **Fix forward:**
   - Logic slightly off? Fix it on the branch.
   - Test only asserts "no crash" instead of behavior? Strengthen the assertion.
   - Vacuous assertion? Rewrite to test actual behavior of the code under test.
   - Regression risk from an uncovered path? Add a test for it.

## Research Verification

Before approving, check whether the PR makes any external claims. A PR is **claim-heavy** if it asserts ANY of the following:

- Perl language semantics (`our`, `my`, `local`, pragma behavior, signature semantics, regex flags)
- LSP 3.17/3.18 protocol behavior
- DAP protocol behavior
- External crate API behavior (tower-lsp, lsp-types, tree-sitter, etc.)
- “PR #NNNN closed this” or “this is fixed by commit SHA”
- Standard library function behavior that the fix depends on

**If ANY claim-heavy criterion is met:**
1. Dispatch the `research-verifier` agent on the original issue before approving.
2. Wait for the `research-reviewed` label or a verification comment.
3. **Fallback — if network is unavailable:** add the `needs-research-verification` label to the PR and block approval. Do not merge blind.

**If no external claims are made:** skip this step — no dispatch needed.

## Output

Record in your task:
```
Logic: CORRECT / FIXED <what you changed>
Tests: GOOD / IMPROVED <what you added>
Regression risk: LOW / MEDIUM / HIGH (details)
Research verification: SKIPPED (no external claims) / DISPATCHED / FALLBACK LABEL SET
Attribution check: SKIPPED (no attribution claims) / VERIFIED / FLAGGED (needs-git-history-check added)
```

## Attribution Check

If the PR description or issue body contains ANY of the following phrases:
- "fixed by PR #NNNN"
- "already shipped in commit SHA"
- "this issue is stale / superseded by #NNNN"
- "closed by #NNNN"

Run the git-history check before proceeding:

```bash
# Verify the PR actually merged and closed the right issue
gh pr view <NNNN> --json state,mergedAt,closingIssuesReferences
# Verify the fix is present in master
git log --oneline master | grep -i <keyword>
```

**If claim checks out:** note `Attribution: VERIFIED` in your output.
**If claim is wrong:** remove or correct the attribution on the PR branch. Add `needs-git-history-check` label to the original issue for ops sweep.
**If uncertain:** add `needs-git-history-check` label, note it in the deep-review comment, and continue. Do not block on uncertainty — just flag it.
