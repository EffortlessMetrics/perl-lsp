# Maintainer Vision Archaeology
## How Maintainer Intent Kept Getting Recast Into Better Agent Surfaces

One of the most distinctive patterns in this repository is that maintainer
vision is not written down once and left alone. It is imparted in waves.

Each wave tries to capture the same judgment more durably:

- what counts as aligned work
- what level of proof is required
- which roles own which stage
- how much the system can trust itself without line-by-line human intervention

The maintainer's clarification sharpens the historical arc: later waves become
more conceptually sound and more iterated, but they can also be less directly
reviewed and tested in the moment because more trust is being moved into gates,
state, and reusable control-plane surfaces.

---

## 1. First Wave: Direct Orchestration

The earliest durable control-plane artifact is
[.claude/ORCHESTRATION_GUIDE.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/ORCHESTRATION_GUIDE.md),
added in `3341bebdb` on `2025-08-28`.

It already contains the core instinct:

- named roles
- staged validation
- merge gates
- post-merge documentation

Nearby commits show the same pattern:

- `3341bebdb` adds the orchestration guide
- `c8b38260d` expands orchestration and GitHub integration

This is the least abstract form of maintainer vision. The repo is still
teaching agents by writing explicit instructions into a relatively direct prompt
surface.

---

## 2. Q3 Turns Vision Into A Process Model

The canonical Q3 swarm in `agents4` is the next major wave.

Its preserved structure is the key clue:

- `generative/` = `issue-to-draft`
- `review/` = `draft-to-pr`
- `integration/` = `pr-to-merge`

That is a major conceptual improvement over direct orchestration. The guidance
is no longer only "do careful work." It is a model of who owns each stage of
delivery.

[CONTROL_PLANE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)
and
[REVIEW_LABEL_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/REVIEW_LABEL_ARCHAEOLOGY.md)
together show that the repo was expressing the same vision in two ways:

- local role packs and flow files
- visible GitHub labels and review lanes

This wave is more conceptually coherent, but still heavy on prompt packs and
staged supervision.

---

## 3. The Jules Lanes Preserve Vision As Domain Memory

The `.jules/` phase is another important transition.

Instead of one general instruction surface, the repo preserves lane-specific
judgment:

- [bolt.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.jules/bolt.md)
  for performance and hot paths
- [sentinel.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.jules/sentinel.md)
  for security and trust boundaries
- [palette.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.jules/palette.md)
  for UX and editor behavior

This is a more durable form of vision transfer:

- repeated judgment becomes lane identity
- lane identity becomes committed memory
- later work can reuse the memory without reconstructing the philosophy from
  scratch

[JULES_BOT_ANALYSIS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/JULES_BOT_ANALYSIS.md)
shows the same pattern from the PR side: the maintainer is curating and
bridging, not just reviewing code line by line.

---

## 4. March 2026 Promotes Vision Into A Control Plane

The March 2026 swarm work is the most mature wave because it stops treating
vision as only prompt text and starts treating it as infrastructure.

Representative commits:

- `9cc2d3b9a` `feat(swarm): continuous swarm infrastructure with agent teams (#1553)`
- `d17b84393` `docs(swarm): codify worktree-first control plane (#1721)`
- `dfdd3be72` `refactor(agents): archive unused agent definitions (#1964)`
- `cb4251735` `docs(swarm): preserve archived agent iterations`

The same judgment is now split across specialized surfaces:

- commands for entrypoints
- skills for reusable procedures
- hooks for deterministic enforcement
- `swarm-state` for durable memory
- archived rosters for historical context

That is more conceptually sound than the earlier waves because the repository is
finally giving its methodology dedicated places to live.

It also changes the review posture. The repo is increasingly trusting:

- gate behavior
- state schemas
- archived pitfalls
- reproducible procedures

rather than re-litigating every instruction file in real time.

---

## 5. The Shift Is From Instruction To Institution

The broad progression is:

1. direct instructions
2. phase models
3. domain lanes
4. reusable procedures plus state

That is why this seam matters historically. The repository is not merely getting
better at prompting agents. It is industrializing maintainer judgment into
surfaces that survive sessions and personnel changes.

The tradeoff is also part of the history:

- earlier waves are more hands-on
- later waves are more reusable
- later waves rely more on trust machinery than on immediate direct inspection

That is not a bug in the story. It is the story.

---

## Evidence Pointers

- [CONTROL_PLANE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)
- [SWARM_SURFACE_EVOLUTION.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_SURFACE_EVOLUTION.md)
- [REVIEW_LABEL_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/REVIEW_LABEL_ARCHAEOLOGY.md)
- [JULES_LANE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/JULES_LANE_ARCHAEOLOGY.md)
- [MAINTAINER_GATEKEEPER_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/MAINTAINER_GATEKEEPER_ARCHAEOLOGY.md)
- [CLAUDE_MD_EVOLUTION.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CLAUDE_MD_EVOLUTION.md)
- [AGENTIC_DEV.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md)
- [AGENTIC_SWARM_ERA.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_SWARM_ERA.md)
- `3341bebdb`, `c8b38260d`, `9cc2d3b9a`, `d17b84393`, `dfdd3be72`, `cb4251735`
