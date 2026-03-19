# Direct Delivery Archaeology
## How The Repo Moved From Direct Coding Into PR-Shaped Swarm Work

Before the Q3 swarm becomes obvious, the repository still reads as a direct
delivery codebase: feature work, fixups, docs, and cleanup all land in a mostly
linear stream. The important transition is not "when did PRs exist?" PRs existed
early. The transition is when PRs stop being incidental packaging and start
becoming the main delivery shape.

This note focuses on that handoff.

---

## 1. The Early History Still Reads As Direct Delivery

The August 2025 history is busy, but it is still mostly direct in tone.

Representative commit subjects from the August/early September slice:

- `feat(parser): auto-detect test expectations`
- `Make C benchmark harness executable`
- `feat: enhance parser with improved regex/substitution support and add concurrency-capped test commands`
- `fix: improve substitution parsing and code quality for PR #42 cleanup`
- `docs: finalize PR #68 documentation updates following Diataxis framework`

That is not yet a swarm-shaped stream. It is product work with PR references
attached, plus cleanup after the fact.

The first-parent history for this period still looks like a normal development
graph rather than a control plane:

- `2025-08-26`: direct feature and agent work begins to stack up
- `2025-09-04` to `2025-09-11`: merge imports and incremental feature work
- `2025-09-19`: `Sync master improvements: Agent refactoring and customization features (#153)`

The branch vocabulary also fits the same picture:

- `codex/*` appears early and stays prominent
- there is not yet a strong `review-pr*` / `maint/pr-*` / `sync/master-commits-*` pattern
- the repo is still mostly naming work by feature or implementation intent

Monthly PR archive counts reinforce that this is still a pre-staging phase:

- `2025-08`: 66 PRs, 0 drafts, 47 merged, 19 closed
- `2025-09`: 78 PRs, 0 drafts, 65 merged, 13 closed

That is active delivery, but not yet a draft-heavy lifecycle machine.

---

## 2. The Transition Window Is Mid-To-Late September

The shift becomes visible when the commit stream starts encoding stage,
validation, and integration as first-class work.

The first-parent history around `2025-09-22` through `2025-09-30` shows the
change clearly:

- `2025-09-22`: `SPEC-149 governance and policy compliance documentation (#161)`
- `2025-09-22`: `Missing Documentation Warnings Infrastructure + Comprehensive Parser Robustness Improvements (#160)`
- `2025-09-24`: `Enable missing documentation warnings with comprehensive API docs (#159)`
- `2025-09-24`: `Merge branch 'master' of github.com:EffortlessMetrics/tree-sitter-perl-rs into review-pr159`
- `2025-09-25`: `Enhanced LSP cancellation system for Issue #48 (#165)`
- `2025-09-27`: `Implement executeCommand method with perl.runCritic command (Issue #145) (#170)`
- `2025-09-27`: `feat: Add comprehensive agents for Perl LSP validation pipeline`
- `2025-09-28`: `feat: Comprehensive ignored test resolution with enhanced LSP error handling for Issue #144 (#173)`

These subjects are no longer just "implement feature X." They are:

- issue-numbered
- review-feedback-aware
- validation-heavy
- cleanup-heavy
- explicitly staged through PR identity

The branch and merge surface changes with it:

- `review-pr159` becomes a visible integration lane
- `sync/master-commits-*` appears as a merge-import path
- `mantle/integ/*` tags show staged validation language
- `codex/*` stays present, but the repo is no longer only `codex/*`

That is the seam where the repository stops looking like direct coding and starts
looking like a PR conveyor.

---

## 3. Why The Repo Stopped Looking Direct

The repo stopped looking direct for three reasons that reinforce each other.

First, the subject lines began to encode review and repair explicitly. The work
was no longer just "ship a feature"; it was "address review feedback," "hygiene
cleanup," "governance validation," and "pre-merge validation."

Second, the branch vocabulary started separating stages:

- generation and implementation still live in `codex/*`
- review surfaces appear as `review-pr*`
- integration surfaces appear as `mantle/integ/*`
- sync imports appear as `sync/master-commits-*`

Third, the PR archive itself shifts from incidental PRs to lifecycle use. The
archive later shows that drafts and explicit staging become normal, but the
transition begins here with review-linked PRs and merge-import branches.

This is the key distinction:

- direct delivery = produce code and clean it up
- PR-shaped delivery = stage, review, validate, integrate, then merge or close

The repo crosses that boundary in the late-September window.

---

## 4. The Q3 Swarm Is The Result, Not The Beginning

The Q3 swarm is not a sudden invention. It is the formalization of work that was
already moving toward staged delivery.

The canonical Q3 control-plane artifacts preserve the final shape:

- [`.claude/agents4/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/issue-to-draft.md)
- [`.claude/agents4/draft-to-pr.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/draft-to-pr.md)
- [`.claude/agents4/pr-to-merge.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/pr-to-merge.md)

Those flow files name the process after the transition has already happened.
The earlier history explains why they were needed:

- direct delivery produced too much cleanup and too many follow-up repair commits
- review and integration had to become visible surfaces
- the repo needed a way to route work through drafts and PRs instead of treating
  every change like a one-off landing

So the archive tells a simple story:

1. direct delivery dominates the early history
2. mid-to-late September turns stage/review/integration into the delivery model
3. the Q3 swarm codifies that model into `issue-to-draft`, `draft-to-pr`, and
   `pr-to-merge`

---

## 5. Evidence Pointers

- `git log --all --since='2025-08-01' --until='2025-09-30'`
- `git log --first-parent --since='2025-08-01' --until='2025-09-30'`
- `gh pr list --state all --limit 2000`
- [Q3_SWARM_PR_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
- [PR_LIFECYCLE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_LIFECYCLE_ARCHAEOLOGY.md)
- [ERA_TIMELINE.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA_TIMELINE.md)
