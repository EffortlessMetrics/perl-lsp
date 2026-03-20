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

4. For each finding:
   - Is it covered by an existing test?
   - Would it cause a crash, security issue, or performance regression?
   - Is it worth blocking the PR or filing a follow-up?

## Output

Record in your task:
```
Edge cases found: <list>
Security: CLEAN / <findings>
Performance: CLEAN / <findings>
Blocking: <list or NONE>
Follow-up: <list or NONE>
```
