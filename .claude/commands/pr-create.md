---
description: Create PR (perl-lsp)
argument-hint: "optional like 'closes #123' or 'draft'"
---

# Create PR

Create a well-structured PR. Context: **$ARGUMENTS**

## Use TodoWrite to track these steps:

1. Gather context (branch, base, commits)
2. Assess impact and risks
3. Draft PR title and body
4. Verify gate is green
5. Push and create PR

## Step 1: Gather context

Run these Bash commands in parallel:
- `git status -sb`
- `git branch --show-current`
- `(git symbolic-ref -q refs/remotes/origin/HEAD 2>/dev/null || echo refs/remotes/origin/master) | sed 's@^refs/remotes/origin/@origin/@'`
- `git remote -v`

Then with the base branch:
- `git log --oneline <base>..HEAD`
- `git diff --stat <base>..HEAD`

## Step 2: Assess impact

Determine:
- **Interface changes**: perl-parser API, LSP, DAP, CLI
- **Risk surface**: panic sites, concurrency, IO paths
- **Test coverage**: what was tested

Use Grep on changed files if needed.

## Step 3: Draft PR

Format:

**Title**: `<type>(<scope>): <description>`
- Types: fix, feat, refactor, docs, test, chore, ci
- Example: `fix(parser): handle empty heredocs correctly`

**Body** (use HEREDOC with gh):
```
## Summary
1-3 paragraphs: what, why, trade-offs.

## Interface & compatibility
- perl-parser API: unchanged | additive | breaking
- LSP surface: unchanged | changed
- DAP surface: unchanged | changed
- CLI: unchanged | changed

## What changed
System-level explanation.

## How to review
Where to start, hotspots.

## Evidence
\`\`\`
<paste gate output>
\`\`\`

## Risk & rollback
Blast radius, failure modes, rollback path.

## Follow-ups
Explicit deferrals if any.
```

## Step 4: Verify gate

```bash
just ci-gate
```

If not green, fix or document what remains.

## Step 5: Push and create

```bash
git push -u origin HEAD
```

Then create PR:
```bash
gh pr create --title "<title>" --body "$(cat <<'EOF'
<body content>
EOF
)"
```

If gh unavailable, output the title and body for manual creation.

Return the PR URL when done.
