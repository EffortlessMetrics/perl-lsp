---
description: PR cleanup pass (perl-lsp)
argument-hint: [optional: constraints e.g. "minimal churn", "skip e2e", "docs-only", "prep for #123"]
---

# PR Cleanup Pass (perl-lsp)

Goal: improve the **current working tree** so it's easier to review and safer to merge.

Use any extra context provided: **$ARGUMENTS**

## Start with this todo list

Use TodoWrite to create these items, then work through them:

1. Gather context (branch, status, base branch, diff scope)
2. Identify hotspots + risk surface
3. Run the best available gate and fix what fails
4. Tighten reviewability (small refactors, docs drift, footguns)
5. Re-run verification and confirm green
6. Write a cleanup report + "PR ready?" verdict

## 1) Gather context (portable)

Run:
- `git status -sb`
- `git branch --show-current`
- Determine the base branch (prefer `origin/HEAD`, else `main`/`master` as appropriate)
- `git log --oneline <base>..HEAD`
- `git diff --stat <base>..HEAD`

Output a short "scope map":
- which crates changed
- which areas are semantic vs mechanical

## 2) Identify hotspots and risks

Look specifically for:
- boundary changes (`perl-parser` public API, LSP/DAP surfaces)
- user-input paths / UTF-8 / string indexing
- concurrency/locking patterns
- unwrap/expect regressions (ratchet scripts)

## 3) Run the best available gate (mode ladder)

Prefer, in order:

1) If Nix is available:
```bash
nix develop -c just ci-gate
```

2) If Just is available:
```bash
just ci-gate
```

3) Pure Cargo fallback:
```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib
```

Fix failures iteratively. Avoid scope creep.

## 4) Cleanup actions (keep it PR-friendly)

Prioritize:

* formatting + clippy
* eliminate obvious panics introduced in new code
* reduce diff noise (remove dead code, tighten names, add doc comments where needed)
* keep changes local; don't redesign architecture during cleanup

## 5) Verify

Re-run the chosen gate and confirm:

* compile
* tests
* ratchet scripts (unwraps / module checks) if present

## 6) Cleanup report (print in chat)

Include:

### Summary

What you changed and why. What you deliberately did not touch.

### Interface verdict

* perl-parser public API: unchanged | additive | breaking | not assessed
* LSP protocol surface: unchanged | changed | not assessed
* DAP protocol surface: unchanged | changed | not assessed
* CLI flags/config: unchanged | changed | not assessed

### Evidence

What you ran and what passed. Provide reproduction commands.

### Remaining concerns

Anything worth a follow-up issue.

### PR readiness

Ready / not ready + blockers.
If ready, recommend running `/pr-create`.
