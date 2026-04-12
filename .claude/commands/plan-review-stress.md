---
description: Plan reviewer step 3 — stress-test the proposed approach
user-invocable: false
---

# Plan Review: Stress Test

Think adversarially about the scout's recommended approach.

## Steps

1. **What could go wrong with this fix?**
   - Could it break other code paths that use the same function?
   - Does it handle all variants of the construct, or just the sampled ones?
   - Could it cause regressions in existing tests?

2. **Is there a simpler approach?**
   - Read the surrounding code — is there an existing pattern for similar fixes?
   - Could a one-line change work instead of a multi-line refactor?
   - Are there other recent PRs that solved similar problems?

3. **Edge cases the scout might have missed:**
   - Nested versions of the construct
   - The construct inside strings/regex/heredocs
   - Unusual whitespace, comments, or line breaks
   - Empty or minimal versions

4. **Test completeness:**
   - Does the proposed test actually test the right thing?
   - Would it fail before the fix and pass after?
   - Are there edge case tests that should be added?

5. **What's missing from the spec?**
   - Is there enough detail for a builder to execute without research?
   - If not, **you'll add it in step 4** — note what needs filling in.

## Research Verification

Before approving the spec, check whether it makes any external claims. A spec is **claim-heavy** if it asserts ANY of the following:

- Perl language semantics (`our`, `my`, `local`, pragma behavior, signature semantics, regex flags)
- LSP 3.17/3.18 protocol behavior
- DAP protocol behavior
- External crate API behavior (tower-lsp, lsp-types, tree-sitter, etc.)
- “PR #NNNN closed this” or “this is fixed by commit SHA”
- Standard library function behavior that the fix depends on

**If ANY claim-heavy criterion is met:**
1. Dispatch the `research-verifier` agent on this issue before marking it builder-ready.
2. Wait for the `research-verified` label or a verification comment.
3. **Fallback — if network is unavailable:** add the `needs-research-verification` label to the issue instead of proceeding blind.

**If no external claims are made:** skip this step — no dispatch needed.

## Output

Record in your task:
```
Risk assessment: LOW / MEDIUM / HIGH
Simpler alternative: NONE / <description>
Missed edge cases: NONE / <list>
Test improvements: NONE / <suggestions>
Research verification: SKIPPED (no external claims) / DISPATCHED / FALLBACK LABEL SET
Attribution check: SKIPPED (no attribution claims) / VERIFIED / FLAGGED (needs-git-history-check added)
```

## Attribution Check

If the issue body or scout's analysis contains ANY of the following phrases:
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
**If claim is wrong:** remove or correct the attribution in the plan and issue. Add `needs-git-history-check` label to the issue for ops sweep.
**If uncertain:** add `needs-git-history-check` label, note it in the plan-review comment, and continue. Do not block on uncertainty — just flag it.
