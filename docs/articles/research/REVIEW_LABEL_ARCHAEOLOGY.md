# Review Label Archaeology
## How GitHub Labels Briefly Became A Review State Machine

This note documents a distinct governance phase in the repository: a short,
highly structured period in late Q3 2025 when GitHub labels were used as an
explicit review state machine.

The repo did not just tag pull requests by topic. It encoded review progress,
review effort, gating results, lane assignment, and readiness directly in the
GitHub label set.

That phase did not last long, but it matters historically because it shows the
same methodology later seen in `.claude` commands and skills trying to exist in
the surfaces GitHub already exposed.

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

Those are not repo-wide norms. They are evidence of a concentrated experiment:
for a brief window, the repo made review state highly legible inside GitHub
itself.

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

This is a GitHub-native version of a workflow engine.

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

The later control plane is more durable, but the decomposition impulse is
already visible here.

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

Historically, that matters because it connects three later themes:

- issue-to-draft routing in the Q3 swarm packs
- issue overflow and routing in the current swarm
- PR governance and readiness as distinct control-plane responsibilities

The repo was already trying to make discovery, implementation, and review
traceable through explicit references and explicit state.

---

## 6. The System Was Brief, Not Permanent

One of the most interesting findings is that this label-heavy system was
intense, but short-lived.

The latest observed usages in the full PR snapshot are early:

- latest `review:stage:intake`: `2025-10-04` on `#209`
- latest `flow:review`: `2025-10-02` on `#205`
- latest `merge-ready`: `2025-10-04` on `#209`

That suggests the repo did not scale this exact GitHub-label model across the
entire later history.

Instead, the governance logic appears to migrate:

1. first into structured GitHub labels and lanes
2. later into Q3 flow packs such as `issue-to-draft` and `pr-to-merge`
3. finally into commands, skills, hooks, and `swarm-state`

So the labels are best understood as a bridge technology.

GitHub was being used as the control plane before the repo had a better one.

---

## 7. What This Says About The Repo

This short label burst is historically important because it shows the repo
trying to solve a very modern problem with the tools it had:

- how to make review state visible
- how to separate gates from judgments
- how to express queue ownership
- how to preserve process truth alongside code truth

The later Claude-era control plane did not invent these concerns. It gave them
better surfaces.

The label phase proves the methodology was already there:

- stages matter
- gates matter
- readiness is distinct from authorship
- queue ownership matters
- GitHub metadata can carry operational truth

That is why this period belongs in the archaeology. It is the repo's earliest
clear attempt to make trusted change legible as structured state.

---

## Evidence Pointers

- [MERGE_DISCIPLINE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/MERGE_DISCIPLINE_ARCHAEOLOGY.md)
- [PR_REVIEW_LOOP_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md)
- [CONTROL_PLANE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)
- [SWARM_SURFACE_EVOLUTION.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_SURFACE_EVOLUTION.md)
- full PR ledger snapshot from `gh pr list --state all --limit 2000 --json number,title,createdAt,labels,url`
- representative PRs: `#153`, `#158`, `#159`, `#160`, `#170`, `#174`, `#205`, `#209`
