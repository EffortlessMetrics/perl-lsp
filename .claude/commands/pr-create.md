---
description: Create a PR from the current branch for perl-lsp (Rust workspace)
argument-hint: [optional: context e.g. "Issue #218", "ready", "draft", "base=master", "closes #123"]
allowed-tools: >
  Bash(git status:*), Bash(git branch:*), Bash(git rev-parse:*), Bash(git symbolic-ref:*), Bash(git remote:*),
  Bash(git merge-base:*), Bash(git log:*), Bash(git show:*), Bash(git diff:*),
  Bash(ls:*), Bash(find:*), Bash(rg:*), Bash(sed:*), Bash(awk:*), Bash(wc:*),
  Bash(mkdir:*), Bash(cat:*), Bash(tee:*), Bash(date:*),
  Bash(gh:*),
  Bash(just:*), Bash(nix:*),
  Bash(cargo:*), Bash(cargo-*:*),
  Bash(tokei:*), Bash(scc:*)
---

# Create PR (perl-lsp)

Create a pull request from the **CURRENT WORKING TREE state** of this branch.

Write the PR like maintainer notes: narrative is welcome. Center it on review signals relevant to this Rust workspace:
- **Interface integrity** (perl-parser public API, LSP protocol surface, CLI flags)
- **Risk surface delta** (`.unwrap()`/`.expect()`, concurrency, IO, deps)
- **Verification depth** (`just ci-gate` passed? LSP tests? What evidence exists?)
- **Future change-cost** (hotspots, modularity, crate boundaries)

Store artifacts when purposeful: keep a **receipt bundle you will actually cite or reuse** (diff anatomy, gate outputs, reproduction commands).

Use any extra context I provide: **$ARGUMENTS**

## Context (auto-collected)

- Branch: !`git branch --show-current`
- Status: !`git status --porcelain=v1 -b`

- Default branch: !`git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo master`

- Receipts dir (created now):
  !`BR=$(git branch --show-current | tr '/ ' '__'); TS=$(date +"%Y%m%d-%H%M%S"); DIR="target/pr-create/${TS}-${BR}"; mkdir -p "$DIR"; echo "$DIR" | tee target/pr-create/LAST_DIR`

- Baseline snapshot saved:
  !`DIR=$(cat target/pr-create/LAST_DIR); BASE=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo master); MB=$(git merge-base HEAD origin/$BASE 2>/dev/null || git merge-base HEAD $BASE); echo "base=$BASE" > "$DIR/base_branch.txt"; echo "$MB" > "$DIR/merge_base_sha.txt"; git diff --name-only $MB..HEAD > "$DIR/files.txt"; git diff --numstat $MB..HEAD > "$DIR/diff_numstat.txt"; git diff --stat $MB..HEAD > "$DIR/diff_stat.txt"; git diff --name-status $MB..HEAD > "$DIR/name_status.txt"; git log --oneline $MB..HEAD > "$DIR/commits_range.txt"; awk '{add=$1; del=$2; if(add=="-"||del=="-"){next} A+=add; D+=del} END{printf "files=%d insertions=%d deletions=%d\n", NR, A, D}' "$DIR/diff_numstat.txt" > "$DIR/summary.txt"; echo "Saved baseline to $DIR"`

- Quick summary:
  !`cat "$(cat target/pr-create/LAST_DIR)/summary.txt"`

- Crates affected:
  !`cat "$(cat target/pr-create/LAST_DIR)/files.txt" | grep -E '^crates/' | cut -d/ -f2 | sort -u | tr '\n' ' '`

- Codebase health:
  !`echo "ignored_tests=$(grep -r '#\[ignore' crates/*/tests/ 2>/dev/null | wc -l) unwraps=$(grep -r '\.unwrap()' crates/*/src/ --include='*.rs' 2>/dev/null | wc -l)"`

## perl-lsp crate structure

| Crate | Path | Focus |
|-------|------|-------|
| **perl-parser** | `crates/perl-parser/` | Main parser library, LSP providers |
| **perl-lsp** | `crates/perl-lsp/` | Standalone LSP server binary |
| **perl-dap** | `crates/perl-dap/` | Debug Adapter Protocol |
| **perl-lexer** | `crates/perl-lexer/` | Context-aware tokenizer |

## How to work (agents in waves)

### Wave 1 — Explore (map the change)
Invoke **Explore** to:
- map semantic hotspots vs mechanical changes
- identify which crates are affected and what interfaces touched
- flag risk surface deltas (`.unwrap()`, concurrency, IO, deps)
- note the gate status (has `just ci-gate` passed?)

Report with anchors (paths, commits, commands), not raw diffs.

### Wave 2 — Plan (compose the story + evidence plan)
Invoke **Plan** to:
- propose a coherent PR narrative (intent → design → review path → risk/evidence)
- produce a crisp **Interface & compatibility verdict**
- recommend which checks to cite (gate output, specific tests, health metrics)
- surface key design decisions that affect maintainability

### Wave 3 — Improve (tighten the PR content)
Invoke helpers to refine:
- Review map + semantic hotspots for reviewers
- Evidence: what was validated, reproduction path, what remains unverified
- Risk surface: `.unwrap()` delta, crate boundary changes, deps delta

### Wave 4 — Create the PR (gh)
Once `pr_title.txt` + `pr_body.md` exist, create the PR with `gh pr create`.

Default: create as **draft**, unless `$ARGUMENTS` clearly indicates "ready".

## Canonical commands

```bash
# Fast merge gate (REQUIRED before push)
nix develop -c just ci-gate

# Full CI (for larger changes)
just ci-full

# Tests by scope
cargo test --workspace --lib          # Fast library tests
just ci-lsp-def                        # LSP semantic definition tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_comprehensive_e2e_test

# Health metrics
just health                            # Codebase scoreboard
just status-check                      # Verify CURRENT_STATUS.md
```

Save outputs you cite into receipts dir (e.g., `gate.log`, `clippy.log`, `tests.log`).

## Deliverables (write these files, then create PR)

1) Write PR title to: `<receipts>/pr_title.txt`
2) Write PR body (Markdown) to: `<receipts>/pr_body.md`
3) Write an index to: `<receipts>/index.md` (what you ran, what you saved, key anchors)
4) Create PR:
   - Save the exact command to: `<receipts>/pr_create_cmd.txt`
   - Save the PR URL to: `<receipts>/pr_url.txt`

### PR body format

Use these sections:

## Summary
1–3 paragraphs: what changed + why, trade-offs, what should be true after merge.

## Interface & compatibility verdict
Crisp top-line statements:
- perl-parser public API: unchanged | additive | breaking | not measured
- LSP protocol surface: unchanged | changed | not measured
- CLI flags/config: unchanged | changed | not measured

## Design & maintainability notes
Crate boundaries, modularity, what affects future change-cost.

## What changed (narrative)
System-level explanation (not a file dump).

## How to review (fast path)
Practical map: key crates/files + semantic hotspots.

## Evidence & verification
What you ran (`just ci-gate`?), what it proves, how to reproduce.
If something wasn't run, say so.

## Risk & rollback
Blast radius, failure modes, rollback path.

## Known limits / follow-ups
Explicit deferrals and next steps.

Now: run the wave process, generate the PR title/body, save purposeful receipts, and create the PR with `gh`.
