# AI-Native Operating Model Archaeology
## How The Repo Moved From Assisted Coding Toward Receipt-Driven Operations

This note traces a specific transition in the repository's own language:

- **AI-assisted** means a human writes the code and the AI suggests or amplifies.
- **Swarm** means parallel, worktree-isolated agent execution with a human acting as architect, dispatcher, and merge gate.
- **AI-native** means the repo's own operating model is built around mechanical verification, receipts, durable state, and agents as the primary execution layer while humans supervise the system rather than line-edit every change.

Those categories are not interchangeable here. The repository documents all three, and the transition between them is visible in the tracked docs, the `.claude/` control plane, and the PR archive.

---

## 1. The Repo Defines The Difference Itself

[`docs/project/AGENTIC_DEV.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md) gives the sharpest high-level distinction:

- **AI-assisted**: human writes, AI suggests
- **AI-native**: human reviews and accepts or rejects, throughput is machine-limited, quality is mechanical, claims are receipt-based

That same document makes the enforcement model explicit:

- `just ci-gate` is the canonical local gate
- `just status-check` prevents docs drift
- `docs/project/LESSONS.md` records wrongness with evidence, fixes, and prevention
- PRs are expected to include receipts and evidence pointers

In other words, the repo's own definition of AI-native is not "more AI." It is a shift from trust-based claims to receipt-based claims.

[`docs/project/AGENTIC_DEVELOPMENT.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEVELOPMENT.md) expands that model:

- DevLT is the scarce resource
- compute is a lever, not the rival
- claims are bound to catalogs like `features.toml`
- reviews are audits, not just human approval

That is the conceptual base layer for everything that followed.

---

## 2. The Assisted Phase Was Direct, Human-Led, And Trust-Biased

The earlier era reads as AI-assisted in the classic sense: a human-led workflow where the agent is helpful, but the human is still the real operator.

[`docs/articles/research/ERA_TIMELINE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA_TIMELINE.md) describes Era 1 as direct coding with Opus and human review. The key traits are:

- human-paced commits
- manual merge and conflict resolution
- no durable agent runtime surfaces
- direct feature work with architecture still being formed

The repo's own language in [`docs/project/AGENTIC_DEVELOPMENT.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEVELOPMENT.md) fits that phase: AI assists, but the human is still writing, judging, and carrying the operational load.

This is the baseline the later model moved away from.

---

## 3. The Swarm Phase Added Parallelism Without Yet Completing The Control Plane

The swarm era is not the same thing as AI-native. It is the bridge.

[`docs/project/AGENT_SWARM_WORKFLOW.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENT_SWARM_WORKFLOW.md) describes the older `/wave` and `/bulk-pr` workflow:

- isolated git worktrees
- small, single-purpose tasks
- mechanical gates
- disposable attempts
- sequential merging

That workflow already looks more industrial than assisted coding, but it still treats the human as the architect and gatekeeper. The work is parallelized, but the control plane is still mostly procedural.

The historical swarm notes make the same point from the PR and git side:

- [`Q3_SWARM_PR_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md) shows the late-September 2025 shift into a PR-heavy pipeline
- [`MERGE_DISCIPLINE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/MERGE_DISCIPLINE_ARCHAEOLOGY.md) shows review, readiness, merge pacing, and triage becoming separate lanes
- [`PR_REVIEW_LOOP_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md) shows cleanup and follow-up PRs becoming normal, explicit work

The swarm phase is where the repo learns to distribute work. It is not yet the fully self-describing operating model the later `.claude/` surfaces establish.

---

## 4. The Control Plane Made The Model Durable

The real transition to AI-native behavior shows up when the repo starts versioning its own operating method.

[`CONTROL_PLANE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md) and [`SWARM_SURFACE_EVOLUTION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_SURFACE_EVOLUTION.md) show the sequence:

- orchestration guide first
- canonical Q3 role packs in `agents4`
- January command surfaces
- March 15 continuous swarm turn-on
- March 16 skill extraction
- March 17 swarm-state schema and findings ledger
- March 16-19 rationalization and archival of the surfaces

That matters because the repo moved from "use AI in development" to "encode development as reusable surfaces":

- commands for operator entrypoints
- skills for reusable procedures
- hooks for deterministic enforcement
- swarm-state for durable memory

This is the architectural threshold where AI stops being an add-on and becomes part of the operating fabric.

---

## 5. `swarm-state` Is The Clearest Sign Of AI-Native Operation

[`SWARM_STATE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_STATE_ARCHAEOLOGY.md) shows the repo learning to remember itself:

- `discovered-issues.md` for live leads
- `known-pitfalls.md` for repeatable failure lessons
- `completed-slices.md` for dedup and lifecycle bookkeeping
- `findings.json` for durable conclusions
- `findings.schema.json` for machine-readable validation

That is AI-native behavior in the repo's own terms because it turns operational knowledge into committed state.

The important distinction is not that the agents know things. The important distinction is that the repo now expects the system to retain, validate, and reapply those things.

That aligns with [`docs/project/AGENTIC_DEV.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md):

