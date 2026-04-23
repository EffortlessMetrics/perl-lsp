# Review Label Archaeology
## How The Q3 Swarm Used GitHub Labels As A Review State Machine

This note documents one GitHub-facing surface of the canonical Q3 swarm: a
short, highly structured period in late Q3 2025 when GitHub labels were used as
an explicit review state machine.

The repo did not just tag pull requests by topic. It encoded review progress,
review effort, gating results, lane assignment, and readiness directly in the
GitHub label set.

That phase did not last long, but it matters historically because it shows the
Q3 swarm's three-phase methodology being expressed directly through GitHub
metadata before later `.claude` commands, skills, hooks, and `swarm-state`
became the more durable control plane.

All counts and PR examples in this note were verified from the full
`gh pr list --state all --limit 2000` ledger on `2026-03-19`.

---

## 1. A Small But Very Dense Governance Burst

The interesting part of the label-based review system is not scale. It is
density.

Across the full PR archive snapshot, the review-pipeline labels of interest
appear on a relatively small cluster of PRs:

- `review:stage:intake`: `6`
- `review:stage:sweep-initial`: `1`
- `review:stage:sweep-final`: `1`
- `review:stage:freshness`: `2`
- `gate:hygiene`: `2`
- `merge-ready`: `4`
- `flow:review`: `7`
- `flow:integrative`: `7`
- `review-lane-1`: `2`

Those are not repo-wide norms. They are evidence of a concentrated Q3 swarm
expression: for a brief window, the repo made review state highly legible
inside GitHub itself.

The label families cluster around the same ideas:

- what stage the PR is in
- how much review effort it likely needs
- whether key gates have passed
- which review lane owns it
- whether the PR is ready to move forward

That is not ordinary repository tagging. It is process encoding.

---

## 2. September 12, 2025 Is The Earliest Dense Example

The earliest clear example is PR `#153`, created on `2025-09-12`.

Its label stack is unusually rich:

- `review:stage:sweep-initial`
- `review:stage:sweep-final`
- `review:stage:freshness`
- `gate:hygiene`
- `gate:matrix`
- `gate:security (clean)`
- `gate:fuzz (clean)`
- `gate:policy (clear)`
- `merge-ready`
- `fix:hygiene`
- `fix:security`
- `Review effort 4/5`

That stack reveals several important behaviors immediately:

- review is decomposed into phases, not one verdict
- gate outputs are preserved as labels, not just comments
- fixes discovered during review are themselves classified
- the PR can show both process state and technical risk at once

The maintainer's clarification sharpens the interpretation: this is not best
read as a separate GitHub overlay. It is part of the same Q3
`issue-to-draft` / `draft-to-pr` / `pr-to-merge` machinery, exposed through
labels because GitHub was one of the few available coordination surfaces for
managing multiple massive PRs across many repos at the same time.

PR `#160`, created on `2025-09-20`, reinforces the same pattern. It carries
both `gate:policy (blocked)` and `gate:policy (clear)`, plus architecture and
schema alignment labels. That means the label set is being used as an audit
trail, not merely a final badge.

---

## 3. Intake And Review Flows Become First-Class

The next visible step is the intake-and-flow vocabulary.

The earliest PR in the archive with `review:stage:intake` is `#158`, created on
`2025-09-17`:

- `#158` `Complete Substitution Operator Parsing Implementation (#147)`

The earliest PR with `flow:review` is `#159`, also on `2025-09-17`:

- `#159` `feat: Enable missing documentation warnings with comprehensive API docs (Issue #149)`

That matters because the labels are starting to distinguish:

- stage labels such as `review:stage:intake`
- flow labels such as `flow:review`
- gate labels such as `gate:hygiene`
- readiness labels such as `merge-ready`

The repo is separating route, stage, and gate instead of collapsing them into
"open" versus "merged."

This is the same design instinct that later shows up in:

- `/review-pr`
- `/pr-ready`
- `/green-merge`
- `/triage-prs`

The later control plane is more durable, but the same decomposition impulse is
already visible here inside the Q3 swarm itself.

