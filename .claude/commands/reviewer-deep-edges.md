---
description: Deep reviewer step 3 — check for edge cases the builder might have missed
user-invocable: false
---

# Deep Reviewer Edge Cases

Think about what the builder didn't think about.

## Steps

1. **Functional edge cases:**
   - For parser: nested constructs, inside string/regex/heredoc, unusual whitespace, empty/minimal
   - For LSP: empty document, file boundaries, unicode identifiers, files with parse errors
   - For all: what happens with unexpected input? Does it fail gracefully?

2. **Security check** (especially for DAP, subprocess calls, file operations):
   - Command injection: are any strings interpolated into shell commands?
   - Path traversal: are file paths validated before use?
   - Untrusted input: does user-supplied content flow into dangerous operations?
   - Information leakage: do error messages expose internal paths or state?

3. **Performance check:**
   - Could this change cause O(n²) behavior on large inputs?
   - Are there unnecessary allocations in a hot path?
   - Could this block the main thread?

4. For each finding, **fix it on the branch:**
   - Missing edge case test? Write it and commit.
   - Logic bug? Fix the logic and commit.
   - Only file a follow-up if it's genuinely out of scope for this PR.

## Output

Record in your task:
```
Edge cases found and fixed: <list of commits pushed>
Security: CLEAN / <findings — fix if possible>
Performance: CLEAN / <findings — fix if possible>
Out-of-scope follow-ups: <list or NONE>
```
