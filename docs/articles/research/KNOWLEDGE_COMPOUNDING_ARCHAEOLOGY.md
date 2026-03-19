# Knowledge Compounding Archaeology
## How The Current Swarm Turns State, Logs, And Reports Into Reusable Knowledge

The current swarm does not compound knowledge through one file.

It compounds knowledge through a layered memory stack with different jobs:

- live overlap state
- dedup and lifecycle memory
- opportunistic discovery memory
- reusable pitfall memory
- schema-backed control-plane findings
- operator-facing status and report surfaces
- dated scout logs preserved after their findings are absorbed elsewhere

That is what makes the current swarm feel more cumulative than the earlier
prompt-pack eras.

---

## 1. The Base Layer Appears On March 15, 2026

The current memory/logging layer becomes explicit with commit `9cc2d3b9a` on
`2026-03-15`:

`feat(swarm): continuous swarm infrastructure with agent teams (#1553)`

That commit introduces the tracked `swarm-state` files:

- `completed-slices.md`
- `discovered-issues.md`
- `known-pitfalls.md`
- `swarm-queue.json`

Its own commit message is unusually clear about the intent:

- tracked `.claude/swarm-state/` is committed and survives across sessions
- ephemeral `.ops-perl-lsp/` is runtime state for the current session only

That is a foundational distinction. The repo is deciding which knowledge should
survive the session and which should not.

---

## 2. `swarm-state` Splits Memory By Job

[`.claude/swarm-state/README.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/README.md)
defines the memory classes directly.

Each file has a distinct job:

- [swarm-queue.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/swarm-queue.json)
  — machine-facing overlap and ownership tracking
- [completed-slices.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/completed-slices.md)
  — dedup and lifecycle ledger
- [discovered-issues.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/discovered-issues.md)
  — out-of-scope leads noticed during other work
- [known-pitfalls.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/known-pitfalls.md)
  — append-only reusable failure lessons
- [findings.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/findings.json)
  — durable control-plane conclusions

That is already more structured than a generic "memory" folder.

It means the swarm is not only remembering. It is classifying what kind of
memory something is before it stores it.

---

## 3. `findings.json` Is The Highest-Rigidity Memory Layer

The most distinctive memory surface is `findings.json`.

Commit `d9aab31bc` on `2026-03-17` adds:

- `findings.json`
- `findings.schema.json`
- the current `swarm-state/README.md`

That creates a stricter conclusion layer:

- durable IDs like `SWARM-FINDING-0001`
- typed finding kinds
- statuses such as `active`, `landed`, `superseded`
- required evidence pointers
- follow-up guidance

[`.claude/swarm-state/findings.schema.json`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/findings.schema.json)
shows that this is not free-form notes. It is machine-validated operational
memory.

That matters because it turns swarm learning into a queryable, auditable,
tool-friendly surface rather than leaving it in prose alone.

---

## 4. The Commands Read And Surface The Memory

The memory files are not passive archive material. The commands actively use
them.

[`.claude/commands/swarm-status.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm-status.md)
surfaces:

- in-progress slices from `completed-slices.md`
- discovery counts from `discovered-issues.md`
- tracked findings from `findings.json`

[`.claude/commands/swarm.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm.md)
tells scouts and builders to read:

- `discovered-issues.md`
- `completed-slices.md`
- `known-pitfalls.md`

before they spawn new work.

[`.claude/commands/swarm-wind-down.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm-wind-down.md)
then treats those same files as the preserved state needed for the next session.

So the memory layer is not just recorded. It is operationally consumed.

---

## 5. Skills And Commands Make The Knowledge Portable

The current swarm surface does not keep the memory model hidden in one command.

[`.claude/skills/swarm/SKILL.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/skills/swarm/SKILL.md)
instructs coordinators to read the state files for dedup and trap avoidance.

The skill also defines boundary types:

- worktree boundary
- worker context boundary
- skill as durable procedure boundary
- hook as deterministic control boundary

That matters because the repo is not only preserving facts. It is preserving
how agents are supposed to use those facts.

The memory model therefore compounds at two levels:

- stored knowledge
- stored procedure for consulting that knowledge

---

## 6. Scout Logs Add A New Memory Class

On `2026-03-19`, commit `344c6a591` adds tracked scout logs under
`.claude/logs/scouts/`.

The first preserved examples are:

- [2026-03-19-v0.12.0-readiness.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/logs/scouts/2026-03-19-v0.12.0-readiness.md)
- [2026-03-19-install-experience.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/logs/scouts/2026-03-19-install-experience.md)

These files are different from `swarm-state`.

They are not:

- live overlap state
- append-only pitfall ledgers
- durable control-plane findings

They are dated scout artifacts: preserved point-in-time research reports whose
useful conclusions were later absorbed into the historical docs.

Both files say so directly in their final note.

That creates a new memory class in the repo:

- not control-plane state
- not session-only scratch
- preserved research substrate for archaeology and future synthesis

---

## 7. The Memory Stack Is Hierarchical

Taken together, the current swarm compounds knowledge through a hierarchy:

1. `swarm-queue.json`
   - what is active right now
2. `completed-slices.md`
   - what already exists or is in flight
3. `discovered-issues.md`
   - what was noticed and should be picked up later
4. `known-pitfalls.md`
   - what should not be repeated
5. `findings.json`
   - what durable control-plane conclusion the repo now believes
6. `swarm-status` / `swarm-report` / `swarm-wind-down`
   - how operators and future sessions read and summarize the state
7. `logs/scouts/*.md`
   - dated research reports kept as reusable source material after synthesis

That is more than persistence. It is structured knowledge compounding.

---

## 8. Why This Matters Historically

Earlier eras already had methodology, lanes, and receipts.

The current layer is different because it explicitly separates:

- runtime versus durable state
- product leads versus control-plane findings
- reusable pitfalls versus dated research artifacts
- machine-facing ledgers versus human-facing summaries

That is why the current swarm feels more self-improving than earlier control
surfaces. It is not only doing work. It is deciding what kind of learned thing
each output is, and storing it in the right layer for reuse.

---

## Evidence Pointers

- [`.claude/swarm-state/README.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/README.md)
- [`.claude/swarm-state/swarm-queue.json`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/swarm-queue.json)
- [`.claude/swarm-state/completed-slices.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/completed-slices.md)
- [`.claude/swarm-state/discovered-issues.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/discovered-issues.md)
- [`.claude/swarm-state/known-pitfalls.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/known-pitfalls.md)
- [`.claude/swarm-state/findings.json`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/findings.json)
- [`.claude/swarm-state/findings.schema.json`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/findings.schema.json)
- [`.claude/commands/swarm.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm.md)
- [`.claude/commands/swarm-status.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm-status.md)
- [`.claude/commands/swarm-report.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm-report.md)
- [`.claude/commands/swarm-wind-down.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm-wind-down.md)
- [`.claude/skills/swarm/SKILL.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/skills/swarm/SKILL.md)
- [`.claude/logs/scouts/2026-03-19-v0.12.0-readiness.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/logs/scouts/2026-03-19-v0.12.0-readiness.md)
- [`.claude/logs/scouts/2026-03-19-install-experience.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/logs/scouts/2026-03-19-install-experience.md)
- commits `9cc2d3b9a`, `d9aab31bc`, `344c6a591`
