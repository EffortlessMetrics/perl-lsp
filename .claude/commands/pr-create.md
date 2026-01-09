---
description: Create a PR from the current branch (narrative + modern quality signals; agents in waves; purposeful receipts; creates PR)
argument-hint: [optional: context/intent e.g. "Issue #218", "ready", "draft", "base=main", "no-ci", "local-gate-canonical"]
allowed-tools: >
  Bash(git status:*), Bash(git branch:*), Bash(git rev-parse:*), Bash(git symbolic-ref:*), Bash(git remote:*),
  Bash(git merge-base:*), Bash(git log:*), Bash(git show:*), Bash(git diff:*),
  Bash(ls:*), Bash(find:*), Bash(rg:*), Bash(sed:*), Bash(awk:*), Bash(wc:*),
  Bash(mkdir:*), Bash(cat:*), Bash(tee:*), Bash(date:*),
  Bash(gh:*),
  Bash(make:*), Bash(just:*), Bash(nix:*),
  Bash(cargo:*), Bash(cargo-*:*) , Bash(pytest:*), Bash(ruff:*), Bash(mypy:*), Bash(pyright:*),
  Bash(node:*), Bash(npm:*), Bash(pnpm:*), Bash(yarn:*),
  Bash(tokei:*), Bash(scc:*), Bash(lizard:*), Bash(radon:*),
  Bash(lychee:*),
  Bash(pip-audit:*), Bash(pip:*), Bash(uv:*), Bash(poetry:*)
---

# Create PR (current branch)

Create a pull request from the **CURRENT WORKING TREE state** of this branch.

Write the PR like maintainer notes: narrative is welcome. Center it on modern review signals:
- **Interface integrity** (public API / contracts / schemas / CLI/config surface)
- **Risk surface delta** (unsafe, concurrency, IO/networking/serialization, deps)
- **Verification depth** (what evidence exists and how to reproduce it)
- **Future change-cost** (hotspots, modularity, complexity proxies, doc rot prevention)

We can store artifacts when we're purposeful with them: keep a **purposeful receipt bundle you will actually cite or reuse** (diff anatomy, tool outputs you reference, reproduction commands, etc.).

Use any extra context I provide: **$ARGUMENTS**

## Context (auto-collected)

- Branch: !`git branch --show-current`
- Status: !`git status --porcelain=v1 -b`

- Default branch (best effort): !`git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo main`

- Receipts dir (created now):
  !`BR=$(git branch --show-current | tr '/ ' '__'); TS=$(date +"%Y%m%d-%H%M%S"); DIR="target/pr-create/${TS}-${BR}"; mkdir -p "$DIR"; echo "$DIR" | tee target/pr-create/LAST_DIR`

- Baseline snapshot saved (no large diff output printed):
  !`DIR=$(cat target/pr-create/LAST_DIR); BASE=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo main); MB=$(git merge-base HEAD origin/$BASE 2>/dev/null || git merge-base HEAD $BASE); echo "base=$BASE" > "$DIR/base_branch.txt"; echo "$MB" > "$DIR/merge_base_sha.txt"; git diff --name-only $MB..HEAD > "$DIR/files.txt"; git diff --numstat $MB..HEAD > "$DIR/diff_numstat.txt"; git diff --stat $MB..HEAD > "$DIR/diff_stat.txt"; git diff --name-status $MB..HEAD > "$DIR/name_status.txt"; git log --oneline $MB..HEAD > "$DIR/commits_range.txt"; awk '{add=$1; del=$2; if(add=="-"||del=="-"){next} A+=add; D+=del} END{printf "files=%d insertions=%d deletions=%d\n", NR, A, D}' "$DIR/diff_numstat.txt" > "$DIR/summary.txt"; echo "Saved baseline to $DIR (files/diff/commits/name-status + summary)"`

- Quick summary:
  !`cat "$(cat target/pr-create/LAST_DIR)/summary.txt"`

- Tooling hints:
  !`DIR=$(cat target/pr-create/LAST_DIR); find . -maxdepth 3 -type f \( -name Cargo.toml -o -name pyproject.toml -o -name package.json -o -name Makefile -o -name justfile -o -name flake.nix -o -name scripts/ci.sh -o -name scripts/gate.sh \) 2>/dev/null | sed 's@^\./@@' > "$DIR/tooling_hints.txt"; (wc -l "$DIR/tooling_hints.txt" | awk '{print "tooling_hints_count="$1}') && (head -50 "$DIR/tooling_hints.txt")`

