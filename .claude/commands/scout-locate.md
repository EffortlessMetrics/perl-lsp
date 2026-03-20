---
description: Scout step 2 — find exact file and line locations for the finding
user-invocable: false
---

# Scout Locate

Find every relevant code location. Your output is a list of file:line references.

## Steps

1. **Grep for the error/feature name** in the codebase:
   ```
   Grep for the error message, function name, or feature keyword
   ```

2. **Read the relevant functions** — don't just find them, read them:
   ```
   Read file_path at the function that handles this case
   ```

3. **Trace the call chain** — how does execution reach this code?
   - What calls this function?
   - What dispatches to it?

4. **Check tests** — do existing tests cover this?
   ```
   Grep for test functions related to this feature/error
   ```

## Output

Record in your task:
```
Files:
- crates/<crate>/src/<file>.rs:<line> — <what this location does>
- crates/<crate>/src/<file>.rs:<line> — <what this location does>
Tests:
- crates/<crate>/tests/<file>.rs — <existing coverage or "none">
```

Do NOT move to step 3 until you have at least one file:line reference.
