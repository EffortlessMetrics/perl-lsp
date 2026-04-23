# Worktree Parallelism Archaeology
## How The Repo Moved From Lane Ideas To Deterministic Worktree Execution

This note traces one specific evolution: the repo wanted parallel generation,
review, and integration lanes before the control plane could reliably support
them. Worktrees were the right abstraction early. The platform and methodology
just took time to catch up.

---

## 1. The Q3 Swarm Already Assumed Lane-Based Parallelism

The canonical Q3 swarm used the three-phase flow now described in two naming
schemes:

- `generative` / `review` / `integrative`
- `issue-to-draft` / `draft-to-pr` / `pr-to-merge`

[`.claude/agents4/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/issue-to-draft.md)
is explicit that the Generative Flow runs in `worktree-serial mode`, with one
agent writing at a time. That sounds serial, but the surrounding architecture
was already lane-based: separate generation, review, and integration phases,
each with its own gates, labels, and handoffs.

The important detail is that this was not just about big PRs. It was about
keeping multiple lanes active at once. If review took hours, generation could
keep feeding the next slice while integration handled a different one. The
concept was already worktree-shaped even before the repo had the durable
surfaces to support it well.

---

## 2. Early Claude Had The Right Shape But Too Much Coordination Burden

The Q3 artifacts show why the model was only partly successful at the time.
The flow docs carry a lot of state in labels, check runs, and ledger comments
because GitHub was doing the orchestration work that local runtime primitives
had not yet absorbed.

That same period also shows the maintainer explicitly thinking in terms of
parallel lanes inside one repo. The issue was not lack of intent. It was that
early Claude still struggled with coordination overhead, so the repo had to
lean on highly explicit prompts and GitHub metadata to keep the lanes from
colliding.

That is why worktrees matter historically: they were the right physical
boundary for the workflow, even if the orchestration around them was still
immature.

---

## 3. `maint/pr-*` Was The Bridge Between Agent Drafts And Human Integration

The next stage is the maintainer bridge era.

[docs/project/JULES_BOT_ANALYSIS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/JULES_BOT_ANALYSIS.md)
shows the pattern clearly: in the early Jules phase, the maintainer created
`57` `maint/pr-*` bridging PRs, and those bridged PRs merged at a `92%` rate.
The `maint/pr-*` naming is not cosmetic. It marks curated integration work
that sits between agent-produced material and the final merge.

Across the full PR archive snapshot on `2026-03-19`, there are `62`
`maint/pr-*` branches total. That makes the bridge family large enough to read
as an operating pattern rather than a handful of cleanup exceptions.

This is the intermediate form of worktree parallelism:

- agents draft in their lanes
- the maintainer bridges and normalizes
- the bridge PR becomes the mergeable unit

The bridge era is where the repo proves the lane idea was correct, even before
the current swarm could run it cleanly end to end.

---

## 4. Current Swarm Control Makes Worktrees A Write Boundary

The modern swarm docs turn that old intuition into explicit policy.

[.claude/skills/swarm/SKILL.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/skills/swarm/SKILL.md)
says every PR-shaped code change happens in its own worktree and that a fresh
worker should be spawned whenever the objective, file surface, or verification
loop changes materially.

[.claude/commands/swarm.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm.md)
goes further: builder workers run with `isolation: "worktree"`, reviewer
workers stay one-PR-at-a-time, and the coordinator model explicitly treats the
worktree as the write boundary.

That is the point where the concept stops being a workaround and becomes the
official operating model.

---

## 5. Deterministic `worktree-agent-*` Branches Show The Model Stabilized

The git history shows the worktree model becoming deterministic enough to
scale.

Across the full PR archive snapshot on `2026-03-19`, there are `50`
`worktree-agent-*` branches total. In the March 11 to March 19, 2026 mixed-tool
window alone, `44` merged PRs use that branch family. The model is no longer a
thought experiment at that point. It is a repeatable execution surface.

Merged PRs with `worktree-agent-*` head branches include:

- `worktree-agent-a90d7ded`
- `worktree-agent-a5823b8f`
- `worktree-agent-a638b42e`
- `worktree-agent-ab615443`

Those branch names are not human-curated bridge names like `maint/pr-*`. They
look like generated, repeatable worker outputs tied to isolated worktree
execution.

That matters historically because it shows the repo moving from:

1. lane ideas in Q3
2. curated maintainer bridges in Q4/Q1
3. deterministic worktree-agent execution in the current swarm

The abstraction did not change. The control plane got better at expressing it.

---

## 6. Historical Meaning

The worktree story is not "worktrees were invented late." It is:

- the repo recognized lane-based parallelism early
- the Q3 swarm encoded that idea with explicit phase separation
- the maintainer bridge era proved the lane model worked operationally
- the current swarm finally made worktrees the durable write boundary

That is why the worktree surfaces in this repo feel unusually central. They are
not just a convenience for branch isolation. They are the physical shape of
the swarm's concurrency model.

---

## Evidence Pointers

- [`.claude/agents4/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/issue-to-draft.md)
- [`.claude/skills/swarm/SKILL.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/skills/swarm/SKILL.md)
- [`.claude/commands/swarm.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm.md)
- [docs/project/JULES_BOT_ANALYSIS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/JULES_BOT_ANALYSIS.md)
- [ERA5_MIXED_TOOL_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA5_MIXED_TOOL_ARCHAEOLOGY.md)
- `git log` merged PRs with `worktree-agent-*` head branches: `#1911`, `#1917`, `#1942`, `#1954`