## How to work (agents in waves)

### Wave 1 — Explore (map the change)
Invoke the **Explore** subagent to:
- map where behavior changed (review map + semantic hotspots)
- separate mechanical vs semantic changes
- identify interface/contract touchpoints (API/schema/CLI/config)
- flag risk surface deltas (unsafe/concurrency/IO/deps)
- identify the repo’s likely “gate” command(s)

Explore should use git + repo inspection directly (or read the receipts files under the receipts dir) and report back with anchors (paths, commits, commands).

### Wave 2 — Plan (compose the story + evidence plan)
Invoke the **Plan** subagent to:
- propose a coherent PR narrative arc (intent → design → review path → risk/evidence)
- produce a crisp **Interface & compatibility verdict** (and how it is supported)
- recommend which tools are worth running here to support claims (best available, not exhaustive)
- surface key decision points that affect maintainability (boundaries, invariants, compatibility intent)

### Wave 3 — Improve (tighten the PR content)
Invoke specialist subagents (or general-purpose helpers) to refine:
- Diff Scout / Maintainability: review map + future change-cost interpretation (hotspots, modularity, complexity proxies)
- Evidence / Verification: what was actually validated, reproduction path, what remains unverified
- Docs Verifier (if docs touched): drift/executable example issues
- Risk Surface: unsafe/concurrency/IO/deps delta + rollback story
- Complexity Analyst: tool-backed if available; otherwise defensible proxies with interpretation

### Wave 4 — Create the PR (gh)
Once `pr_title.txt` + `pr_body.md` exist, create the PR with `gh pr create`.

Default: create as **draft**, unless `$ARGUMENTS` clearly indicates “ready”.

## Useful tools (guidance)

Use what fits the repo and what supports the claims you plan to make:
- Rust: `cargo fmt`, `cargo clippy`, `cargo test`/`cargo nextest`, `cargo semver-checks`/`cargo-semver-checks`, `cargo-audit`, `cargo-deny`, `cargo-geiger`, `cargo llvm-lines`, `tokei`
- Python: `ruff`, `mypy`/`pyright`, `pytest`, `pip-audit`, `radon`
- JS/TS: `eslint`, `tsc`, `jest`, `npm audit`
- Docs: doctests, link checks (`lychee`)

Save outputs you cite into the receipts dir (e.g., `gate.log`, `clippy.log`, `tests.log`, `audit.log`, `complexity.txt`).

## Deliverables (write these files, then create PR)

1) Write PR title to: `<receipts>/pr_title.txt`
2) Write PR body (Markdown) to: `<receipts>/pr_body.md`
3) Write an index you can reuse to: `<receipts>/index.md` (what you ran, what you saved, key anchors)
4) Create PR:
   - Save the exact command you ran to: `<receipts>/pr_create_cmd.txt`
   - Save the PR URL (or failure details) to: `<receipts>/pr_url.txt`

### PR body format (narrative + modern signals)

Use these sections:

## Summary
1–3 paragraphs: what changed + why, trade-offs, what should be true after merge.

## Interface & compatibility verdict
Crisp top-line statements (supported by tools or concrete deltas):
- Public API: unchanged | additive | breaking | not measured
- Schemas/contracts: unchanged | updated | breaking | not measured
- CLI/config surface: unchanged | changed | not measured

## Design & maintainability notes
Boundaries, modularity, and what changed future change-cost.

## What changed (narrative)
System-level explanation (not a file dump).

## How to review (fast path)
A practical map: key dirs/files + semantic hotspots.

## Evidence & verification
What you ran, what it proves, and how to reproduce.
If something wasn’t run, say so.

## Complexity (future change-cost)
Tool-backed if available; otherwise proxies (hotspots/churn, module splits, API delta, unsafe delta, deps delta).
Interpret implications rather than scoring.

## Risk & rollback
Blast radius, failure modes, rollback/recovery.

## Known limits / follow-ups
Explicit deferrals and next steps.

## Retrospective (earnest)
Surprises, corrections, and what to mechanize next time (new gate/receipt/invariant).

Now: run the wave process, generate the PR title/body, save purposeful receipts, and create the PR with `gh`.
