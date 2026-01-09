---
description: Cleanup pass to make the current branch PR-ready for perl-lsp (Rust workspace)
argument-hint: [optional: intent/constraints e.g. "minimal churn", "full gate", "skip lsp tests", "prep for issue #218"]
---

# PR Cleanup Pass (perl-lsp)

Do a **quality-first cleanup pass** on the current branch to make it PR-ready.

Use any extra context provided: **$ARGUMENTS**

## Goals

- Reduce future change-cost
- Make interfaces/boundaries clearer (especially `perl-parser` public API)
- Make verification credible
- Remove footguns (lint/test drift, `.unwrap()` in new code, risky patterns)

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
- Run `git log --oneline master..HEAD` (or appropriate base) to see commits
- Run `git diff --stat master..HEAD` to see scope of changes
- Identify which crates are affected

### 2. Explore and assess

- Map semantic hotspots vs mechanical changes
- Flag parser/LSP interface touchpoints (`crates/perl-parser/src/lsp/`)
- Flag risk surface (`.unwrap()`, concurrency, IO)
- Note any obvious issues

### 3. Plan cleanup

Propose a cleanup plan:
- Separate "quick wins" vs "follow-ups"
- Avoid scope creep
- Suggest commit strategy if helpful (mechanical vs semantic)

### 4. Apply fixes

Run the canonical gate and apply fixes:

```bash
# Fast merge gate (REQUIRED before push)
nix develop -c just ci-gate

# Or if nix isn't available:
cargo fmt --all
cargo clippy --workspace --lib -- -D warnings
cargo test --workspace --lib
```

Apply safe mechanical fixes:
- `cargo fmt`
- Clippy lints
- Reduce `.unwrap()` / `.expect()` in new code where feasible

### 5. Verify and report

After fixes:
- Re-run the gate
- Confirm tests pass

## Report format

Provide a cleanup summary including:

### Summary
What you tightened and why, what you deliberately didn't touch.

### Interface verdict
- perl-parser public API: unchanged | additive | breaking | not assessed
- LSP protocol surface: unchanged | changed | not assessed
- CLI flags/config: unchanged | changed | not assessed

### What changed
Key files/crates touched.

### Remaining concerns
What's still worth doing.

### PR readiness
Ready / not ready + blockers.
If ready, recommend running `/pr-create` next.

## Start with this todo list

Use TodoWrite to create these items, then work through them:

1. Gather context (git status, branch, commits, diff stats)
2. Explore changes and assess risk surface
3. Plan cleanup approach
4. Run gate and apply fixes
5. Verify all checks pass
6. Report cleanup summary and PR readiness
