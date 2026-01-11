---
description: PR cleanup pass (perl-lsp)
argument-hint: "optional constraints like 'minimal churn' or 'skip tests'"
---

# PR Cleanup Pass

Clean up the current branch for review. Context: **$ARGUMENTS**

## Use TodoWrite to track these steps:

1. Gather context (branch, base, diff scope)
2. Identify risks and hotspots
3. Run gate and fix failures
4. Apply cleanup fixes
5. Verify gate is green
6. Output cleanup report

## Step 1: Gather context

Run these Bash commands in parallel:
- `git status -sb`
- `git branch --show-current`
- `git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null || echo origin/master`

Then with the base branch:
- `git log --oneline <base>..HEAD`
- `git diff --stat <base>..HEAD`

Output a scope summary: which crates changed, semantic vs mechanical changes.

## Step 2: Identify risks

Use Grep to search for these patterns in changed files:
- `unwrap\(` and `expect\(` - panic sites
- Public API changes in `crates/perl-parser/src/lib.rs`
- LSP handler changes in `crates/perl-parser/src/lsp/`

## Step 3: Run gate

```bash
just ci-gate
```

Fallback if just unavailable:
```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --lib
```

Fix failures using Edit tool. Stay focused - don't expand scope.

## Step 4: Cleanup actions

Use Edit tool to:
- Fix formatting/clippy issues
- Remove dead code from this branch
- Add doc comments where logic is unclear

Keep changes minimal.

## Step 5: Verify

Re-run the gate. Confirm green.

## Step 6: Report

Output in chat:

```
### Summary
What changed and why. What you didn't touch.

### Interface verdict
- perl-parser API: unchanged | additive | breaking
- LSP surface: unchanged | changed
- DAP surface: unchanged | changed
- CLI: unchanged | changed

### Evidence
Commands run and results.

### Remaining concerns
Follow-up items if any.

### PR readiness
Ready / Not ready + blockers.
```

If ready, suggest `/pr-create`.
