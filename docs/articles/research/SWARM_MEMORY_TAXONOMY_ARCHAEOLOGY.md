# Swarm Memory Taxonomy Archaeology
## How The Repo Turned Coordination Notes Into Durable Memory

The swarm did not start with a single memory system. It accreted one.

The first step was to make runtime coordination visible and committed. The next
step was to split that coordination into different memory classes. The final
step was to let GitHub issues recover lessons that did not belong in the live
swarm state but still had to survive across sessions.

This note traces that progression through `.claude/swarm-state/` and the issue
title taxonomies the repo uses for learning, articles, friction, and audits.

---

## 1. March 15 Made Swarm State A Committed Surface

The first durable swarm state lands in `9cc2d3b9a` on `2026-03-15`
(`feat(swarm): continuous swarm infrastructure with agent teams (#1553)`).
That commit creates the tracked swarm-state files:

- `completed-slices.md` for deduping work that already exists
- `discovered-issues.md` for leads noticed outside the current slice
- `known-pitfalls.md` for reusable traps and lessons
- `swarm-queue.json` for overlap tracking

That move matters because it shifts coordination out of chat and into a
committed repository surface. The files are explicitly described as durable and
shared across sessions, worktrees, and operators.

---

## 2. March 17 Added A Schema-Backed Findings Ledger

`d9aab31bc` on `2026-03-17` adds the stronger memory layer:
[`findings.json`](../../../.claude/swarm-state/findings.json) and its README.
The README says `findings.json` is not a bug tracker and not a handoff file. It
records stable conclusions that should change how the repo describes or
operates the swarm.

The contract is intentionally narrow:

- stable ID
- kind of finding
- active, landed, or superseded status
- conclusion
- affected surfaces
- evidence pointers
- follow-up PRs or notes

That same area is validated shortly after in `37ddcf56d`
(`fix(swarm): validate empty findings ledgers (#1743)`), which confirms the
empty ledger is a valid bootstrap state. The repo therefore treats memory as a
real artifact even before it is populated.

---

## 3. The Memory Classes Are Deliberately Different

The `swarm-state` README separates the memory surfaces by job:

- `discovered-issues.md` captures product or codebase work noticed outside the
  current slice
- `known-pitfalls.md` captures repeatable failure lessons
- `completed-slices.md` captures lifecycle status and deduplication
- `swarm-queue.json` captures active overlap and ownership
- `findings.json` captures durable control-plane conclusions

`known-pitfalls.md` is especially useful as a contrast class. It is
append-only, human-readable, and meant for lessons from fixer agents and failed
builds. `findings.json` is stricter: it is schema-backed and meant for stable
swarm-control conclusions. Together they show the repo separating short-lived
coordination from durable institutional memory.

---

## 4. GitHub Issues Became The Recovery Channel

The issue tracker then takes on the same memory role from a different angle.
The repo’s `learning:`, `article:`, `friction:`, and `audit:` entries are mostly
issue-title prefixes rather than dedicated labels. That is deliberate: the
title itself carries the memory type.

The key examples are already in the issue genealogy notes:

- `learning: parser fix agent experience report (#1700)` in issue `#2190`
- `learning: parser fix agent experience report (#1703)` in issue `#2191`
- `article: Corpus-Driven Parser Development — Testing Against 4,355 Real CPAN Files` in issue `#2195`
- `article: The Self-Improving Swarm — How Our Development System Learns From Every Session` in issue `#2197`
- `friction: cycle 2 operational friction log — 14 items` in issue `#1678`
- `audit(swarm): cycle 2 improvements & protocol gaps` in issue `#1667`
- `audit: skill definitions have documentation gaps, missing governance skills, and outdated patterns` in issue `#1670`

Those issues are not just backlog entries. They are recovery artifacts:

- learning issues preserve how a fix was found
- article issues preserve publication evidence
- friction issues preserve operational pain
- audit issues preserve protocol gaps and follow-up intent

That is how the swarm recovers context when the live control plane is gone.

---

## 5. Scout Logs Add A Preserved Research Tier

The current swarm also preserves a smaller but important memory class outside
both `swarm-state` and GitHub issues: tracked scout logs.

As of `2026-03-19`, the committed log directory includes:

- `.claude/logs/scouts/2026-03-19-v0.12.0-readiness.md`
- `.claude/logs/scouts/2026-03-19-install-experience.md`

These logs are not active queue state and not stable doctrine. They preserve
dated research passes after their useful conclusions have already been absorbed
into tracked historical docs.

That gives the repo another memory class:

- `swarm-state` for live durable coordination
- scout logs for preserved session evidence
- issue titles for recoverable learning, article, friction, and audit memory

The logs matter because they keep more of the swarm's investigative reasoning
recoverable instead of collapsing everything directly into polished narrative.

---

## 6. The Taxonomy Is A Memory Graph, Not A Flat List

The repo’s memory model now has two linked halves:

1. committed swarm state for durable coordination
2. issue-backed titles for lessons, audits, and story capture

The bridge between them is the issue↔PR lineage recorded elsewhere in the
archaeology notes. The same PR can appear as a live swarm-state lead, a
follow-up pitfall, or a learning/article issue depending on what the swarm
needs to preserve.

That is the important historical shift:

- chat is transient
- swarm-state is committed coordination memory
- scout logs are preserved session research memory
- issue titles are recoverable lesson memory
- findings.json is the durable control-plane conclusion layer

The repo ends up with a taxonomy of memory, not just a backlog.

For the broader current stack that adds operator commands, skill usage, and
preserved scout logs on top of `swarm-state`, see
[KNOWLEDGE_COMPOUNDING_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/KNOWLEDGE_COMPOUNDING_ARCHAEOLOGY.md).

---

## Evidence Pointers

- [.claude/swarm-state/README.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/README.md)
- [.claude/swarm-state/known-pitfalls.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/known-pitfalls.md)
- [.claude/swarm-state/discovered-issues.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/discovered-issues.md)
- [.claude/swarm-state/findings.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/findings.json)
- [.claude/swarm-state/findings.schema.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/findings.schema.json)
- [ISSUE_PR_CROSSLINK_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_PR_CROSSLINK_ARCHAEOLOGY.md)
- [ISSUE_LABEL_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_LABEL_ARCHAEOLOGY.md)
- `9cc2d3b9a`, `d9aab31bc`, `37ddcf56d`
