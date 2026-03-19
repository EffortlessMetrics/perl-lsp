# Enhancement Builder Prompt Template

Use this when spawning a worktree worker to add a new feature or enhancement.

## Template

```
Invoke /coding-standards.

Goal: Implement <ENHANCEMENT_DESCRIPTION>.

## Context
- Issue: <ISSUE_NUMBER_OR_LINK>
- Crate: <PRIMARY_CRATE>
- Target files: <FILE_SURFACE>
- Entry point: <WHERE_THE_NEW_CODE_WILL_BE_CALLED_FROM>

## Task List
1. <STEP_1 — e.g., "Add the new type/struct in src/types.rs">
   Skill: /coding-standards
2. <STEP_2 — e.g., "Implement the core logic in src/handler.rs">
   Skill: /coding-standards
3. <STEP_3 — e.g., "Wire it into the entry point at src/main.rs:handle_request()">
   Skill: /coding-standards
4. Add tests for the new functionality
   Skill: /verify-build <CRATE>
5. Run `python3 scripts/update-current-status.py && just status-check` if tests were added

## Decision Budget
Make at most 3 judgment calls on your own. If you face a 4th ambiguous
decision, stop and report back to the coordinator with options.

## Wiring Check
Before marking complete, verify the new code is reachable:
- The new function/method is called from an existing entry point
- The call chain is: entry point -> ... -> your new code
- grep for the function name to confirm at least one call site exists

## Rules
- Do NOT rebase. Only fix code and verify locally.
- Do NOT expand scope beyond the declared task list
- Do NOT add code that is not wired to an entry point
- If the enhancement requires changes to 3+ crates, stop and report back

## Verification
cargo fmt --all && cargo clippy -p <CRATE> --tests -- -D warnings && cargo test -p <CRATE>
```
