---
description: Create PR (perl-lsp)
argument-hint: [optional: context e.g. "Issue #218", "ready", "draft", "base=master", "closes #123"]
---

# Create PR (perl-lsp)

Create a pull request from the current branch.

Use any extra context provided: **$ARGUMENTS**

## Crate structure

| Crate | Path | Focus |
|-------|------|-------|
| **perl-parser** | `crates/perl-parser/` | Main parser library, LSP providers |
| **perl-lsp** | `crates/perl-lsp/` | Standalone LSP server binary |
| **perl-dap** | `crates/perl-dap/` | Debug Adapter Protocol |
| **perl-lexer** | `crates/perl-lexer/` | Context-aware tokenizer |

## Workflow

### 1. Gather context

First, understand the current state:
- Run `git status` and `git branch --show-current`
- Run `git log --oneline master..HEAD` to see commits
- Run `git diff --stat master..HEAD` to see scope
- Check if branch is pushed: `git status -sb`
- Identify which crates are affected

### 2. Assess the change

- Map semantic hotspots vs mechanical changes
- Identify interface touchpoints
- Note the gate status (has `just ci-gate` passed?)
- Flag risk surface (`.unwrap()`, concurrency, IO)

### 3. Compose the PR

Write a PR with these sections:

#### Summary
1-3 paragraphs: what changed, why, trade-offs.

#### Interface & compatibility
- perl-parser public API: unchanged | additive | breaking | not assessed
- LSP protocol surface: unchanged | changed | not assessed
- CLI flags/config: unchanged | changed | not assessed

#### What changed
System-level explanation (not a file dump).

#### How to review
Key crates/files + semantic hotspots.

#### Evidence
What was validated (`just ci-gate`?), what remains unverified.

#### Risk & rollback
Blast radius, failure modes.

#### Known limits / follow-ups
Explicit deferrals.

### 4. Create the PR

Push the branch if needed:
```bash
git push -u origin HEAD
```

Create the PR (draft by default unless "ready" specified):
```bash
gh pr create --draft --title "..." --body "..."
```

## Key commands

```bash
# Gate (should pass before PR)
nix develop -c just ci-gate

# Tests
cargo test --workspace --lib
just ci-lsp-def

# Health
just health
```

## Start with this todo list

Use TodoWrite to create these items, then work through them:

1. Gather context (git status, branch, commits, diff stats)
2. Assess changes and identify interface touchpoints
3. Compose PR title and body
4. Push branch if needed
5. Create PR with gh
