---
description: Builder step 3 — implement the fix described in the spec
user-invocable: false
---

# Builder Implement

Make the change. Minimal diff. Exactly what the spec says.

## Steps

1. Open the file from the spec at the specified line number

2. Make the change described in the spec. Keep the diff small:
   - Only touch the files listed in the spec
   - Don't refactor surrounding code
   - Don't add comments unless the logic is non-obvious
   - Don't add features beyond the spec

3. Run the test from step 2 — it should now PASS:
   ```bash
   cargo test -p <crate> -- <test_name> --exact 2>&1
   ```

4. If the test still fails, debug:
   - Re-read the spec's root cause analysis
   - Check if you changed the right location
   - Check if the fix logic matches what was recommended

5. Run ALL tests in the crate to catch regressions:
   ```bash
   cargo test -p <crate> 2>&1
   ```

## Coding standards

- No `unwrap()`, `expect()`, `panic!()`, `todo!()` in production code
- Use `?`, `.ok_or_else()`, pattern matching, `Result`/`Option`
- Prefer `.first()` over `.get(0)`
- Regex: `Option<Regex>` with `.ok()` for graceful degradation

## Scope guard

If you discover something that needs fixing but isn't in the spec:
- **Small and on the same code path?** Fix it — you're already here.
- **Different concern or crate?** Note it for the orchestrator: "Discovered: <issue> — recommend a follow-up scout/builder"
- **Spec was wrong about the root cause?** If it hasn't been plan-reviewed, consider bumping back. If it has, adapt and fix forward.

## Output

Record in your task:
```
Files changed: <list>
Lines changed: <count>
Test result: PASS / FAIL
Regressions: NONE / <list>
```
