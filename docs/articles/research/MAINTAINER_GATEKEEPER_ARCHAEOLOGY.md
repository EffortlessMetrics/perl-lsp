# Maintainer Gatekeeper Archaeology
## How The Human Role Shifted From Writing Code To Steering Trusted Change

This note tracks the maintainer's role in this repository as it moved from direct coding toward architectural direction, acceptance/rejection, merge pacing, and trusted-change oversight.

The recurring pattern across the docs is consistent: the human does not disappear. The human becomes the selector, the architect, and the merge bottleneck that keeps machine-generated work trustworthy.

The maintainer's later clarification adds a second dimension to that role
change: the repo is not only moving from direct coding toward review. It is
also going through repeated waves of imparting maintainer vision into agent
surfaces. Across those waves, the guidance becomes more conceptually sound,
more iterated, and more reusable, even as some later surfaces depend less on
line-by-line human review in the moment and more on gates, state, and archived
lessons.

---

## 1. The Repository Says The Human Role Changed

[`docs/project/AGENTIC_DEV.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md) defines the shift directly:

- AI-assisted means the human writes and the AI suggests
- AI-native means the human reviews, accepts, or rejects
- throughput becomes machine-limited
- quality becomes mechanical
- claims become receipt-based

That is the cleanest statement of the maintainer's new position. The scarce resource is no longer typing. It is judgment.

[`docs/project/AGENTIC_DEVELOPMENT.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEVELOPMENT.md) expands the role further:

- setting architectural direction
- writing and refining agent instructions
- designing CI gates
- reviewing and merging agent-generated PRs
- handling cases where agents get stuck or produce incorrect results

That is not a passive reviewer. It is a gatekeeper with architectural authority.

---

## 2. The Maintainer Became The Selector

[`docs/project/JULES_BOT_ANALYSIS.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/JULES_BOT_ANALYSIS.md) shows the maintainer acting as a selector rather than a direct implementer.

The strongest evidence is the January 2026 bridge pattern:

- 57 `maint/pr-*` bridging PRs curated Jules output into mergeable form
- those bridging PRs had a 92% merge rate
- Jules itself had a much higher rejection rate, especially once it ran out of low-hanging fruit

The important inference is that the maintainer was not merely accepting agent output. The maintainer was curating it:

- keep the good idea
- reject the repeated or misframed attempt
- translate promising output into a mergeable slice

That is a selector's role. The maintainer is shaping what the repo should keep, not just approving what appears.

This matches the later swarm docs too:

- review loops are explicit
- cleanup PRs are normal
- drafts become staging
- triage removes duplicates and stale work

The maintainer is the person who decides which slices deserve to continue.

---

## 3. The Maintainer Stayed The Architect

[`docs/articles/research/ERA_TIMELINE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA_TIMELINE.md) shows that the human role changed by era, not by disappearance.

Era 1 is direct coding with human review.
Era 3 is architectural hardening, with mutation testing, ADRs, and a slower pace that enabled later parallelism.
Era 5 still has a human in the loop, but now the human is supervising short bursts, mixed-tool orchestration, and worktree-isolated agents.

The maintainer's architectural job is visible in the repo's own descriptions:

- set direction for tiering, crate families, and SRP extraction
- define gate budgets and acceptance criteria
- decide when the problem is architectural rather than mechanical
- preserve the repo's quality model while letting agents move fast

That is why the repo can scale at all. The maintainer is not just the last reviewer. The maintainer is the person who keeps the architecture stable enough for agents to operate inside it.

That is also why maintainer intent deserves its own archaeology. The repo is
repeatedly trying to teach agents what "aligned with this codebase" means, not
just asking them to execute isolated tasks faster.

---

## 4. The Maintainer Is Also The Merge Bottleneck

The PR archive makes the bottleneck role explicit.

[`docs/articles/research/PR_LIFECYCLE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_LIFECYCLE_ARCHAEOLOGY.md) shows drafts, merges, and closures becoming lifecycle states. That only works because someone is selecting which state a PR should enter next.

[`docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md) shows review and follow-up as normal work:

- review happens before readiness
- cleanup is a legitimate output
- follow-up PRs are explicit repair, not embarrassment

[`docs/articles/research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md) is the clearest statement of the human bottleneck:

- the merge queue is three wide
- CI throughput is the hard boundary
- batches are paced to avoid cancellation cascades
- excess work overflows into issues rather than queue collapse

That means the maintainer is not only a reviewer. The maintainer is the queue governor. The repo can generate more work than it can safely absorb, and the human decides the pace.

---

## 5. The Role Shift Is Visible In The Tooling

The maintainer's changing job shows up in the control surfaces the repo preserved:

- `review-pr` for focused review
- `pr-ready` for readiness transitions
- `triage-prs` for cleanup and disposal
- `green-merge` for batch merge pacing
- `swarm-state` for queue state, pitfalls, and findings

[`docs/articles/research/SWARM_SURFACE_EVOLUTION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_SURFACE_EVOLUTION.md) and [`docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md) show that these surfaces were not accidental.
They are the repo's answer to the human bottleneck:

- use commands for orchestration
- use skills for repeatable procedure
- use hooks for deterministic enforcement
- use state files for durable memory

That architecture makes the maintainer's role more selective and less mechanical. The human spends less time writing code and more time deciding what should be kept, merged, staged, or discarded.

The tradeoff is visible in the historical arc. Later waves of guidance are
better structured and easier to reuse, but they also sit further from direct
maintainer inspection at the individual-change level. The repo keeps trying to
close that gap with receipts, review lanes, merge gates, and persistent memory.

---

## 6. What The Maintainer Actually Does Now

The repository's own evidence points to a stable operating model:

1. set architecture and boundaries
2. decide which agent output is worth keeping
3. pace merges so CI stays trustworthy
4. reject or close work that is stale, duplicated, or misframed
5. preserve the lessons in state and docs

That is the maintainer role in the AI-native repo. The human is not removed from the loop. The human becomes the part of the loop that preserves judgment, keeps the queue legible, and prevents machine throughput from outrunning trust.

---

## Evidence Pointers

- [`docs/project/AGENTIC_DEVELOPMENT.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEVELOPMENT.md)
- [`docs/project/AGENTIC_DEV.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md)
- [`docs/project/JULES_BOT_ANALYSIS.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/JULES_BOT_ANALYSIS.md)
- [`docs/articles/research/ERA_TIMELINE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA_TIMELINE.md)
- [`docs/articles/research/MAINTAINER_VISION_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/MAINTAINER_VISION_ARCHAEOLOGY.md)
- [`docs/articles/research/PR_LIFECYCLE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_LIFECYCLE_ARCHAEOLOGY.md)
- [`docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md)
- [`docs/articles/research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md)
- [`docs/articles/research/SWARM_SURFACE_EVOLUTION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_SURFACE_EVOLUTION.md)
- [`docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)
