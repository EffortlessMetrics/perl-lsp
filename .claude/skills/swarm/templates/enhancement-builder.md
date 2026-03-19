# Enhancement Builder Template

Use this template when a scout has identified an existing feature to enhance.

## Prerequisites
- Scout spec with exact file paths, function signatures, and verify command
- Associated GitHub issue number

## Task List (copy and customize)

1. Invoke /coding-standards
2. Read [EXACT FILE FROM SCOUT SPEC] — understand current implementation
3. Read [EXACT TEST FILE FROM SCOUT SPEC] — understand test patterns
4. Modify [EXACT FUNCTION] to [EXACT CHANGE FROM SCOUT SPEC]
5. Add/update tests in [EXACT TEST FILE]
6. Run: python3 scripts/update-current-status.py (if tests added)
7. Verify: [EXACT VERIFY COMMAND FROM SCOUT SPEC]
8. Commit: "[conventional commit message] (#ISSUE)"
9. Create draft PR

## Rules
- Do NOT rebase onto master (merge queue handles this)
- Do NOT make changes beyond the scout spec scope
- Maximum 3 implicit decisions — everything else should be in the spec
- If the spec is unclear, STOP and report back instead of guessing