- wrongness is recorded
- reviews are audits
- receipts prove claims
- mechanical checks catch errors

The repository is no longer relying on session memory alone.

---

## 6. The PR Ledger Shows The Operating Model At Scale

The GitHub PR archive makes the same transition visible from a different angle:

- [`PR_BRANCH_NAMING_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_BRANCH_NAMING_ARCHAEOLOGY.md) shows branch names moving from direct `codex/` work to concern lanes, Copilot waves, and deterministic `worktree-agent-*` names
- [`PR_LIFECYCLE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_LIFECYCLE_ARCHAEOLOGY.md) shows drafts, closures, and merge states becoming deliberate lifecycle steps
- [`PR_WAVE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_WAVE_ARCHAEOLOGY.md) shows the repo moving in bursts rather than a smooth stream
- [`PR_SLICE_SIZE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_SLICE_SIZE_ARCHAEOLOGY.md) shows a default preference for small bounded slices, with large umbrella changes reserved for structural work

That pattern is consistent with AI-native operation as defined here:

- the unit of work is bounded
- the proof is attached
- the review loop is explicit
- disposal is acceptable when the slice is obsolete or superseded

This is not just volume. It is an operating discipline.

---

## 7. Era 3 Is The Foundation, Not A Detour

[`ARCHITECTURAL_SIDECHAIN_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ARCHITECTURAL_SIDECHAIN_ARCHAEOLOGY.md) explains why the slowdown mattered:

- parser v3 became possible
- mutation testing and property-based testing became first-class
- CI and governance became receipt-driven
- the January 2026 Jules bridge fit inside that same architectural phase

That slowdown is what made later swarm speed trustworthy. Without it, the repo would have had parallel generation without a stable quality model.

This is the key bridge from assisted development to AI-native operation:

1. assisted coding proves the problem space
2. swarm parallelism proves the repo can scale work
3. architectural hardening proves the repo can trust the output
4. the control plane and receipts prove the process can be repeated

---

## 8. Copilot And Claude Are Different Phases Of The Same Industrialization

[`COPILOT_FLEET_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/COPILOT_FLEET_ARCHAEOLOGY.md) and [`ERA5_MIXED_TOOL_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA5_MIXED_TOOL_ARCHAEOLOGY.md) show the late-stage operating model:

- Copilot CLI produced a firehose of PR-shaped work
- Claude Code short bursts handled cleanup, orchestration, and targeted runs
- Codex waves still appeared in the same window
- the repo's own `.claude` surfaces turned that mixed-tool environment into something tractable

The distinction that matters is the one the repo itself draws:

- Copilot-era work was high-throughput but less controlled
- Claude-era work is more explicit about roles, receipts, hooks, and memory

That is why the later era feels more AI-native, even when the raw volume is lower. The control plane is better encoded.

---

## 9. What "AI-Native" Means Here

If you collapse the repo's evidence into one sentence, it is this:

The project became AI-native when it stopped treating agent output as a batch of suggestions and started treating the development method itself as versioned infrastructure.

The evidence for that is spread across:

- [`AGENTIC_DEV.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md)
- [`AGENTIC_DEVELOPMENT.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEVELOPMENT.md)
- [`AGENT_SWARM_WORKFLOW.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENT_SWARM_WORKFLOW.md)
- [`CONTROL_PLANE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)
- [`SWARM_SURFACE_EVOLUTION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_SURFACE_EVOLUTION.md)
- [`SWARM_STATE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_STATE_ARCHAEOLOGY.md)
- [`PR_LIFECYCLE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_LIFECYCLE_ARCHAEOLOGY.md)
- [`PR_REVIEW_LOOP_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md)
- [`PR_WAVE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_WAVE_ARCHAEOLOGY.md)

The common thread is mechanical trust:

- claims backed by receipts
- wrongness recorded instead of hidden
- work split into bounded slices
- state preserved across sessions
- review and merge treated as explicit lifecycle states

That is the operating model this repository now lives in.

---

## Evidence Pointers

- [`docs/project/AGENTIC_DEV.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md)
- [`docs/project/AGENTIC_DEVELOPMENT.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEVELOPMENT.md)
- [`docs/project/AGENT_SWARM_WORKFLOW.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENT_SWARM_WORKFLOW.md)
- [`docs/project/LESSONS.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/LESSONS.md)
- [`docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)
- [`docs/articles/research/SWARM_SURFACE_EVOLUTION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_SURFACE_EVOLUTION.md)
- [`docs/articles/research/SWARM_STATE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_STATE_ARCHAEOLOGY.md)
- [`docs/articles/research/PR_BRANCH_NAMING_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_BRANCH_NAMING_ARCHAEOLOGY.md)
- [`docs/articles/research/PR_LIFECYCLE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_LIFECYCLE_ARCHAEOLOGY.md)
- [`docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md)
- [`docs/articles/research/PR_SLICE_SIZE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_SLICE_SIZE_ARCHAEOLOGY.md)
- [`docs/articles/research/PR_WAVE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_WAVE_ARCHAEOLOGY.md)
