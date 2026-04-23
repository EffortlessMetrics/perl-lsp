# Issue Family Genealogy Archaeology
## How Recurring Issue Families Carry Discovery, Fixes, Follow-Ups, And Learning Forward

This note is narrower than the broader issue/PR genealogy notes. It focuses on
recurring issue families and the full lineage they leave behind:

- discovery issue
- implementation PR
- bridge or follow-up PRs
- learning or article issue

The important point is not aggregate counts. It is that the repository often
preserves a recoverable family tree for the same underlying problem, even when
the exact issue number changes or the work branches into sibling threads.

All examples below were verified from local GitHub CLI snapshots on
`2026-03-19`.

---

## 1. The Cancellation Family Forks, Then Reconverges

The LSP cancellation problems are a good example of a family that starts as one
bug, splits into related threads, and then reconverges around a canonical fix.

The lineage is visible in three pieces:

- issue `#21`, `Make LSP cancellation tests deterministic (remove cfg(ci) ignores)`
- issue `#48`, `Fix LSP cancellation test failures and cleanup unused test helpers`
- PR `#20`, the temporary CI-ignore bridge
- PR `#165`, the eventual `Enhanced LSP Cancellation System` fix

Issue `#21` is the deterministic-test thread. Its own comments explicitly say it
is related to `#48`, and the issue body records PR `#20` as the temporary
ignore-based fix.

Issue `#48` then becomes the primary cancellation thread. Its comments call out
`#21` as the deterministic-testing approach, and later it records that the
issue was resolved by PR `#165`.

That is a recoverable family tree:

1. the bug is discovered
2. PR `#20` patches over the problem temporarily
3. issue `#48` becomes the canonical tracking thread
4. PR `#165` provides the durable implementation

The important archival lesson is that the repo does not always keep one issue
per root cause. It can fork a family, then later collapse it back into a single
implementation line.

---

## 2. The Parser-Fix Family Has Clear Sibling Branches

The parser-fix work is even more explicit because the learning issues preserve
the exact issue/PR pairings.

One branch is the `for` / `foreach` family:

- discovery issue `#1700`
- implementation PR `#2040`, `fix(parser): handle for/foreach without explicit loop variable`
- learning issue `#2190`, `learning: parser fix agent experience report (#1700)`

The learning issue records what made the work harder than expected:

- the obvious patterns already worked
- the real failures were declarator and token-routing edge cases
- `assert_clean_parse` was misleading because it missed uppercase `(ERROR ...)`

That makes the family recoverable in three steps:

1. issue `#1700` identifies the parser family
2. PR `#2040` fixes the concrete root causes
3. issue `#2190` captures the hard-won lessons for the next agent

The sibling branch is the `unexpected_arrow_expr` family:

- discovery issue `#1703`
- implementation PR `#2180`, `fix(parser): handle arrow after typeglob, block, sub, and builtins (#1703)`
- learning issue `#2191`, `learning: parser fix agent experience report (#1703)`

This branch is especially useful because PR `#2180` names the architectural
move that made the fix possible: extracting `parse_postfix_chain()` so nodes
created outside the normal expression chain could still accept `->`.

Taken together, `#1700` and `#1703` show that the repo does not only track a
bug. It tracks a family of parser ambiguities, then preserves the exact fix
shape and the exact testing lesson that came out of each branch.

---

## 3. The Corpus-Ratchet Family Connects Discovery, Repair, And Publication

The corpus baseline work is a shorter lineage, but it is still recoverable.

- discovery issue `#1889`
- implementation PR `#2039`, `chore: update stale corpus baselines (closes #1889)`
- article issue `#2195`, `article: Corpus-Driven Parser Development — Testing Against 4,355 Real CPAN Files`

PR `#2039` is the implementation node. Its body records the actual corpus
movement:

- CPAN clean files `3,139 -> 3,484`
- CPAN rate `72.1% -> 80.0%`
- system clean files `5,139 -> 5,892`
- system rate `72.4% -> 83.0%`

The article issue then reuses that PR as publication evidence. That is the key
archaeological point: the same family does not end at merge. It becomes source
material for the public story about corpus-driven development.

So this family shows a slightly different shape:

1. stale corpus state is discovered
2. PR `#2039` ratchets the baselines
3. issue `#2195` turns the ratchet into launch-story evidence

---

## 4. Review-Driven Families Turn PRs Into Follow-Up Issue Trees

Some families start from a PR rather than from a classic backlog issue. That is
still genealogy, because the PR creates its own downstream issue tree.

The strongest example is PR `#153` and its integrative review issue `#157`.

PR `#153` is the implementation node. Issue `#157`, `Integrative Review
Summary: PR #153 findings and follow-up actions`, records the full review
outcome and creates the follow-up issues:

- `#154` performance regression
- `#155` mutation testing improvement
- `#156` agent validation enhancement

This is the same repository pattern in a different direction:

1. implementation lands
2. integrative review turns the PR into structured memory
3. follow-up issues split the remaining work into separate repair tracks

That is why the archive is useful for genealogy. The codebase preserves not
only how work started, but how it was decomposed after review.

---

## 5. What Makes The Genealogy Recoverable

The recurring pattern is not just `issue -> PR`. It is:

- issue numbers preserve problem identity
- PR numbers preserve implementation identity
- bridge PRs preserve the temporary or partial fix
- learning issues preserve what future agents should remember
- article issues preserve what the repo decided was worth telling publicly

The family tree is therefore recoverable from GitHub alone, even when the
underlying work spans multiple sessions or splinters into sibling issues.

That is the main historical value here. The repo is not only tracking work. It
is preserving lineage.

---

## Evidence Pointers

- `gh issue view 21 --json number,title,body,url,createdAt,closedAt,comments`
- `gh issue view 48 --json number,title,body,url,createdAt,closedAt,comments`
- `gh issue view 157 --json number,title,body,url,createdAt,closedAt,comments`
- `gh issue view 1700 --json number,title,body,url,createdAt,closedAt,comments`
- `gh issue view 1703 --json number,title,body,url,createdAt,closedAt,comments`
- `gh issue view 1889 --json number,title,body,url,createdAt,closedAt,comments`
- `gh issue view 2190 --json number,title,body,url,createdAt,closedAt,comments`
- `gh issue view 2191 --json number,title,body,url,createdAt,closedAt,comments`
- `gh issue view 2195 --json number,title,body,url,createdAt,closedAt,comments`
- `gh issue view 2197 --json number,title,body,url,createdAt,closedAt,comments`
- `gh pr view 20 --json number,title,body,url,createdAt,mergedAt,headRefName,baseRefName`
- `gh pr view 165 --json number,title,body,url,createdAt,mergedAt,headRefName,baseRefName`
- `gh pr view 2040 --json number,title,body,url,createdAt,mergedAt,headRefName,baseRefName`
- `gh pr view 2180 --json number,title,body,url,createdAt,mergedAt,headRefName,baseRefName`
- `gh pr view 2039 --json number,title,body,url,createdAt,mergedAt,headRefName,baseRefName`
