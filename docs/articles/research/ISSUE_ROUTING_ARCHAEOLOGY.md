# Issue Routing Archaeology
## How The Issue Tracker Became Part Of The Swarm Control Plane

This note traces a specific change in the repository's operating model: the issue
tracker stopped being just a backlog and became an active routing fabric for
swarm discovery, prioritization, and overflow handling.

That evolution matters because the repo's later swarm does not only use pull
requests as the unit of work. It also uses issues to preserve discoveries the
current merge queue, review budget, or active worktree roster cannot safely
absorb yet.

The shift is visible in three stages:

- early issues carry priority and release-pressure signals
- the March 2026 swarm adds discovery-specific labels and overflow conventions
- the issue queue becomes a typed handoff surface for builders, reviewers, and future sessions

All issue-archive counts in this note were verified from GitHub on `2026-03-19`.

---

## 1. Priority And Triage Existed Before The Current Swarm

The issue tracker already carried operational meaning well before the current
Claude swarm.

Early `priority:high` issues appear in September 2025, including examples such
as:

- `#73` LSP test infrastructure resource constraints
- `#135` xtask compilation errors blocking end-to-end test execution
- `#143` production `unwrap()` usage as a stability and security problem
- `#147` incomplete substitution operator parsing
- `#210` formalizing merge-blocking gates, receipts, and check-run lifecycle

Those earlier issues already show the repo using issues as more than generic
feature requests. They encode:

- release pressure
- test and parser blockers
- trust and gating work
- architectural cleanup that must happen before broader acceleration

The label taxonomy also gets richer over time. By late Q3 and Q4, issues use
combinations such as:

- `priority:high`
- `parser` and `lsp`
- `phase:stability`
- `area:*`
- `P1-high`
- `v0.9-blocker`

That is the pre-swarm foundation: the tracker already knows how to describe
importance, domain, and release impact.

---

## 2. March 16, 2026 Turns Issues Into A Discovery Bus

The sharp inflection happens on `2026-03-16`.

The first `swarm-discovered` issues appear within minutes:

- `#1556` graceful shutdown fix for outbound writer join handling
- `#1557` duplication extraction in `main.rs`
- `#1558` outbound writer serialization error logging
- `#1582` and `#1583` diagnostics pipeline wiring
- `#1584` dead-code detection work
- `#1586` and `#1587` first-run health validation and its test

This matters because the label does not mean "important bug." It means "an
agent found this while working on something else, and the repo wants to preserve
it without forcing it into the current slice."

As of the `2026-03-19` snapshot, the issue archive contains `189`
`swarm-discovered` issues.

That is a control-plane shift:

- every agent is allowed to notice adjacent work
- not every discovery becomes an immediate branch
- the issue queue preserves leads without losing them to chat

The corresponding local state contract says the same thing in tracked files:

- `.claude/swarm-state/discovered-issues.md` says every agent is a passive scout
- `.claude/swarm-state/README.md` defines discovered issues as durable coordination knowledge

The issue tracker is now acting as persistent scout memory at repository scale.

---

## 3. Labels Become Routing Lanes, Not Just Tags

Once `swarm-discovered` exists, the tracker starts behaving like a typed queue.

The current label set includes swarm-specific routing families:

- `swarm-discovered`
- `swarm-improve-devex`
- `swarm-improve-infra`
- `swarm-improve-tests`
- `swarm-architectural`

Verified issue counts on `2026-03-19`:

- `swarm-discovered`: `189`
- `swarm-improve-infra`: `18`
- `swarm-improve-devex`: `13`
- `swarm-improve-tests`: `11`
- `swarm-improve-docs`: `0`
- `swarm-architectural`: `0`

That pattern is interesting for two reasons.

First, the taxonomy is narrower than the generic GitHub defaults. It is
explicitly built around swarm routing and self-improvement.

Second, the label family is ahead of usage. Some lanes exist before they are
heavily populated. That means the repo is designing a routing protocol, not just
reacting ad hoc to whatever happened first.

The first visible waves show the taxonomy in action:

- `#1667` records cycle-2 swarm protocol gaps under `swarm-improve-infra`
- `#2030` and `#2031` open the `swarm-improve-devex` lane for bootstrap and worktree ergonomics
- `#2087`, `#2093`, `#2096`, and `#2099` route test-system work into `swarm-improve-tests`
- `#2151` through `#2162` use `swarm-improve-infra` for skill, hook, template, and metrics gaps inside the swarm itself

At that point, labels are no longer descriptive decoration. They are work
classification.

---

## 4. The Queue Preserves What The Merge Path Cannot Absorb Yet

The issue archive also shows the queue acting as a pressure-release valve.

On `2026-03-16`, the swarm emits parser investigations and release-adjacent
items such as:

- `#1648` subcategorize `unexpected_token_in_expr`
- `#1649` improve delimiter recovery
- `#1650` corpus fetch and sweep pipeline
- `#1652` first-run devex polish
- `#1654` performance baseline work
- `#1655` release gate checklist

On `2026-03-19`, the queue expands further:

- `#2116` encodes cycle-5 learnings into swarm skills and templates
- `#2193` through `#2197` route launch-article work as discovered documentation slices
- `#2213`, `#2215`, `#2216`, `#2217`, and `#2218` turn audit findings into bounded test and hygiene tasks

This is not random issue creation. It is the system preserving discovered work
that should exist, even when:

- the current merge queue is full
- a finding is real but not part of the active slice
- a scout found enough evidence for a builder-ready task
- the work belongs in a later batch or a different lane

That is why the issue queue belongs in the archaeology of the swarm. It is how
the repo keeps discovery ahead of implementation without collapsing into
untracked sprawl.

---

## 5. Local Files Explain How Discovery Becomes A Buildable Slice

The tracked control-plane files connect the GitHub issue archive to the current
swarm process.

`.claude/swarm-state/discovered-issues.md` is the local spillover ledger. It
defines a structured append-only shape for out-of-scope discoveries:

- category
- severity
- description
- context
- files
- suggested branch

`.claude/skills/plan-fix/SKILL.md` then explains how a discovery graduates into
a builder-ready handoff:

- confirm the work is not already covered
- keep the slice bounded
- define one verification loop
- specify exact file surface and receipt expectations

Read together, those files show the full routing chain:

1. an agent notices something outside the current slice
2. the discovery is preserved locally or in the issue queue
3. the issue receives typed routing labels
4. a planner turns it into a builder-ready packet
5. a later worker can execute it without rediscovering the problem

That is a much more structured system than a normal issue backlog.

---

## 6. What This Means Historically

The issue tracker evolved across three roles:

1. classic backlog and blocker tracking
2. priority and release coordination surface
3. swarm overflow queue and typed discovery router

That last role is the distinctive one.

By March 2026, the repository is using issues to do four different jobs at once:

- preserve overflow
- route future work
- record self-improvement opportunities in the swarm itself
- keep discoveries durable across sessions and tool changes

In other words, the issue tracker became part of the control plane.

It is not just where problems are reported.
It is where parallel discovery gets stored until trusted change can catch up.

---

## Evidence Pointers

- [`.claude/swarm-state/README.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/README.md)
- [`.claude/swarm-state/discovered-issues.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/discovered-issues.md)
- [`.claude/skills/plan-fix/SKILL.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/skills/plan-fix/SKILL.md)
- [QUEUE_BOTTLENECK_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md)
- GitHub issue archive snapshot on `2026-03-19`
- Example issues: `#73`, `#143`, `#210`, `#1556`, `#1667`, `#2030`, `#2116`, `#2193`, `#2218`
