---
description: Spec planner step 2 — verify all paths, functions, and signatures exist now
user-invocable: false
---

# Spec Planner: Verify

Confirm every file path, function name, and type signature in the spec
against the current codebase. Specs go stale fast — PRs merge daily.

## Steps

1. For each file path in the spec:
   ```bash
   # Verify file exists
   test -f "<path>" && echo "EXISTS" || echo "MISSING: <path>"
   ```

2. For each function/struct/enum mentioned:
   ```bash
   # Verify it exists and check current line number
   grep -n "fn <name>\|struct <name>\|enum <name>" <path>
   ```

3. For each line number reference:
   - Read the file at that line and ±10 lines
   - Confirm the content matches what the spec describes
   - Note updated line numbers if they've shifted

4. Check for callers/consumers of functions being modified:
   ```bash
   grep -r "<function_name>" --include="*.rs" -l
   ```

5. Check for conflicting in-flight work:
   ```bash
   gh pr list --search "<filename>" --state open
   gh issue list --label "in-build" --search "<crate-name>"
   ```

## Output

```
Verification:
  ✓ <path>:<line> — <function> exists, line accurate
  ⚠ <path>:<line> — shifted to line <new>, content matches
  ✗ <path> — MISSING or renamed
  
Callers: <function> called from N files: <list>
Conflicts: <any open PRs touching same files>
```
