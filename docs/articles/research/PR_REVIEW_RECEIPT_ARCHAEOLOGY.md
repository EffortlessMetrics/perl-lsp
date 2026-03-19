# PR Review Receipt Archaeology
## How GitHub PRs Became Governance Artifacts

This note traces a specific historical pattern in the repo: a pull request did
not merely carry code. It carried stage labels, gate labels, review effort
signals, comments, check runs, and receipt-like bodies that together made the
PR itself a governance surface.

That is most visible in the Q3 swarm, but the same pattern keeps echoing in
later review-follow-up PRs as well.

---

## 1. The Q3 PR Surface Encoded Review State Directly

The strongest early examples are PR `#153` and PR `#160`, both from the late
September 2025 review burst.

PR `#153` carried a dense state stack:

- `review:stage:sweep-initial`
- `review:stage:sweep-final`
- `review:stage:freshness`
- `gate:hygiene`
- `gate:matrix`
- `gate:fuzz (clean)`
- `gate:security (clean)`
- `gate:policy (clear)`
- `merge-ready`
- `Review effort 4/5`

The PR was not just tagged by topic. It was labeled as a specific point in the
review pipeline, with gate outcomes exposed as labels and review effort encoded
explicitly. The PR also had `35` reviews and `100` comments, with review
traffic from the maintainer plus multiple automated reviewers. That makes it
read as a live governance artifact rather than a static diff.

PR `#160` shows the same model with a different emphasis:

- `review:stage:intake`
- `gate:hygiene`
- `gate:matrix`
- `gate:docs (clean)`
- `gate:perf (ok)`
- `gate:policy (blocked)`
- `gate:policy (clear)`
- `merge-ready`
- `integrative-review`
- `arch:aligned`
- `schema:aligned`
- `docs:complete`

That combination is important because it shows the PR carrying both technical
and governance meaning at once. The PR had `13` reviews and `57` comments. The
blocked/clear policy labels are not a final badge. They are the audit trail.

---

## 2. The PR Body Became A Receipt Bundle

PR `#205` and PR `#209` show the next layer of the pattern.

PR `#205` is a smaller example, but it still carries explicit review-flow
labels:

- `flow:review`
- `flow:integrative`
- `Review effort 4/5`

That tells us the repo was already using PR metadata as a routed workflow
surface, not just a merge queue entry.

PR `#209` is the canonical receipt-heavy example. Its label stack includes:

- `review:stage:intake`
- `merge-ready`
- `gate:docs (clean)`
- `gate:perf (ok)`
- `gate:tests (pass)`
- `gate:security (clean)`
- `gate:policy (clear)`
- `state:in-progress`
- `state:ready`
- `ready-to-merge`
- `flow:integrative`

It also had `6` reviews and `29` comments. The body is a full receipt bundle:
test counts, performance claims, security claims, documentation claims, and a
checklist-style readiness statement. The status rollup attached to the PR shows
many named checks, not a single green/red bit. The review traffic is also
multi-tool: Codex, CodeRabbit, Copilot, and Gemini all appear on the same PR.

Historically, that matters because the PR itself is no longer just carrying a
diff. It is carrying proof about the diff.

---

## 3. Labels, Checks, And Reviews Form A Single Governance Surface

The distinctive thing about these PRs is not that they have labels or CI
checks. It is that the repository uses all of them together as one review
system.

The label set tells you:

- which stage the PR is in
- how much review effort it likely needs
- whether it is merge-ready or still in-progress
- which gate families are blocked or clear

The body tells you:

- what tests ran
- what performance and security claims are being made
- what documentation or architecture obligations were satisfied

The comments and reviews tell you:

- how much manual review happened
- whether the PR attracted follow-up discussion
- whether the PR was a place for gating, repair, or clarification

That is why the PR archive reads like a governance ledger. The repo was
encoding state, proof, and review pressure directly onto the PR surface.

PR `#533`, `feat: implement standardized CI gate harness`, shows the later
surface after some of that governance moved into repo infrastructure. It has
only `2` reviews and `3` comments and no special labels, which is exactly the
point: some of the governance burden had migrated from PR decoration into the
gate harness, receipt schema, and CI status plumbing.

---

## 4. Later Review Repair Became Normal

The later review-follow-up PRs show that the governance model did not stop at
the large Q3 batches.

PR `#237`, `fix: PR #236 review follow-up - dead code and dependency cleanup`,
and PR `#248`, `fix(lsp): harden text fallbacks after #247 modularization`, are
smaller, but they still carry the same pattern of explicit review work:

- `#237` had `2` reviews and `2` comments
- `#248` had `2` reviews and `2` comments

They are interesting because they show review repair becoming a normal PR
shape. A later fix-up is not hidden in chat or left as an implicit obligation.
It is made explicit in the PR title and the PR activity.

That is the same historical move in a smaller form: the PR itself is the place
where the repo records what still needs to be trusted.

---

## 5. Historical Meaning

The PR archive shows three related shifts:

1. Q3 labels turned PRs into staged review artifacts.
2. Receipt-heavy bodies and check rollups turned PRs into proof bundles.
3. Later cleanup PRs turned review repair into routine work.

Read together, that means the repository did not merely use GitHub PRs as
delivery containers. It used them as a governance layer for trusted change.
The PR was where the repo described its state, defended its claims, and
recorded its review history.

---

## Evidence Pointers

- [REVIEW_LABEL_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/REVIEW_LABEL_ARCHAEOLOGY.md)
- [RECEIPTS_LIE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/RECEIPTS_LIE_ARCHAEOLOGY.md)
- [GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md)
- [PR_REVIEW_LOOP_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md)
- [PR_LIFECYCLE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_LIFECYCLE_ARCHAEOLOGY.md)
- GitHub PR archive snapshot on `2026-03-19`
- PR `#153`, PR `#160`, PR `#205`, PR `#209`, PR `#237`, PR `#248`