Historically, this also explains why the surface is so label-heavy: the repo
was trying to keep generation, review, and integration lanes stable with
different agents and worktrees, but the tooling was still early and Claude
struggled with the coordination burden. The labels are therefore not only
metadata. They are visible scaffolding for a swarm that had not yet found
better local control-plane primitives.

---

## 4. Review Lanes Show Queue Ownership

By late September 2025, the labels start expressing ownership as well as state.

The earliest `review-lane-1` usage appears on `2025-09-26` in PR `#170`:

- `#170` `feat(lsp): Implement executeCommand method with perl.runCritic command (Issue #145)`

PR `#174` follows on `2025-09-28` with the same lane label:

- `#174` `feat(perl-parser): restore architectural integrity for Issue #146`

Those PRs also carry:

- `review:stage:intake`
- `flow:review`
- `flow:integrative`
- `Review effort 4/5`

That combination is revealing. The repository is not only saying "this PR needs
review." It is saying:

- this is where the PR is in the pipeline
- this is the lane that owns it
- this is roughly how expensive review will be
- this is part of an explicit review and integration flow

That is the same queue-awareness later formalized in `green-merge`,
`swarm-status`, and `swarm-state`, just expressed through GitHub labels rather
than local runtime surfaces.

---

## 5. The Review Labels Are Closely Tied To Issue-Linked Work

Another useful pattern is that many of the labeled PRs are explicitly linked to
issues in their titles:

- `#158` references `#147`
- `#159` references `Issue #149`
- `#170` references `Issue #145`
- `#173` references `Issue #144`
- `#174` references `Issue #146`
- `#205` references `Issue #178`
- `#209` references `#207`

That means the label-based review system was not operating on anonymous diffs.
It was attached to issue-shaped delivery.

Historically, that matters because it connects three aligned themes:

- issue-to-draft routing in the Q3 swarm packs
- draft-to-pr promotion inside the same Q3 flow pack
- issue overflow and routing in the current swarm
- PR governance and readiness as distinct control-plane responsibilities

The repo was already trying to make discovery, implementation, and review
traceable through explicit references and explicit state.

---

## 6. The GitHub-Label Surface Was Brief, Not The Q3 Swarm

One of the most interesting findings is that this label-heavy system was
intense, but short-lived.

The latest observed usages in the full PR snapshot are early:

- latest `review:stage:intake`: `2025-10-04` on `#209`
- latest `flow:review`: `2025-10-02` on `#205`
- latest `merge-ready`: `2025-10-04` on `#209`

That suggests the repo did not scale this exact GitHub-label surface across the
entire later history.

Instead, the governance logic evolves:

1. in Q3, into structured GitHub labels and lanes alongside the three-phase
   `issue-to-draft` / `draft-to-pr` / `pr-to-merge` swarm
2. later, into more durable `.claude` commands, skills, hooks, and
   `swarm-state`

So the labels are best understood as one surface of the canonical Q3 swarm, not
as a separate governance era.

GitHub was one of the Q3 swarm's control surfaces before the repo promoted more
of that logic into dedicated local runtime surfaces.

---

## 7. What This Says About The Repo

This short label burst is historically important because it shows the canonical
Q3 swarm solving a very modern problem with the tools it had:

- how to make review state visible
- how to separate gates from judgments
- how to express queue ownership
- how to preserve process truth alongside code truth

The later Claude-era control plane did not invent these concerns. It gave the
same concerns better surfaces.

The label phase proves the methodology was already there:

- stages matter
- gates matter
- readiness is distinct from authorship
- queue ownership matters
- GitHub metadata can carry operational truth

That is why this period belongs inside the Q3 swarm archaeology. It is one of
the clearest surviving examples of the three-phase swarm making trusted change
legible as structured state.

---

## Evidence Pointers

- [MERGE_DISCIPLINE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/MERGE_DISCIPLINE_ARCHAEOLOGY.md)
- [Q3_SWARM_PR_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
- [PR_REVIEW_LOOP_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md)
- [CONTROL_PLANE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)
- [SWARM_SURFACE_EVOLUTION.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_SURFACE_EVOLUTION.md)
- full PR ledger snapshot from `gh pr list --state all --limit 2000 --json number,title,createdAt,labels,url`
- representative PRs: `#153`, `#158`, `#159`, `#160`, `#170`, `#174`, `#205`, `#209`
