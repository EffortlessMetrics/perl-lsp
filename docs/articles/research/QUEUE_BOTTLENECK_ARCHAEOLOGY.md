# Queue Bottleneck Archaeology
## How The Repo Learned That Throughput Lives In CI, Not In Agent Count

This note traces a specific operational lesson in the repository: parallel agent work eventually stops being limited by generation and starts being limited by the merge queue, CI throughput, and the amount of state the control plane can keep legible.

The docs say this in several different ways. Read together, they show the same pattern:

- the swarm can deploy many agents
- the merge queue is only three wide
- CI must stay green between merge batches
- when the queue is saturated, the swarm should not keep piling on work
- excess work overflows into issues or queued slices instead of into more chaos

That is the bottleneck story.

---

## 1. The Swarm Is Built Around Queue Awareness

The swarm docs treat queue depth as a first-class signal, not a side effect.

In [`.claude/commands/swarm.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm.md), the lead is told to:

- check CI queue depth before launching builders
- stop launching more work if more than five runs are already in progress
- drain the merge queue with `/green-merge`
- message scouts for more work when the queue is low

That is a deliberate feedback loop. The swarm is not supposed to maximize active work at all times. It is supposed to keep the queue understandable enough that the operator can keep trust and throughput aligned.

The same pattern appears in the control-plane state:

- [`.claude/swarm-state/README.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/README.md) stores `swarm-queue.json` as active overlap tracking
- `completed-slices.md` records dedup and lifecycle status
- `discovered-issues.md` records leads that overflow the current slice
- `known-pitfalls.md` keeps reusable traps

Queue state is therefore not just operational noise. It is part of the swarm's memory.

---

## 2. The Merge Queue Is Explicitly Three Wide

The repository's own archaeology repeats the same hard limit:

- `docs/articles/research/swarm_development_methodology.md` says the merge queue is `3-wide`
- `docs/articles/research/DEVELOPMENT_ARCHAEOLOGY.md` says CI bottleneck is a three-wide merge queue, with optimal coding agents around `9`
- `docs/articles/research/ERA_TIMELINE.md` repeats the same conclusion: `3-wide merge queue means ~9 optimal coding agents`

That is the core bottleneck. The repo can spawn far more parallel work than it can merge at once.

The consequence is visible in the control-plane commands:

- [`.claude/commands/green-merge.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/green-merge.md) merges green PRs sequentially
- it defaults to a `batch-size` of `3`
- after each batch, it pauses and waits for master CI to go green
- this is specifically to prevent merge/cancel cascades where rapid merges cancel each other's CI runs

The merge queue limit is not incidental. It shapes the whole operating model.

---

## 3. CI Throughput Becomes The Real Bottleneck

Once the swarm can produce more than the merge queue can absorb, the limiting factor stops being agent generation and becomes CI.

The historical docs say this plainly:

- `swarm_development_methodology.md` calls out the `3-wide merge queue` as the discovered CI bottleneck
- the same report says around `100` agents can be deployed, but the limit is what the queue and gates can safely absorb
- `ERA_TIMELINE.md` says scaling beyond the queue means either removing Steven as the bottleneck with better CI tooling or treating overflow as intentional

That matters because it changes how the swarm should behave:

1. scouts discover slices
2. builders produce bounded work in worktrees
3. reviewers stage PRs
4. ops merges in small batches
5. CI validates between batches

The system is not trying to keep every worker busy. It is trying to keep the queue from saturating faster than CI can confirm change.

`docs/project/CI_TEST_LANES.md` reinforces that same logic with budget discipline:

- core and LSP tests run by default
- heavy lanes are label-gated
- concurrency cancellation prevents wasted runs
- `paths-ignore` keeps docs-only changes from burning CI

That is how the repo turns CI from a naive floodgate into a managed resource.

---

## 4. The Swarm Adapted By Routing Around Saturation

The swarm did not solve the queue limit by pretending it did not exist. It adapted in several concrete ways.

### Keep the queue legible

`green-merge.md` says merge in batches of `3`, then wait for master CI before continuing. That pacing is the queue discipline.

### Repurpose idle capacity

`swarm_development_methodology.md` says idle agents can be repurposed via `SendMessage` instead of spawning new ones when the roster is already full.

### Overflow to issues when necessary

`DEVELOPMENT_ARCHAEOLOGY.md` and `ERA_TIMELINE.md` both say the team roster ceiling is around `75`, and excess work overflows to the GitHub issue queue.

That is a pressure-release valve. Work that cannot be safely merged or staffed immediately is still preserved, but it is not forced through the merge queue.

### Keep discovery separate from merge pressure

The swarm docs treat scouts, builders, reviewers, and ops as separate concerns. That separation matters because discovery can continue even when merge throughput is constrained.

This is the same architecture described elsewhere in the repo:

- discovery stays broad
- implementation stays bounded
- merge stays paced
- state survives across sessions

That keeps queue pressure from collapsing the whole system.

---

## 5. Queue State Became Durable Memory

The strongest sign of maturation is that queue pressure itself got committed to state.

In [`.claude/swarm-state/README.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/README.md), the repo says:

- `swarm-queue.json` is active overlap tracking
- `completed-slices.md` is lifecycle bookkeeping
- `discovered-issues.md` captures overflow leads
- `findings.json` holds durable control-plane conclusions

That means the repo is not just reacting to queue pressure in real time. It is remembering how the pressure behaved so the next session can route better.

The queue itself is becoming an artifact.

---

## 6. What The Bottleneck Means For The Repo

The architecture lesson is straightforward:

- agent count can scale quickly
- merge queue width cannot
- CI latency is the hard boundary
- issue overflow is preferable to queue collapse
- batch pacing is what keeps the system trustworthy

That is why the repo's swarm is disciplined instead of simply maximal.

The point is not to keep adding more workers until everything is busy.
The point is to keep the whole system moving without outrunning validation.

---

## Evidence Pointers

- [docs/articles/research/swarm_development_methodology.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/swarm_development_methodology.md)
- [docs/articles/research/DEVELOPMENT_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/DEVELOPMENT_ARCHAEOLOGY.md)
- [docs/articles/research/ERA_TIMELINE.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA_TIMELINE.md)
- [`.claude/commands/swarm.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm.md)
- [`.claude/commands/green-merge.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/green-merge.md)
- [`.claude/swarm-state/README.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/README.md)
- [docs/project/CI_TEST_LANES.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CI_TEST_LANES.md)
