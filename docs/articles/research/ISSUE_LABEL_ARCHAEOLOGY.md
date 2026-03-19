# Issue Label Archaeology
## How GitHub Issues Became Typed Routing Memory

This note focuses on a narrower question than
[ISSUE_ROUTING_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_ROUTING_ARCHAEOLOGY.md):
not just when issues became overflow memory, but how the label and title
taxonomy itself turned the issue ledger into a swarm control surface.

The important pattern is not a single label. It is the combination of:

- `swarm-discovered` as the catch-and-preserve lane
- `swarm-improve-*` as self-improvement routing lanes
- priority and status labels as release pressure and work state
- title prefixes like `learning:`, `article:`, `friction:`, and `audit:` as a
  second memory taxonomy when labels are too coarse

All counts and examples in this note were verified from the GitHub issue and
label archive on `2026-03-19`.

---

## 1. The Catalog Is A Routing Vocabulary, Not A Generic Backlog

The current label catalog includes a deliberately typed swarm family:

- `swarm-core`
- `swarm-improve-devex`
- `swarm-improve-docs`
- `swarm-improve-infra`
- `swarm-improve-tests`
- `swarm-architectural`
- `swarm-discovered`

It also carries a parallel triage vocabulary:

- `priority:high`, `priority:critical`, `priority-high`
- `P0-critical`, `P1-high`, `P2-medium`, `P3-low`
- `status:blocked`, `status:in-progress`, `status:needs-triage`, `status:ready`
- `area:ci`, `area:lsp`, `area:parser`, `area:dap`, `area:tests`,
  `area:docs`, `area:lexer`, `area:semantic`

That is not a flat backlog vocabulary. It separates at least three different
questions:

- what kind of work this is
- how urgent it is
- how the swarm should route it

---

## 2. `swarm-discovered` Turned Issues Into Overflow Memory At Scale

The biggest signal is `swarm-discovered`.

Verified counts on `2026-03-19`:

- `swarm-discovered`: `189` total
- open `swarm-discovered`: `161`

The first visible wave lands on `2026-03-16`:

- `#1556` graceful shutdown join handling
- `#1557` extraction of a reader-thread helper
- `#1558` outbound writer serialization errors
- `#1582` and `#1583` diagnostics pipeline wiring
- `#1584` dead-code detection
- `#1586` and `#1587` health validation and its test

That label does not mean "important bug." It means "an agent found something
real outside the current slice, and the repo wants to preserve it."

The latest examples in the same label family include:

- `#2195`, `#2196`, `#2197` for article work
- `#2213`, `#2215`, `#2216`, `#2217`, `#2218` for test and hygiene follow-ups

So the label became a durable overflow lane rather than a one-off scout tag.

---

## 3. `swarm-improve-*` Split Self-Improvement Into Explicit Lanes

The smaller label families are just as revealing because they show the repo
turning swarm self-improvement into typed queues instead of dumping it all into
`swarm-discovered`.

Verified counts on `2026-03-19`:

- `swarm-improve-infra`: `18`
- `swarm-improve-devex`: `13`
- `swarm-improve-tests`: `11`
- `swarm-improve-docs`: `0`
- `swarm-architectural`: `0`

The earliest visible examples show the lane split clearly:

- `#1667` `audit(swarm): cycle 2 improvements & protocol gaps`
  `swarm-improve-infra`
- `#2026`, `#2027`, `#2028`, `#2116`, `#2151`
  `swarm-improve-infra`
- `#2030` swarm-bootstrap and `#2031` rust-analyzer worktrees
  `swarm-improve-devex`
- `#2087`, `#2093`, `#2096`, `#2099`, `#2101`
  `swarm-improve-tests`

The asymmetry matters. Some lanes are heavily used, some are empty, and some
are explicitly reserved. That is what a designed routing protocol looks like.

---

## 4. `swarm-architectural` Is A Reserved Escalation Path

One of the most interesting findings is what did not happen.

On `2026-03-19`, `swarm-architectural` exists in the label catalog but has `0`
issues attached to it.

That does not make it dead weight. The committed control-plane docs treat it as
a deliberate escalation path:

- [.claude/commands/swarm-protocol.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm-protocol.md)
  says to use `swarm-architectural` when the work needs a design decision.
- [.claude/skills/swarm-protocol/SKILL.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/skills/swarm-protocol/SKILL.md)
  repeats the same rule.
- [docs/handoff/SWARM_DESIGN.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/handoff/SWARM_DESIGN.md)
  describes `swarm-discovered` and `swarm-architectural` as the two core issue
  lanes.

That makes `swarm-architectural` historically interesting: the repo built the
design-escalation lane before it needed to use it heavily.

---

## 5. Title Prefixes Became A Second Taxonomy

Another easy-to-miss pattern is that some of the most important issue classes
do not live primarily in labels at all. They live in titles.

Representative examples:

- `learning:` issues `#2190`, `#2191`, `#2192`
- `article:` issues `#2193` through `#2197`
- `friction:` issue `#1678`
- `audit(swarm):` issue `#1667`
- `audit:` issue `#1670`

That is a second taxonomy layered on top of the labels:

- labels route the work
- titles preserve what kind of memory artifact the issue is

This is why the repo can use the same issue tracker for bugs, scout leads,
process lessons, and launch-article evidence without collapsing them into one
flat category.

---

## 6. Historical Meaning

The issue tracker evolved from backlog storage into typed routing memory.

By March 2026, the ledger is doing at least four jobs at once:

- preserving discovered work that does not fit the current branch
- routing swarm self-improvement through explicit lanes
- carrying priority, area, and status metadata for release pressure
- storing lessons, articles, friction logs, and audits through title taxonomy

That is a stronger model than a normal issue backlog. It is a reusable routing
and memory vocabulary for the swarm itself.

---

## Evidence Pointers

- [ISSUE_ROUTING_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_ROUTING_ARCHAEOLOGY.md)
- [.claude/commands/swarm.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm.md)
- [.claude/commands/swarm-protocol.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/swarm-protocol.md)
- [.claude/skills/swarm/SKILL.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/skills/swarm/SKILL.md)
- [.claude/skills/swarm-protocol/SKILL.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/skills/swarm-protocol/SKILL.md)
- [docs/handoff/SWARM_DESIGN.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/handoff/SWARM_DESIGN.md)
- GitHub label and issue archive snapshot on `2026-03-19`
