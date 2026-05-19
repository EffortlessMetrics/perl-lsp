# PLSP-SPEC-0006: PR queue disposition

Status: proposed
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked ADRs: none yet
Linked plan: [0.14.0 Readiness Queue](../releases/0.14.0-readiness.md)
Status impact: PR review comments, duplicate-cluster cleanup, merge and close
recommendations

## Contract

Open PRs are backlog evidence, not an ordered merge queue. A maintainer or
agent must review each PR or duplicate cluster against current `master`, choose
an explicit disposition, and leave enough rationale that a later reviewer can
understand why the PR was merged, retained, stacked, or closed.

Queue work must preserve the Real Perl Editor Trust lane rule: review, improve,
merge, or close with exact rationale. Do not close a branch just because it is
old, behind `master`, large, agent-generated, CI-stale, or conflict-prone when
the work is still rebaseable and aligned with an active spec or product goal.

The required maintenance loop is:

1. Inspect current `master`.
2. Compare duplicates and sibling PRs that touch the same crates, files, or
   behavior.
3. Select the retained branch or stack order.
4. Rebase onto current `master`.
5. Fix conflicts only inside the PR's intended scope.
6. Run proof appropriate to the touched surface.
7. Squash merge when the retained PR is clean and still valuable.
8. Close superseded PRs with a specific rationale and retained PR reference.
9. Checkpoint after merge bursts.
10. Pause the merge train for control-plane failures.

Old PR descriptions, stale green checks, and prior agent claims are review
inputs. They are not proof after rebase.

## Classifications

Every PR or cluster review must first assign one classification:

| Classification | Meaning |
| --- | --- |
| `merge-candidate` | In-scope, valuable, and likely mergeable after fresh proof |
| `needs-rebase` | Valuable but not yet reviewed against current `master` |
| `needs-fix` | Valuable but requires in-scope code, test, docs, or proof repair |
| `superseded` | Current `master` or another retained PR already covers the value |
| `draft-hold` | Investigation, proposal, or unstable work that should not merge yet |
| `risky-refactor` | Refactor with enough blast radius to need behavior proof first |
| `stack-member` | Valuable only as part of an explicit stack order |

Classification is not final disposition. For example, a PR may be classified
`needs-rebase` and later disposed as `merged`, or classified `superseded` and
disposed as `superseded-by:<PR>`.

## Valid Dispositions

A PR may be closed or considered complete only with one of these dispositions:

| Disposition | Required rationale |
| --- | --- |
| `merged` | Squash merge completed after fresh proof on current `master` |
| `superseded-by:<PR>` | Retained PR or merged PR includes the behavior, tests, or docs |
| `duplicate-of:<PR>` | Another open PR owns the same scope and should be reviewed instead |
| `misaligned-with:<spec-or-proposal>` | The work contradicts an active spec, proposal, ADR, or claim boundary |
| `unsafe-without-redesign` | The approach has reviewed safety or correctness problems that cannot be fixed in scope |
| `stale-investigation-artifact` | The PR was a discovery artifact and should not become product code |
| `blocked-by-missing-contract` | The work needs a spec, policy ledger, or acceptance contract before implementation can proceed |

If current `master` already contains the useful behavior, use
`superseded-by:<PR>` when the covering PR is known. If the covering change is
not tied to an obvious PR, cite the commit, file, or test evidence in the close
rationale.

## Invalid Closure Reasons

These are not valid close rationales by themselves:

- old
- behind `master`
- large
- agent-generated
- CI-stale
- conflict-prone but rebaseable
- old checks are red
- old checks are green
- another PR title sounds similar but overlap was not checked

Any of those facts may affect priority or proof needs. None of them disposes of
the work without overlap review and a valid disposition.

## Acceptance

A PR or cluster disposition satisfies this spec when the maintainer comment or
PR body includes:

```text
classification:
overlap checked:
current master already contains:
retained branch:
proof run:
close/merge rationale:
follow-up:
```

The comment must be concrete enough to answer:

- Which sibling PRs, merged commits, or touched files were checked?
- Which branch is retained, if any?
- What proof ran after rebase?
- What exact behavior, test, docs, or policy value is being merged or retired?
- What follow-up remains, if any?

When reviewing a duplicate cluster, the retained PR must be identified before
closing sibling PRs. If two PRs are both valuable, the comment must name the
stack order instead of closing one as a vague duplicate.

When a broad gate fails because a gate wrapper times out but the underlying
command passes directly, classify it as a control-plane timeout rather than a
product-code failure. Capture the direct reproduction and decide whether a
small control-plane PR should land before more merges.

## Comment Template

Maintainers and agents may paste this template into PR review, merge, or close
comments:

```text
classification:
disposition:
overlap checked:
current master already contains:
retained branch:
proof run:
result:
close/merge rationale:
follow-up:
```

Use `none` only when the field is truly not applicable. For duplicate clusters,
`overlap checked` should name the PR numbers or files compared.

## Proof Commands

All PR dispositions must run at least the proof required by the touched surface
after rebase. For ordinary code PRs, start with:

```bash
git diff --check
./scripts/storage-doctor
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask fmt
```

For touched Rust crates, run the targeted crate gates:

```bash
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe check --all-targets -p <crate> --profile agent --locked
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe test -p <crate> --profile agent --locked
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe clippy -p <crate> --profile agent --locked -- -D warnings -A missing_docs
```

Parser production changes must also include the relevant parser or corpus
receipt. LSP stdio or end-to-end changes must include the relevant serialized
smoke or test binary when applicable. Refactors must follow the refactor
acceptance contract when that spec exists.

Docs-only disposition specs may use:

```bash
git diff --check
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask ci-hygiene check-doc-paths docs
```

Additional docs checks should run when the changed doc surface has a registered
checker.

## Non-goals

- Do not replace code review, security review, or maintainer judgment.
- Do not require every open PR to merge.
- Do not create a numeric merge order.
- Do not define release readiness or publish approval.
- Do not authorize broad refactors without behavior proof.
- Do not define branch deletion policy.
- Do not hand-edit generated status.

## Claim Boundaries

A disposition comment proves only that the named PR or cluster was reviewed
against the stated current `master` and proof surface. It does not prove broad
product correctness, release readiness, support-tier promotion, parser bucket
movement, or provider cutover.

Closing a PR as superseded means the retained branch or current `master`
contains the specific reviewed value. It does not mean every idea in the closed
PR was globally rejected.

Merging a PR means the stated scope passed fresh proof. It does not validate old
green checks, unrelated claims in the PR body, or sibling PRs in the same area.
