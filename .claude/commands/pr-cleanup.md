---
description: Cleanup pass to make the current branch PR-ready (agents in waves; apply fixes; save purposeful receipts; report)
argument-hint: [optional: intent/constraints e.g. "minimal churn", "run full gate", "skip benches", "prep for issue #218"]
allowed-tools: >
  Bash(git status:*), Bash(git branch:*), Bash(git rev-parse:*), Bash(git symbolic-ref:*), Bash(git remote:*),
  Bash(git merge-base:*), Bash(git log:*), Bash(git show:*), Bash(git diff:*),
  Bash(git add:*), Bash(git restore:*), Bash(git checkout:*), Bash(git stash:*), Bash(git commit:*),
  Bash(ls:*), Bash(find:*), Bash(rg:*), Bash(sed:*), Bash(awk:*), Bash(wc:*),
  Bash(mkdir:*), Bash(cat:*), Bash(tee:*), Bash(date:*),
  Bash(make:*), Bash(just:*), Bash(nix:*),
  Bash(cargo:*), Bash(cargo-*:*) , Bash(pytest:*), Bash(ruff:*), Bash(mypy:*), Bash(pyright:*),
  Bash(node:*), Bash(npm:*), Bash(pnpm:*), Bash(yarn:*),
  Bash(tokei:*), Bash(scc:*), Bash(lizard:*), Bash(radon:*),
  Bash(lychee:*),
  Bash(pip-audit:*), Bash(pip:*), Bash(uv:*), Bash(poetry:*),
  Bash(gitleaks:*), Bash(trufflehog:*)
---

# PR Cleanup Pass (current branch)

Do a **quality-first cleanup pass** on the **CURRENT WORKING TREE state** to make this branch PR-ready.

The goal is maintainability and reviewability:
- reduce future change-cost
- make interfaces/boundaries clearer
- make verification credible
- remove obvious footguns (lint/test/docs drift, dependency posture, risky patterns)

We can store artifacts when we're purposeful with them: keep a **purposeful receipt bundle you will actually cite or reuse** (baseline snapshots, gate outputs, tool logs you reference, etc.).

Use any extra context I provide: **$ARGUMENTS**

## Context (auto-collected)

- Branch: !`git branch --show-current`
- Status: !`git status --porcelain=v1 -b`

- Default branch (best effort): !`git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo main`

- Receipts dir (created now):
  !`BR=$(git branch --show-current | tr '/ ' '__'); TS=$(date +"%Y%m%d-%H%M%S"); DIR="target/pr-cleanup/${TS}-${BR}"; mkdir -p "$DIR"; echo "$DIR" | tee target/pr-cleanup/LAST_DIR`

- Baseline snapshot saved (no large diff output printed):
  !`DIR=$(cat target/pr-cleanup/LAST_DIR); BASE=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo main); MB=$(git merge-base HEAD origin/$BASE 2>/dev/null || git merge-base HEAD $BASE); echo "base=$BASE" > "$DIR/base_branch.txt"; echo "$MB" > "$DIR/merge_base_sha.txt"; git diff --name-only $MB..HEAD > "$DIR/files_before.txt"; git diff --numstat $MB..HEAD > "$DIR/diff_numstat_before.txt"; git diff --stat $MB..HEAD > "$DIR/diff_stat_before.txt"; git diff --name-status $MB..HEAD > "$DIR/name_status_before.txt"; git log --oneline $MB..HEAD > "$DIR/commits_range_before.txt"; awk '{add=$1; del=$2; if(add=="-"||del=="-"){next} A+=add; D+=del} END{printf "files=%d insertions=%d deletions=%d\n", NR, A, D}' "$DIR/diff_numstat_before.txt" > "$DIR/summary_before.txt"; echo "Saved baseline to $DIR (*_before + summary_before)"`

- Quick baseline summary:
  !`cat "$(cat target/pr-cleanup/LAST_DIR)/summary_before.txt"`

- Tooling hints:
  !`DIR=$(cat target/pr-cleanup/LAST_DIR); find . -maxdepth 3 -type f \( -name Cargo.toml -o -name pyproject.toml -o -name package.json -o -name Makefile -o -name justfile -o -name flake.nix -o -name scripts/ci.sh -o -name scripts/gate.sh \) 2>/dev/null | sed 's@^\./@@' > "$DIR/tooling_hints.txt"; (wc -l "$DIR/tooling_hints.txt" | awk '{print "tooling_hints_count="$1}') && (head -50 "$DIR/tooling_hints.txt")`

## How to work (agents in waves)

### Wave 1 — Explore (find what matters)
Invoke **Explore** to:
- map semantic hotspots and what’s mechanical
- flag interface/contract touchpoints
- flag risk surface deltas (unsafe/concurrency/IO/deps)
- identify repo-native “gate” commands and how to run them locally

Explore should report back with anchors (paths, commands, commit references), not raw diffs.

### Wave 2 — Plan (cleanup plan with maintainability intent)
Invoke **Plan** to:
- propose a cleanup plan that improves maintainability/PR quality without scope creep
- separate “quick wins” vs “follow-ups”
- choose which tooling to run (gate-first, then targeted checks)
- suggest a commit plan if it will materially improve review (mechanical vs semantic)

### Wave 3 — Improve & fix (apply changes in the working tree)
Invoke appropriate fixing agents (general-purpose or specialist) to:
- run the repo’s best available gate (just/make/scripts/xtask/nix) and address findings
- apply safe mechanical fixes (format/lint/docs drift) and straightforward correctness fixes
- tighten boundaries / reduce future change-cost where it’s clearly beneficial (especially in hotspots)
- save tool outputs you will cite into the receipts dir (gate logs, audit outputs, link checks, etc.)

### Wave 4 — Verify & report (prove readiness)
After fixes:
- re-run the relevant gate/checks
- save “after” snapshots + key logs into the receipts dir
- produce a narrative cleanup report with a crisp interface verdict and evidence pointers

## Useful tools (guidance)

Prefer repo-native commands; otherwise use what fits:
- Rust: `cargo fmt`, `cargo clippy`, `cargo test`/`cargo nextest`, `cargo semver-checks`/`cargo-semver-checks`, `cargo-audit`, `cargo-deny`, `cargo-geiger`, `cargo llvm-lines`, `tokei`
- Python: `ruff`, `mypy`/`pyright`, `pytest`, `pip-audit`, `radon`
- JS/TS: `eslint`, `tsc`, `jest`, `npm audit`
- Docs: doctests, `lychee`

Save outputs you cite into the receipts dir.

## Output (cleanup report)

Write the report to:
- `<receipts>/cleanup_report.md`
and also print it.

Include:

### Cleanup summary (narrative)
What you tightened and why (maintainability + reviewability), and what you deliberately didn’t touch.

### Interface & compatibility verdict (crisp)
- Public API: unchanged | additive | breaking | not measured
- Schemas/contracts: unchanged | updated | breaking | not measured
- CLI/config surface: unchanged | changed | not measured
Back each with anchors (paths, commands, or saved tool outputs).

### Evidence & receipts
What you ran and where you saved it under the receipts dir.

### What changed during cleanup
Key files/dirs touched + “before → after” highlights (lint/test/docs/risk surface).

### Remaining concerns / follow-ups
What’s still worth doing and what you’d mechanize next time.

### PR readiness verdict
Ready / not ready + blockers.
If ready, recommend running `/pr-create` next (with suggested context).

Now proceed: explore → plan → apply fixes → verify → report, saving purposeful receipts along the way.
