---
description: Builder step 1 — read and validate the spec before implementing
user-invocable: false
---

# Builder Read Spec

Read the issue or handoff that was given to you and extract the implementation spec.

## Steps

1. If you were given an issue number, read it:
   ```bash
   gh issue view <number> --json title,body --jq '.body'
   ```

2. Extract these four required fields:
   - **File:line** — where to change (must be exact path and line number)
   - **Change** — what to change (must be specific, not "improve" or "fix")
   - **Test code** — the test to add (must be actual code, not a description)
   - **Verify command** — how to confirm (must be a runnable command)

3. Validate each field exists:
   ```
   ✓ File:line — found: crates/perl-parser-core/src/engine/parser/declarations.rs:845
   ✓ Change — found: add Colon peek before emitting phase block error
   ✓ Test code — found: #[test] fn test_check_as_label() { ... }
   ✓ Verify — found: cargo test -p perl-parser-core -- test_check_as_label
   ```

4. If ANY field is missing or vague:
   - TaskUpdate: "spec incomplete — missing: <field>"
   - STOP. Do not proceed to step 2.
   - Report back to the orchestrator.

## Output

Record in your task:
```
Spec validated:
  File: <path:line>
  Change: <one sentence>
  Test: <function name>
  Verify: <command>
```
