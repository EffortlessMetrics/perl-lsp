# Mode Shift Archaeology
## How The Repo Moved From Assisted To Native To Industrialized Work

This note uses the maintainer's mode framework, but grounds it in the repo's
own evidence rather than abstract theory.

The useful distinction here is not "more AI" versus "less AI." It is who
writes, who reviews, and what limits the system. In this repo, the limit shifts
from human attention to machine throughput to full swarm-scale persistence.

---

## 1. Assisted Mode Was Real, But It Was Not The Whole Story

The assisted phase is visible in the earliest era docs.

[`ERA_TIMELINE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA_TIMELINE.md)
describes Era 1 as direct coding with Opus and human review. That looks like
classic assisted development:

- the human is still the main writer and operator
- the agent is helpful but not yet the operating system
- merges and conflict resolution still depend on direct human labor

The repo's own project docs define the same baseline. In
[`docs/project/AGENTIC_DEV.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md),
AI-assisted means the human writes and the AI suggests. That is the starting
point the later modes move away from.

---

## 2. Q4/Q1 Was Already AI-Native, Even If It Was Hands-On

The important correction is that the late-2025 to early-2026 bridge was not
simply "assisted with better tooling."

[`Q4_Q1_HANDS_ON_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q4_Q1_HANDS_ON_ARCHAEOLOGY.md)
shows a PR-heavy, stable phase where the maintainer still carried the
integration burden, but the work itself was already machine-shaped:

- `195` merged PRs in the `2025-12-28` to `2026-02-25` window
- `165` authored by `EffortlessSteven`
- `16` merged from `app/google-labs-jules`
- `57` `maint/pr-*` bridge PRs in the broader Q4/Q1 bridge window
- median merge latency of `1.09` hours

The raw mode is the key point:

- AI was already writing substantial slices
- review was increasingly machine-mediated
- the maintainer was still the bottleneck for integration

That is why this era is best understood as AI-native but hands-on, not as
assisted.

---

## 3. Native Mode Is Where The Repo Starts Treating Receipts And State As The
Operating Model

The transition to native mode shows up when the repo starts versioning its own
control plane.

[`AI_NATIVE_OPERATING_MODEL_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/AI_NATIVE_OPERATING_MODEL_ARCHAEOLOGY.md)
ties that shift to:

- receipt-based claims instead of prose confidence
- `just ci-gate` and `just status-check` as mechanical enforcement
- `swarm-state` as committed memory
- `commands`, `skills`, and `hooks` as reusable operating surfaces

This is the real native threshold. The repo is no longer merely using AI to
help write code. It is using AI inside a control plane that expects durable
state, receipts, and explicit lifecycle transitions.

The Q3 swarm is the bridge into that world:

- [`Q3_SWARM_PR_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
  shows the `issue-to-draft` / `draft-to-pr` / `pr-to-merge` pipeline
- [`CONTROL_PLANE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)
  shows the move from prompt packs to commands, skills, hooks, and swarm-state
- [`SWARM_STATE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_STATE_ARCHAEOLOGY.md)
  shows the repo learning to remember itself across sessions

That is native mode in practice: AI writes, AI reviews, humans supervise the
system, and the repo records the result as machine-readable state.

---

## 4. Industrialized Mode Is Persistent And Self-Reinforcing

Industrialized mode is what happens when the swarm stops being a burst and
starts behaving like maintenance infrastructure.

[`PR_WAVE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_WAVE_ARCHAEOLOGY.md)
and [`QUEUE_BOTTLENECK_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md)
show the structural signs:

- PRs arrive in waves rather than a smooth stream
- the merge queue becomes a throughput constraint
- discovery spills into issues instead of staying in chat
- review and integration are treated as separate lanes

By March 2026, the repo is no longer just shipping features in bursts. It is
running ongoing discovery, self-improvement, trust governance, and archival
memory work in parallel.

That is the industrialized end of the framework: AI writes and AI reviews at
scale, the human selects and arbitrates, and the system keeps learning between
sessions.

The maintainer's clarification adds one more useful nuance inside the March
2026 mixed-tool window itself: not every active surface sat at exactly the same
mode. The Codex CLI wave was quasi-industrialized: AI was reviewing AI, but the
loop still needed more hands-on prodding than it ideally should have. The short
Claude Code swarm runs were fully industrialized even though the interface
still allowed steering. The steering is not what made them industrialized; it
is simply notable that the interface remained steering-enabled rather than
fully async while the underlying operating mode was already industrial.

---

## 5. Why The Same Underlying Mode Looked Different Across Eras

The maintainer's correction matters because the same AI-native mode can look
very different depending on platform constraints.

Q4/Q1 was hands-on because the repo had not yet externalized the control plane
enough to offload integration cost. Q3 was more explicitly staged through
review/generation/integration lanes. The later Claude Code swarm was more
durable because commands, skills, hooks, and swarm-state turned that method
into reusable infrastructure.

So the history is not:

1. assisted
2. native
3. industrialized

It is:

1. assisted coding with human-led review
2. AI-native work that is still manually integrated
3. AI-native control planes that can persist and scale
4. industrialized swarm operation with durable memory and explicit gates

That is a distinction worth preserving because it explains why the same repo
can feel both extremely hands-on and clearly AI-native at the same time.

---

## Evidence Pointers

- [ERA_TIMELINE.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA_TIMELINE.md)
- [Q4_Q1_HANDS_ON_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q4_Q1_HANDS_ON_ARCHAEOLOGY.md)
- [AI_NATIVE_OPERATING_MODEL_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/AI_NATIVE_OPERATING_MODEL_ARCHAEOLOGY.md)
- [Q3_SWARM_PR_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
- [CONTROL_PLANE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)
- [SWARM_STATE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_STATE_ARCHAEOLOGY.md)
- [PR_WAVE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_WAVE_ARCHAEOLOGY.md)
- [QUEUE_BOTTLENECK_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md)
