# Issue-PR Crosslink Archaeology
## How Issue Numbers, PR Bodies, And Follow-Up Issues Became Swarm Memory

This note goes deeper than the genealogy note. It focuses on one specific
historical mechanism: issue and pull request crosslinks became a durable memory
system for the swarm.

The pattern is not just "issues track work" and "PRs close issues." The repo
also learned to:

- use explicit close/fix/resolves language in PR bodies
- write issues that cite prior PRs as evidence or source material
- create learning issues that summarize what a PR taught the next agent
- create article issues that cite PRs as receipts for later writing

Taken together, those links make sessions recoverable without chat history.
The GitHub ledger itself carries the lineage.

All counts below were verified from GitHub CLI snapshots on `2026-03-19`:

- full PR ledger: `gh pr list --state all --limit 2000`
- issue ledger sample: `gh issue list --state all --limit 400`

---

## 1. The Ledger Starts With Explicit Closure

The earliest visible explicit closure in the archive is PR `#20`:

- [#20](https://github.com/EffortlessMetrics/perl-lsp/pull/20) `ci: fix flaky cancellation tests by conditionally ignoring in CI`
- body: `Fixes #15`

That is a small detail with a large implication. The PR is not merely shipping
code. It is declaring which issue it resolves.

The matching issue-side behavior appears immediately:

- [#21](https://github.com/EffortlessMetrics/perl-lsp/issues/21) `Make LSP cancellation tests deterministic (remove cfg(ci) ignores)`
- body references `PR #20` as the temporary fix and `PR #15` as the original attempt

The issue and the PR are already forming a loop:

- the PR says what it closes
- the issue says what the PR meant

That loop is the first version of swarm memory in this repository.

---

## 2. Explicit Close/Fix Language Becomes A Routing Contract

The full PR archive shows that this is not a one-off habit.

- `71` PRs in the full `2000`-PR ledger use explicit closing language such as
  `Closes #...`, `Fixes #...`, or `Resolves #...`

Representative March 2026 examples:

- [#2173](https://github.com/EffortlessMetrics/perl-lsp/pull/2173) `fix(parser): improve delimiter recovery to reduce cascading errors`
- body: `Fixes #1649`
- [#2180](https://github.com/EffortlessMetrics/perl-lsp/pull/2180) `fix(parser): handle arrow after typeglob, block, sub, and builtins (#1703)`
- body: `Fixes #1703`
- [#2221](https://github.com/EffortlessMetrics/perl-lsp/pull/2221) `fix(parser): accept complex expressions in use/no import lists`
- body: `Closes #2184`
- [#2229](https://github.com/EffortlessMetrics/perl-lsp/pull/2229) `feat(lsp): add large file size guard (#2163)`
- body: `Closes #2163`

The exact wording matters less than the protocol:

- the PR body states lineage explicitly
- the issue number becomes part of the merge contract
- later agents can recover scope without reconstructing a chat thread

This is why the crosslink language matters historically. It turns merge text into
searchable task identity.

---

## 3. Issues Start Remembering PRs

The issue archive does the reverse move too.

In the sampled `400`-issue ledger, `32` issues mention PRs in the body.
That is already evidence that the tracker is not just upstream of implementation.
It is also a downstream record of what the implementation taught.

Early examples are straightforward follow-ups:

- [#16](https://github.com/EffortlessMetrics/perl-lsp/issues/16) `Lexer: support single-quote delimiters for s/// operator`
- body: `Fixed in parser: PR #3`
- [#157](https://github.com/EffortlessMetrics/perl-lsp/issues/157) `Integrative Review Summary: PR #153 findings and follow-up actions`
- body frames the issue as a review summary of PR `#153`
- [#198](https://github.com/EffortlessMetrics/perl-lsp/issues/198) `Stabilize Test Infrastructure: Fix 17 Ignored Tests from PR #176`
- body treats PR `#176` as the source of the follow-up queue

These are not backlog issues in the ordinary sense. They are memory packets for
the work already done.

The PR number is the anchor that lets a later session jump directly to the
relevant context.

---

## 4. Learning Issues Turn PRs Into Reusable Experience

The learning issues are the clearest evidence that the repo was preserving
session knowledge in GitHub itself.

In the sampled issue ledger, the learning/article subset with PR references is
small but highly structured:

- `3` issues with titles starting `learning:` or `article:` mention a PR in the body

The learning examples are especially explicit:

- [#2190](https://github.com/EffortlessMetrics/perl-lsp/issues/2190) `learning: parser fix agent experience report (#1700)`
- body cites `PR #2040`
- [#2191](https://github.com/EffortlessMetrics/perl-lsp/issues/2191) `learning: parser fix agent experience report (#1703)`
- body cites `PR #2180`

Those issues are not asking for new implementation. They record what the PR
taught:

- what failed first
- what debugging method worked
- what pattern another builder should recognize next time
- what trap to avoid in a later session

That is swarm memory in its most practical form. A later agent can read the
issue and recover the lesson without chat history, because the issue itself
contains the lesson and the PR number that generated it.

---

## 5. Article Issues Turn PRs Into Publication Receipts

Article issues use the same pattern, but for narrative and publication work.

- [#2195](https://github.com/EffortlessMetrics/perl-lsp/issues/2195) `article: Corpus-Driven Parser Development — Testing Against 4,355 Real CPAN Files`
- body cites `PR #2039` as evidence for the corpus-ratchet story

That crosslink matters because the article issue is no longer speculative
planning. It is a publication seed with receipts attached.

The body can point to a PR and say, in effect: this claim is grounded in this
delivery. That makes the article recoverable as a historical artifact rather
than a free-floating draft.

---

## 6. Why This Makes Sessions Recoverable

The recovery mechanism is simple and durable:

1. the issue number preserves the task identity
2. the PR number preserves the implementation identity
3. explicit close/fix language binds the two together
4. follow-up issues preserve the next question or lesson

That means a later session can reconstruct state from GitHub alone:

- open the issue to see the original problem or lesson
- open the PR to see the implementation and closure language
- open the follow-up issue to see what the next agent should learn

This is better than chat history for archaeology because it is:

- searchable
- timestamped
- durable across worktree pruning
- visible to future maintainers and agents

The repository therefore evolved from a backlog-plus-merge-log model into a
linked memory graph:

- issue -> PR -> learning issue
- issue -> PR -> article issue
- issue -> PR -> follow-up issue

That graph is the reason the swarm can be replayed after the fact.

---

## Evidence Pointers

- [ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md)
- [ISSUE_ROUTING_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_ROUTING_ARCHAEOLOGY.md)
- [Q3_SWARM_PR_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
- [#20](https://github.com/EffortlessMetrics/perl-lsp/pull/20)
- [#16](https://github.com/EffortlessMetrics/perl-lsp/issues/16)
- [#21](https://github.com/EffortlessMetrics/perl-lsp/issues/21)
- [#157](https://github.com/EffortlessMetrics/perl-lsp/issues/157)
- [#198](https://github.com/EffortlessMetrics/perl-lsp/issues/198)
- [#2190](https://github.com/EffortlessMetrics/perl-lsp/issues/2190)
- [#2191](https://github.com/EffortlessMetrics/perl-lsp/issues/2191)
- [#2195](https://github.com/EffortlessMetrics/perl-lsp/issues/2195)
- `71` PRs with explicit close/fix/resolves language in the full archive snapshot
- `32` issues in the sampled archive with PR references in the body
