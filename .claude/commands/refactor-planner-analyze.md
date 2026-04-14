---
description: Refactor planner step 2 — identify simplification, reuse, and quality opportunities
user-invocable: false
---

# Refactor Planner: Analyze

Systematically scan the diff for refactoring opportunities.

## Checklist

1. **Duplication** — grep for similar code blocks in the diff:
   ```bash
   # Look for repeated patterns
   git diff origin/master..HEAD | grep -E "^\+" | sort | uniq -d
   ```

2. **Existing helpers** — check if the crate already has what the builder hand-rolled:
   ```bash
   grep -r "fn " crates/<crate>/src/ --include="*.rs" -l
   ```

3. **Dead code** — clippy already ran in read step, note unused items.

4. **Complexity** — read each new function. Count nesting levels. >3 levels = flag.

5. **Type tightness** — scan for `String` params that could be `&str`, `Vec` that could be `&[T]`.

6. **Error handling** — scan for verbose `match Ok/Err` that could be `?`.

7. **Reuse across the workspace** — check if a sibling crate has a utility:
   ```bash
   grep -r "<pattern>" crates/*/src/ --include="*.rs" -l
   ```

For each finding, note the exact file:line and what the change would be.
