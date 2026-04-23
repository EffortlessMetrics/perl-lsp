---
description: Drain green PRs sequentially (merge all passing PRs)
argument-hint: "[--dry-run] [--limit N]"
---

# Green Merge

Merge all open PRs that have passing checks. Context: **$ARGUMENTS**

## Steps

### 1. Inventory open PRs
```bash
gh pr list --state open --json number,title,headRefName,mergeable,statusCheckRollup --limit 50
```

### 2. Classify each PR
- **Green**: mergeable, no failing checks → merge
- **Conflicted**: merge conflicts → skip (use `/rebase-open` first)
- **Failing**: CI failures → skip (needs `/parser-fix` or `fixer`)
- **Draft**: skip unless `--include-drafts`

### 3. Merge green PRs sequentially
For each green PR, in dependency order:
```bash
pr=<number>
gh pr merge "$pr" --squash \
  --subject "feat(scope): <imperative summary>" \
  --body "Merge PR #$pr

- <change 1>
- <change 2>"
```
Wait for each merge to complete before the next. Sequential merging prevents race conditions.

Rules for consistency:
- Use a conventional prefix: `feat|fix|chore|refactor|docs` plus a single scope in parentheses.
- Keep the subject imperative and <=72 characters if possible.
- Make the summary specific to behavior impact, not generic PR intent.
- Put bullet list details in the body so reviewers can audit intent after squash.

Example commit subject patterns:
- `feat(perl-lsp): add linked-editing capability wiring`
- `fix(parser): recover nested here-doc parsing`
- `chore(release): add release-turnkey xtask wrapper`
- `refactor(semantic): simplify call-hierarchy parent map`

If the PR title is not conventional, convert it before merge (do not keep `Merge pull request` defaults).

Optional: derive a clearer subject from files changed:
- `scope=lsp` if files are in `crates/perl-lsp-rs/**`
- `scope=dap` if files are in `crates/perl-dap/**`
- `scope=parser` for `crates/perl-parser*`/`crates/perl-lexer*`/`crates/perl-parser-core*`
- `scope=semantic` for `crates/perl-semantic-analyzer/**`/`crates/perl-workspace-index/**`
- `scope=release` for `scripts/**`, `xtask/**`, `docs/**`, `.github/**`
Examples used in this repo:
- `feat(lsp): ...`
- `fix(parser): ...`
- `chore(release): ...`

### 4. Handle post-merge drift
After all merges complete:
```bash
# Regenerate status
python3 scripts/update-current-status.py
git diff docs/project/CURRENT_STATUS.md
# If changed, commit
```

### 5. Report
| PR | Title | Status |
|----|-------|--------|
| #N | ... | merged / skipped (reason) |
