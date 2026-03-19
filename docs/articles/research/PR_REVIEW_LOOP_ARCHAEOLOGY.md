# PR Review Loop Archaeology
## How This Repo Treats Follow-Up, Cleanup, And Review Repair As Normal Work

The PR archive shows that review loops are not an exception path in this repository. They are part of the operating model.

In my filtered pass over the `gh pr list --state all --limit 2000` ledger slice, I found:

- `10` review-like PRs
- `24` cleanup-like PRs
- `9` follow-up-like PRs

That is enough to show the pattern without pretending the counts are the whole story. The more important signal is that review, cleanup, and follow-up are all named in branch names and PR titles, which means the repo is comfortable making repair work visible rather than hiding it.

---

## 1. Review Is A Separate Phase, Not A Verdict

The strongest evidence is the March 2026 control-plane split:

- [`.claude/commands/review-pr.md`](../../.claude/commands/review-pr.md)
- [`.claude/commands/pr-ready.md`](../../.claude/commands/pr-ready.md)
- [`.claude/skills/triage-prs/SKILL.md`](../../.claude/skills/triage-prs/SKILL.md)

Those surfaces encode the distinction directly:

- review happens before readiness
- readiness happens before merge
- triage handles the residue after batch work

That model matters because it reframes review as a routing step, not as a pass/fail judgment on the author. The repo is built to accept that a first PR may need a second pass.

Representative PRs make that clear:

- `#1696` `feat(review): add /review-pr skill, enforce one-PR-per-agent pattern`
- `#892` `fix(lsp): address unresolved review comments from PRs #881 and #882`
- `#373` `feat(tree-sitter-perl-rs): add manual review edge case detectors`
- `#340` `feat: Implement detection for manual review edge cases`

The review vocabulary is explicit. It shows up in titles, branch names, and workflow surfaces.

---

## 2. Cleanup Is Trusted Change

Cleanup PRs are common enough to be a normal category, not a shame category.

The PR archive contains several cleanup-shaped examples:

- `#9` `Cleanup after PR #1`
- `#237` `fix: PR #236 review follow-up - dead code and dependency cleanup`
- `#623` `chore(docs): cleanup outdated documentation and fix clippy warnings`
- `#841` `chore: v0.9.1 repo cleanup — remove obsolete files and rename references to perl-lsp`
- `#898` `chore: update debt ledger after cleanup campaign`
- `#1601` `feat(swarm): add worktree cleanup script for bootstrap`
- `#1689` `feat(janitor): add mid-cycle worktree cleanup for completed agents`
- `#1961` `feat(skill): add /triage-prs for post-batch-tool cleanup`

That spread matters.

Cleanup is not limited to docs or dead code. It reaches into swarm infrastructure, worktree lifecycle, and post-batch disposal. The repo treats cleanup as a valid engineering output because cleanup reduces future risk and future review burden.

This matches the merge-discipline story in [MERGE_DISCIPLINE_ARCHAEOLOGY.md](MERGE_DISCIPLINE_ARCHAEOLOGY.md): cleanup is part of governance, not a side effect of governance.

---

## 3. Follow-Up PRs Are Repair, Not Embarrassment

The explicit follow-up pattern is small but real.

Examples from the ledger:

- `#237` `fix: PR #236 review follow-up - dead code and dependency cleanup`
- `#892` `fix(lsp): address unresolved review comments from PRs #881 and #882`
- `#118` `docs: comprehensive post-workflow documentation and cleanup enhancements`

Those titles are important because they do not hide the relationship to the earlier PR. The repo names the dependency chain, fixes it, and moves on.

That is a mature pattern:

1. a PR lands or is reviewed
2. review exposes a smaller problem
3. a follow-up PR isolates the repair
4. the repair is merged or disposed of on its own merits

The alternative would be to bury the fix in a larger redo. This repo does the opposite. It prefers a smaller, explicit repair PR because that keeps trusted change auditable.

---

## 4. The Review Loop Became A Control-Plane Surface

The current swarm-era docs show how the repository formalized this behavior:

- [PR_LIFECYCLE_ARCHAEOLOGY.md](PR_LIFECYCLE_ARCHAEOLOGY.md) shows drafts, merges, and closures as lifecycle states
- [SWARM_SURFACE_EVOLUTION.md](SWARM_SURFACE_EVOLUTION.md) shows commands, skills, hooks, and swarm-state becoming the current control plane
- [CONTROL_PLANE_ARCHAEOLOGY.md](CONTROL_PLANE_ARCHAEOLOGY.md) shows the lineage from orchestration guides through the current swarm surfaces

That means review loops are no longer just social processes. They are encoded in the surfaces the repo exposes:

- `review-pr` for focused review
- `pr-ready` for readiness transitions
- `triage-prs` for cleanup and disposal
- `swarm-state` for durable memory of what went wrong or was resolved

The system is designed to remember repair paths, not just final code.

---

## 5. What The Ledger Says About Trust

The PR archive does not suggest a repo that expects first-pass perfection. It suggests a repo that expects correction and has built a way to absorb it.

That is why the review-loop evidence matters:

- review PRs exist as first-class work
- cleanup PRs are normal outputs
- follow-up PRs preserve accountability and traceability
- unresolved review comments become their own repair target

This is trusted change in practice. The trust is not "never make mistakes." The trust is "make the mistake visible, isolate the repair, and keep the control plane honest."

---

## Evidence Pointers

- [MERGE_DISCIPLINE_ARCHAEOLOGY.md](MERGE_DISCIPLINE_ARCHAEOLOGY.md)
- [PR_LIFECYCLE_ARCHAEOLOGY.md](PR_LIFECYCLE_ARCHAEOLOGY.md)
- [SWARM_SURFACE_EVOLUTION.md](SWARM_SURFACE_EVOLUTION.md)
- [CONTROL_PLANE_ARCHAEOLOGY.md](CONTROL_PLANE_ARCHAEOLOGY.md)
- [`.claude/commands/review-pr.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/review-pr.md)
- [`.claude/commands/pr-ready.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/pr-ready.md)
- [`.claude/skills/triage-prs/SKILL.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/skills/triage-prs/SKILL.md)
- `gh pr list --state all --limit 2000`
- `gh pr list --state all --limit 2000 --json number,title,headRefName,createdAt,closedAt,mergedAt,isDraft,baseRefName`
