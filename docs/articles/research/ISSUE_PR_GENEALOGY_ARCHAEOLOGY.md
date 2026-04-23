# Issue-PR Genealogy Archaeology
## How Issues And PRs Became A Shared Delivery Ledger

This note traces a specific historical shift in the repository: issues and pull
requests stop acting like loosely related GitHub objects and start acting like a
shared delivery ledger.

Early on, issues mostly describe problems and PRs mostly ship fixes.
Later, the links between them get stronger:

- PRs explicitly close issues
- issues reference prior PRs as evidence
- swarm-discovered issues become future PR seeds
- learning and article issues cite PRs as receipts

That is not just better hygiene. It is the repo turning GitHub itself into a
traceable genealogy of discovery, implementation, validation, and learning.

All counts in this note were verified from local GitHub CLI archive snapshots on
`2026-03-19`:

- full PR ledger: `gh pr list --state all --limit 2000`
- issue sample ledger: `gh issue list --state all --limit 400`

---

## 1. The Linkage Starts Immediately

The earliest explicit closing PR in the full ledger is PR `#20`, created on
`2025-08-26`:

- [#20](https://github.com/EffortlessMetrics/perl-lsp/pull/20) `ci: fix flaky cancellation tests by conditionally ignoring in CI`

Its body says:

- `Fixes #15`

That matters because the repo is already using the issue tracker as more than a
parking lot. It is treating issue resolution as something a PR should declare
explicitly.

The issue side shows the same behavior almost immediately. Issue `#16`, also
created on `2025-08-26`, references an earlier PR directly:

- [#16](https://github.com/EffortlessMetrics/perl-lsp/issues/16) `Lexer: support single-quote delimiters for s/// operator`
- body reference: `Fixed in parser: PR #3`

So from the first week of visible history, the repo already has both directions:

- PRs closing issues
- issues referencing PRs as prior work or nearby evidence

That is the seed of a shared ledger.

---

## 2. Q3 Makes Issue-Linked Delivery More Deliberate

By late Q3 2025, issue-linked delivery is clearly part of the canonical swarm
shape.

Representative issue-linked PRs from the early label-heavy and PR-shaped period:

- `#158` `Complete Substitution Operator Parsing Implementation (#147)`
- `#159` `feat: Enable missing documentation warnings with comprehensive API docs (Issue #149)`
- `#174` `feat(perl-parser): restore architectural integrity for Issue #146`
- `#205` `feat(parser,lexer): eliminate fragile unreachable!() macros (Issue #178)`
- `#209` `feat(dap): Phase 1 DAP support - Bridge to Perl::LanguageServer (#207)`

This matters because the Q3 swarm is already PR-shaped, and the issue numbers
are being carried into that PR flow as identity markers. Work is not just
"change a file." It is "advance issue-shaped work through the swarm."

This lines up with the canonical Q3 swarm machinery documented elsewhere:

- `generative/` = `issue-to-draft`
- `review/` = `draft-to-pr`
- `integration/` = `pr-to-merge`

Issue identity is therefore part of the Q3 swarm's three-phase flow, not an
afterthought added later.

---

## 3. The PR Archive Quantifies The Shift

Across the full `2000`-PR archive slice:

- `284` PRs mention an issue in title or body
- `71` PRs use explicit closing language in the body such as `Closes #...`,
  `Fixes #...`, or `Resolves #...`

The distribution over time is the interesting part.

### Explicit closing-language PRs by month

- `2025-08`: `3`
- `2025-09`: `3`
- `2025-10`: `3`
- `2025-11`: `7`
- `2025-12`: `2`
- `2026-01`: `4`
- `2026-02`: `2`
- `2026-03`: `47`

That means March 2026 alone accounts for `47` of the `71` explicitly
issue-closing PRs in the full ledger.

So while issue linkage existed from the beginning, March 2026 is when explicit
closure language becomes swarm-normal rather than occasional.

The maintainer's clarification makes the reason more specific: this was not
just prose cleanup or a style fad. The sharper close/fix/resolve language came
from general automation and handoff alignment for agents. Once builders,
reviewers, scouts, and later learning issues all had to recover task lineage
mechanically, explicit closure language stopped being optional nicety and
started becoming routing infrastructure.

That lines up with what the repo is doing in that period:

- scouts discover issues
- builders open PRs against those issues
- PR bodies explicitly close the routed task
- learning and article issues are created from the resulting work

The linkage becomes operational, not merely archival.

---

## 4. March 2026 Turns Linkage Into A Delivery Protocol

The March 2026 wave is where the genealogy becomes especially legible.

Representative PRs from `2026-03-19` alone:

- `#2034` closes `#1660`
- `#2039` closes `#1889`
- `#2125` fixes `#2031`
- `#2171` closes `#1651`
- `#2202` references `#436`
- `#2206` references `#1704`
- `#2229` closes `#2163`

In the current open-PR snapshot, `27` of the first `80` open PRs already carry
explicit close/fix/resolve language in their bodies.

That is strong evidence that issue-linked PR authoring is now a real protocol.
The PR is expected to say what issue it is advancing or closing.

The important nuance is that this delivery protocol was deliberate. It helped
the swarm hand work across agents without losing identity:

- scouts could route issue-shaped work
- builders could publish against the same issue identity
- reviewers could reason about scope without rereading prior chat
- later learning and article issues could cite the same lineage as evidence

This is one reason the current swarm can sustain more parallel work without
losing legibility:

- issue numbers preserve task identity
- PRs preserve implementation identity
- the closing language binds them together mechanically

The result is a lineage trail a later session can recover without rereading all
of chat history.

---

## 5. Issues Start Referencing PRs For Learning And Publication

The issue side evolves too.

Within the sampled `400`-issue ledger:

- `32` issues explicitly mention PRs

The earliest examples are classic engineering references:

- issue `#16` references PR `#3`
- issue `#21` names the follow-up after PR `#20`

Later examples are qualitatively different.

By March 2026, issues are referencing PRs not just to request code, but to
capture conclusions:

- [#2190](https://github.com/EffortlessMetrics/perl-lsp/issues/2190) `learning: parser fix agent experience report (#1700)` references both issue `#1700` and PR `#2040`
- [#2191](https://github.com/EffortlessMetrics/perl-lsp/issues/2191) `learning: parser fix agent experience report (#1703)` references PR `#2180`
- [#2195](https://github.com/EffortlessMetrics/perl-lsp/issues/2195) article planning cites PR `#2039` as evidence for the corpus-ratchet story

This is an important shift:

- early issues point at PRs as nearby technical context
- later issues point at PRs as receipts, lessons, and publication evidence

That means the issue tracker is no longer only upstream of the PR. Sometimes it
is downstream of the PR, recording what the PR taught the swarm.

---

## 6. The Repo Moves From Backlog To Genealogy

Read together, the archive shows four phases of issue↔PR relationship:

1. **Problem/Fix**
   Issues describe bugs; PRs fix them.

2. **Issue-Tagged Delivery**
   PR titles and bodies start carrying issue identity more consistently.

3. **Swarm-Routed Closure**
   In March 2026 especially, PR bodies explicitly close the issues they were
   routed from.

4. **Learning And Story Capture**
   Issues start citing PRs as evidence for lessons, audits, and article
   material.

That fourth phase is the distinctive one.

GitHub is no longer just tracking work.
It is preserving the lineage of work.

---

## 7. Why This Matters For Archaeology

This repository is unusually legible because it tends to preserve both sides of
the relationship:

- the problem identity
- the implementation identity

And later:

- the lesson identity
- the publication identity

That is why the issue/PR archive is so valuable here. It does not only answer
"what landed?" It also answers:

- what problem was being worked?
- what PR resolved it?
- what did the fix teach later agents?
- what evidence is reused in launch-story documentation?

That is a much richer historical record than a normal backlog plus merge log.

---

## Evidence Pointers

- [ISSUE_ROUTING_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_ROUTING_ARCHAEOLOGY.md)
- [Q3_SWARM_PR_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
- [REVIEW_LABEL_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/REVIEW_LABEL_ARCHAEOLOGY.md)
- full PR ledger snapshot from `gh pr list --state all --limit 2000 --json number,title,body,createdAt,mergedAt,url`
- issue ledger snapshot from `gh issue list --state all --limit 400 --json number,title,body,createdAt,url`
- representative examples: `#20`, `#16`, `#159`, `#174`, `#209`, `#2039`, `#2190`, `#2191`, `#2195`, `#2229`
