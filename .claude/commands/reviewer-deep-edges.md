---
description: Deep reviewer step 3 — check for edge cases the builder might have missed
---

# Deep Reviewer Edge Cases

Think about what the builder didn't think about.

## Steps

1. For parser fixes, check:
   - What happens with nested versions of this construct?
   - What about the construct inside a string/regex/heredoc?
   - What about the construct with unusual whitespace/comments?
   - What about empty or minimal versions?

2. For LSP features, check:
   - What happens with an empty document?
   - What happens at file boundaries (first/last line)?
   - What happens with unicode identifiers?
   - What happens if the file has parse errors?

3. For each edge case you find:
   - Is it covered by an existing test?
   - Would it cause a crash/panic?
   - Is it worth blocking the PR or filing a follow-up?

## Output

Record in your task:
```
Edge cases found: <list>
Blocking: <list or NONE>
Follow-up: <list or NONE>
```
