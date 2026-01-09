---
description: Cleanup pass to make the current branch PR-ready for perl-lsp (Rust workspace)
argument-hint: [optional: intent/constraints e.g. "minimal churn", "full gate", "skip lsp tests", "prep for issue #218"]
allowed-tools: >
  Bash(git status:*), Bash(git branch:*), Bash(git rev-parse:*), Bash(git symbolic-ref:*), Bash(git remote:*),
  Bash(git merge-base:*), Bash(git log:*), Bash(git show:*), Bash(git diff:*),
  Bash(git add:*), Bash(git restore:*), Bash(git checkout:*), Bash(git stash:*), Bash(git commit:*),
  Bash(ls:*), Bash(find:*), Bash(rg:*), Bash(sed:*), Bash(awk:*), Bash(wc:*),
  Bash(mkdir:*), Bash(cat:*), Bash(tee:*), Bash(date:*),
  Bash(just:*), Bash(nix:*),
  Bash(cargo:*), Bash(cargo-*:*),
  Bash(tokei:*), Bash(scc:*)
---

# PR Cleanup Pass (perl-lsp)

Do a **quality-first cleanup pass** on the **CURRENT WORKING TREE state** to make this branch PR-ready.

The goal is maintainability and reviewability:
- reduce future change-cost
- make interfaces/boundaries clearer (especially in `perl-parser` public API)
- make verification credible
- remove obvious footguns (lint/test/docs drift, `.unwrap()` in new code, risky patterns)

We store artifacts when purposeful: keep a **receipt bundle you will actually cite or reuse** (baseline snapshots, gate outputs, tool logs).

Use any extra context I provide: **$ARGUMENTS**

## Context (auto-collected)

- Branch: !`git branch --show-current`
- Status: !`git status --porcelain=v1 -b`

- Default branch (best effort): !`git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo master`

- Receipts dir (created now):
  !`BR=$(git branch --show-current | tr '/ ' '__'); TS=$(date +"%Y%m%d-%H%M%S"); DIR="target/pr-cleanup/${TS}-${BR}"; mkdir -p "$DIR"; echo "$DIR" | tee target/pr-cleanup/LAST_DIR`

- Baseline snapshot saved:
  !`DIR=$(cat target/pr-cleanup/LAST_DIR); BASE=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo master); MB=$(git merge-base HEAD origin/$BASE 2>/dev/null || git merge-base HEAD $BASE); echo "base=$BASE" > "$DIR/base_branch.txt"; echo "$MB" > "$DIR/merge_base_sha.txt"; git diff --name-only $MB..HEAD > "$DIR/files_before.txt"; git diff --numstat $MB..HEAD > "$DIR/diff_numstat_before.txt"; git diff --stat $MB..HEAD > "$DIR/diff_stat_before.txt"; git diff --name-status $MB..HEAD > "$DIR/name_status_before.txt"; git log --oneline $MB..HEAD > "$DIR/commits_range_before.txt"; awk '{add=$1; del=$2; if(add=="-"||del=="-"){next} A+=add; D+=del} END{printf "files=%d insertions=%d deletions=%d\n", NR, A, D}' "$DIR/diff_numstat_before.txt" > "$DIR/summary_before.txt"; echo "Saved baseline to $DIR"`

- Quick baseline summary:
  !`cat "$(cat target/pr-cleanup/LAST_DIR)/summary_before.txt"`

- Codebase health snapshot:
  !`echo "ignored_tests=$(grep -r '#\[ignore' crates/*/tests/ 2>/dev/null | wc -l) unwraps=$(grep -r '\.unwrap()' crates/*/src/ --include='*.rs' 2>/dev/null | wc -l)"`

## perl-lsp crate structure

| Crate | Path | Focus |
|-------|------|-------|
| **perl-parser** | `crates/perl-parser/` | Main parser library, LSP providers |
| **perl-lsp** | `crates/perl-lsp/` | Standalone LSP server binary |
| **perl-dap** | `crates/perl-dap/` | Debug Adapter Protocol |
| **perl-lexer** | `crates/perl-lexer/` | Context-aware tokenizer |

## How to work (agents in waves)

### Wave 1 — Explore (find what matters)
Invoke **Explore** to:
- map semantic hotspots vs mechanical changes
- flag parser/LSP interface touchpoints (`crates/perl-parser/src/lsp/`)
- flag risk surface deltas (`.unwrap()`, concurrency, IO)
- identify which crates are affected

Explore should report with anchors (paths, commands, commits), not raw diffs.

### Wave 2 — Plan (cleanup plan with maintainability intent)
Invoke **Plan** to:
- propose a cleanup plan that improves quality without scope creep
- separate "quick wins" vs "follow-ups"
- choose tooling: `just ci-gate` first, then targeted checks
- suggest commit strategy if it helps review (mechanical vs semantic)

### Wave 3 — Improve & fix (apply changes)
Invoke fixing agents to:
- run the canonical gate: `nix develop -c just ci-gate` (or `just ci-gate`)
- apply safe mechanical fixes (`cargo fmt`, clippy lints)
- reduce `.unwrap()` / `.expect()` in new code where feasible
- save tool outputs to receipts dir

### Wave 4 — Verify & report (prove readiness)
After fixes:
- re-run gate: `nix develop -c just ci-gate`
- save "after" snapshots + logs to receipts dir
- produce cleanup report with interface verdict and evidence

## Canonical commands

```bash
# Fast merge gate (REQUIRED before push, ~2-5 min)
nix develop -c just ci-gate

# Full CI (~10-20 min, for larger changes)
just ci-full

# Format + lint
cargo fmt --all
cargo clippy --workspace --lib -- -D warnings -A missing_docs

# Tests
cargo test --workspace --lib          # Fast library tests
just ci-lsp-def                        # LSP semantic definition tests

# Health metrics
just health                            # Codebase scoreboard
just status-check                      # Verify CURRENT_STATUS.md is current
```

## Output (cleanup report)

Write to: `<receipts>/cleanup_report.md` and print it.

Include:

### Cleanup summary (narrative)
What you tightened and why (maintainability + reviewability), what you deliberately didn't touch.

### Interface & compatibility verdict
- perl-parser public API: unchanged | additive | breaking | not measured
- LSP protocol surface: unchanged | changed | not measured
- CLI flags/config: unchanged | changed | not measured

Back each with anchors (paths, commands, saved outputs).

### Evidence & receipts
What you ran and where you saved it.

### What changed during cleanup
Key files/crates touched + "before → after" highlights.

### Remaining concerns / follow-ups
What's still worth doing and what to mechanize next time.

### PR readiness verdict
Ready / not ready + blockers.
If ready, recommend running `/pr-create` next (with suggested context).

Now proceed: explore → plan → apply fixes → verify → report.
