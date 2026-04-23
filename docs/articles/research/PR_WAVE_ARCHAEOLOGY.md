# PR Wave Archaeology
## How The Repository Moves In Bursts, Not A Smooth Stream

This note uses GitHub PR metadata as the primary source. I fetched the accessible PR ledger slice for this repository from GitHub and looked for clustered creation dates, branch-name families, draft usage, and close/merge behavior.

The shape is not steady throughput. It is bursts with distinct signatures.

In the 1,883 PRs returned by the GitHub API, `codex/*` is the dominant branch family. The rest of the ledger is still heavy on `test/*`, `fix/*`, `feat/*`, and `docs/*`, which is a strong hint that the repo works in batch modes rather than by isolated one-off PRs.

---

## 1. The Ledger Has Clear Burst Days

The largest creation days in the fetched slice are concentrated in March 2026:

- `2026-03-18` is the biggest spike
- `2026-03-04`, `2026-03-12`, `2026-03-16`, and `2026-03-19` are the next major peaks
- `2026-02-28` is a distinct release-campaign day
- `2025-08-26` is the opening codex burst in the visible slice

That is the first important conclusion: the repo does not grow by a smooth daily drip. It moves through high-energy PR waves that are easy to identify from creation timestamps alone.

---

## 2. Each Wave Has A Different Shape

### Early codex burst

The earliest visible day in the slice, `2025-08-26`, is already PR-heavy and already `codex/*`-named. The PRs are mostly direct implementation work:

- parser and lexer changes
- completion and incremental parsing work
- test and symbol-collection updates

That looks like the first large-scale batch mode: many small, concern-scoped PRs in one day.

### Release-campaign burst

`2026-02-28` changes shape. The branches are mostly `release/*` and `wave4/*`, and the titles are release-readiness, packaging, and workflow hardening. This is not feature churn. It is a coordinated release campaign.

### Launch-prep burst

`2026-03-04` is dominated by `codex/prepare-for-*` branches. The PR titles cluster around crates.io launch prep, VS Code Marketplace prep, and supporting runbooks. That is a recognizable batch pattern: many small PRs, one operational goal.

### Stabilization burst

`2026-03-12` mixes version bumps, docs polish, parser fixes, and worktree-scoped branches. It reads like a stabilization wave: release, repair, and documentation all happen in the same traffic pattern.

### Control-plane burst

`2026-03-16` and `2026-03-18` show the current swarm model more clearly. Branch names shift toward `worktree-agent-*`, `chore(swarm)`, `docs(status)`, `refactor(skill)`, and dense test/fix families. The work is no longer just product code; it is also control-plane scaffolding.

### Article burst

`2026-03-19` is the clearest docs/article wave in the current tree. Five PRs were created within minutes, including four article drafts:

- `#2224` `docs: add Five Eras of AI Development article`
- `#2225` `docs: add swarm methodology reference article`
- `#2226` `docs: add zero-panic reliability and security article`
- `#2227` `docs: add Parsing Perl challenges article`
- `#2228` `docs: add development curiosities and records`

That is a classic swarm artifact: the repo emits documentation as a batch, not as an afterthought.

---

## 3. Drafts And Closures Are Part Of The System

The fetched ledger slice contains 237 draft PRs and 686 PRs that were closed without merge. That matters.

The repository is not treating every PR as a success path. Drafts are used as staging, and a substantial amount of work is intentionally disposed of, superseded, or left unmerged when it no longer fits the current path.

That lines up with the current merge discipline documented elsewhere in the archaeology:

- review and readiness are separate from merge
- merge queues are paced
- closure is a valid outcome
- documentation and control-plane PRs are part of the operating system, not side work

---

## 4. How This Fits The Era Story

This PR-wave view matches the larger era story:

- the early repo leans on `codex/*` bursts and direct feature delivery
- the Q3 swarm turns work into a PR-shaped pipeline
- the February 2026 Copilot period becomes a release-and-batch campaign
- the March 2026 Claude-era control plane turns bursts into reusable surfaces, skills, hooks, and state
- the current docs/article wave shows that historical analysis itself is now a batchable repo activity

The important point is not just that the repo has many PRs. It is that the repo repeatedly reorganizes itself into waves with different purposes.

That is a useful archaeological signal because it explains why the codebase feels unusually readable as history: branch names, PR titles, and burst timing all encode the operating mode of the moment.

---

## 5. Evidence Pointers

- [Q3_SWARM_PR_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
- [COPILOT_FLEET_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/COPILOT_FLEET_ARCHAEOLOGY.md)
- [ERA5_MIXED_TOOL_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA5_MIXED_TOOL_ARCHAEOLOGY.md)
- [MERGE_DISCIPLINE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/MERGE_DISCIPLINE_ARCHAEOLOGY.md)
- [SWARM_SURFACE_EVOLUTION.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_SURFACE_EVOLUTION.md)
