# Refactoring Builder Prompt Template

Use this when spawning a worktree worker for a refactoring task (SRP splits, dead code removal, API cleanup).

## Template

```
Invoke /coding-standards.

Goal: Refactor <CRATE_NAME> — <REFACTORING_DESCRIPTION>.

## Context
- Issue: <ISSUE_NUMBER_OR_LINK>
- Motivation: <WHY_THIS_REFACTORING>
- Target files: <FILE_SURFACE>
- Downstream dependents: <CRATES_THAT_DEPEND_ON_THIS>

## Steps
1. Read the target files and map the current structure
2. Identify all callers/dependents with grep before moving code
3. Make the structural change in small, verifiable steps
4. Ensure all public APIs remain backward-compatible unless explicitly breaking
5. Run verification against this crate AND downstream dependents
6. Commit: `refactor(<CRATE>): <description>`

## Verification
cargo fmt --all && cargo clippy -p <CRATE> --tests -- -D warnings && cargo test -p <CRATE>

If 3+ crates affected:
nix develop -c just ci-gate

## Out of Scope
- Do not change behavior — this is a structural change only
- Do not add features or fix bugs alongside the refactoring
- Do not touch files outside the declared file surface
```
