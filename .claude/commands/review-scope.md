---
description: Review a PR for scope creep, focus violations, and oversized changes
argument-hint: "<PR-number-or-branch>"
---

# Scope Review

Review PR **$ARGUMENTS** for focus and scope compliance.

## Checklist

- [ ] PR does ONE thing (not three things bundled)
- [ ] No files changed outside the stated scope
- [ ] No "while I'm here" cleanup of unrelated code
- [ ] No new features snuck into a bug fix
- [ ] No unnecessary abstractions or helpers
- [ ] PR size is reasonable (<300 lines for most changes)
- [ ] Commit messages match actual changes
- [ ] No commented-out code or TODO markers left behind

## File Ownership Check

1. Read the PR description to identify the stated scope
2. List all changed files via `gh pr diff $ARGUMENTS --stat`
3. Check `files_touched` against the SLICE definition (if available)
4. Flag files outside the slice's `crates_affected` as potential scope creep
5. Exception: `Cargo.toml` changes for dependency additions are acceptable

## Verdict

Report one of:
- **PASS** — focused, well-scoped
- **WARN** — minor scope issues, suggest cleanup
- **FAIL** — significant scope creep, request split into separate PRs
