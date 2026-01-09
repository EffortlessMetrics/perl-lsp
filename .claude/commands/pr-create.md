---
description: Create PR (perl-lsp)
argument-hint: [optional: context e.g. "closes #123", "draft", "ready", "base=main"]
---

# Create PR (perl-lsp)

Goal: produce a reviewable PR: good narrative, clear risk surface, reproducible verification.

Use any extra context provided: **$ARGUMENTS**

## Start with this todo list

Use TodoWrite to create these items, then work through them:

1. Gather context (branch, base, diff scope)
2. Decide PR intent + interface verdict
3. Draft PR title + body
4. Ensure gate is green (or declare what isn't)
5. Push branch if needed
6. Create PR with `gh` (or output copy/pasteable title/body)

## 1) Gather context

Run:
- `git status -sb`
- Determine base branch (prefer `origin/HEAD`, else `main`/`master` as appropriate)
- `git log --oneline <base>..HEAD`
- `git diff --stat <base>..HEAD`
- Identify affected crates and key files

## 2) Decide the top-line verdicts

You must state:
- interface impact (API / protocol / CLI)
- risk surface delta (panic sites, concurrency, IO, deps)
- verification depth (what was run, what wasn't)

## 3) Draft PR title + body (write in chat)

Use this structure:

### Summary
1–3 paragraphs: what changed, why, trade-offs.

### Interface & compatibility
- perl-parser public API: unchanged | additive | breaking | not assessed
- LSP protocol surface: unchanged | changed | not assessed
- DAP protocol surface: unchanged | changed | not assessed
- CLI flags/config: unchanged | changed | not assessed

### What changed
System-level explanation (not a file dump).

### How to review
Where to start + hotspots.

### Evidence
Exact commands and results. Say what wasn't run.

### Risk & rollback
Blast radius, failure modes, rollback path.

### Known limits / follow-ups
Explicit deferrals.

## 4) Gate status (mode ladder)

Prefer:
```bash
nix develop -c just ci-gate
```

Fallbacks:
```bash
just ci-gate
cargo test --workspace --lib
```

If not green, explicitly say why and what remains.

## 5) Push + create PR

If `gh` is available:
```bash
git push -u origin HEAD
gh pr create --draft --title "…" --body "…"
```

If `gh` isn't available:
* output a copy/pasteable PR title + PR body in chat.
