# Reviewer Ecology Archaeology
## How The Repo Moved From Human Review To Bot Review To Receipts

This note tracks the repository's reviewer ecology across eras. The important
shift is not simply "more review." It is that the repo first used GitHub PRs as
a place for human maintainer judgment, then let bots review bots, and finally
moved a large part of the trust burden into gates, receipts, and CI surfaces.

That makes the PR archive useful as evidence for reviewer ecology, not just for
code history.

All counts and PR examples below were verified from the GitHub PR archive on
`2026-03-19`.

---

## 1. The Earliest Ecology Is Human-Heavy But Already Multi-Agent

The strongest early example is PR `#153`,
`Sync master improvements: Agent refactoring and customization features`.
It had:

- `35` reviews
- `100` comments
- review labels such as `review:stage:sweep-initial`,
  `review:stage:sweep-final`, `gate:hygiene`, `gate:matrix`,
  `gate:fuzz (clean)`, `gate:security (clean)`, `gate:policy (clear)`, and
  `merge-ready`
- review effort marked as `Review effort 4/5`

The review log shows a mixed ecology:

- `EffortlessSteven`
- `copilot-pull-request-reviewer`
- `gemini-code-assist`
- `bito-code-review`
- `chatgpt-codex-connector`
- `codiumai-pr-agent-free`
- `coderabbitai`

That is the key historical fact. The maintainer is still present as a review
actor, but the PR is already being reviewed by a small ecosystem of automated
reviewers. The repo did not wait for a later "AI review" era. It already had
one.

PR `#160` makes the same point from a different angle:

- `13` reviews
- `57` comments
- labels including `review:stage:intake`, `gate:docs (clean)`,
  `gate:perf (ok)`, `gate:policy (blocked)`, `gate:policy (clear)`,
  `integrative-review`, `arch:aligned`, `schema:aligned`, and `docs:complete`

Its review log is smaller but still automated-first:

- `copilot-pull-request-reviewer`
- `chatgpt-codex-connector`
- `coderabbitai`

This is not just a human reviewer using bots. It is a PR surface where the
reviewers are already distributed across human and machine actors.

---

## 2. The Q3 Swarm Makes AI-Reviewing-AI Visible

PR `#209`,
`feat(dap): Phase 1 DAP support - Bridge to Perl::LanguageServer (#207)`,
shows the review ecology after the workflow had become more explicit.
It had:

- `6` reviews
- `29` comments
- labels including `review:stage:intake`, `merge-ready`,
  `gate:docs (clean)`, `gate:perf (ok)`, `gate:tests (pass)`,
  `gate:security (clean)`, `gate:policy (clear)`, `state:in-progress`,
  `state:ready`, `ready-to-merge`, and `flow:integrative`

Its review log is entirely automated:

- `copilot-pull-request-reviewer`
- `gemini-code-assist`
- `chatgpt-codex-connector`
- `chatgpt-codex-connector`
- `coderabbitai`
- `coderabbitai`

That is the cleanest evidence in the archive for "AI reviews AI" as a real
operating mode, not a slogan. The PR is still gated, labeled, and merged by
human process, but the review traffic itself is machine-heavy.

This matters historically because the review burden is no longer centered in a
single maintainer's comments. The PR is already carrying stage labels and gate
labels, which means review state can be inferred without rereading the full
comment stream.

---

## 3. Governance Burden Starts Moving Into Gates, Receipts, And CI

PR `#533`, `feat: implement standardized CI gate harness`, is the best marker
for the later migration.
It had only:

- `2` reviews
- `3` comments

Its review log is still automated:

- `gemini-code-assist`
- `copilot-pull-request-reviewer`

That smaller surface is the point. By this stage, the repo has started moving
governance work out of PR comment volume and into actual infrastructure:

- gate harnesses
- receipt schemas
- CI status plumbing
- validator and audit surfaces

So the archive shows a clear transition:

1. In the Q3 swarm, review state is visible directly in labels and comments.
2. In the receipt era, the PR body and comment stream carry structured proof.
3. In the later gate era, the trust burden moves into CI and receipts, so the
   PR itself becomes a thinner interface to a larger verification system.

That is the real reviewer-ecology shift. Review is not disappearing. It is
being externalized into systems that can be checked mechanically.

---

## 4. Repair Reviews Become Routine

Later fix-up PRs show that review itself became a normal work product.

PR `#237`,
`fix: PR #236 review follow-up - dead code and dependency cleanup`,
and PR `#248`,
`fix(lsp): harden text fallbacks after #247 modularization`,
are both examples of review repair becoming explicit.

They do not just absorb review feedback. They name the follow-up as review
work, which means the repository was beginning to treat review repair as part
of the delivery system rather than as informal cleanup.

That is a quiet but important shift in ecology:

- early on, reviewers validate the change
- later, reviewers and bots help shape the change
- later still, the repository records review repair as a normal PR shape

---

## 5. What The Archive Says About The Repo

The reviewer ecology is unusual because it is layered rather than replaced.
The archive does not show a clean handoff from humans to bots. It shows a
stack:

- human maintainer judgment remains present
- automated reviewers become normal
- PR labels encode review state
- receipts encode proof
- gates and CI absorb part of the review load

That is why this repository is interesting as an early AI-age codebase.
It did not merely add bots to code review. It built a review system that could
be partially read by humans and partially enforced by machines, then gradually
shifted the trust burden into the machine-enforceable parts.

---

## Evidence Pointers

- [PR_REVIEW_RECEIPT_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_RECEIPT_ARCHAEOLOGY.md)
- [REVIEW_LABEL_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/REVIEW_LABEL_ARCHAEOLOGY.md)
- [GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md)
- [TRUSTED_CHANGE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/TRUSTED_CHANGE_ARCHAEOLOGY.md)
- GitHub PR archive snapshot on `2026-03-19`
- PR `#153`, PR `#160`, PR `#209`, PR `#237`, PR `#248`, PR `#533`
