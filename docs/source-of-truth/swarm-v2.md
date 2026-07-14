# Swarm v3 R1 — lifecycle-state source-of-truth map

**This is a continuity index, not a competing authority.** Every concept
named below is already implemented by an existing, gh-integrated file; this
document exists so a fresh session (or a fresh agent) can find the right
authority quickly instead of re-deriving or re-implementing it. **Do not
restate the logic here — REFERENCE the file.** If this doc and the file it
points at ever disagree, the file is right and this doc is stale; fix the
doc, never the other way around.

Tracking issue: #3886 (Swarm v3 R1). Parent program: #3807. Substrate owner:
#3693/#3753 (the review-protocol train that shipped everything named below).

## Why this exists (the two constraints R1 was built under)

1. **A gate that depends on agents running `cargo xtask <x> check` locally
   will not get run.** Enforcement must be gh-integrated: a CI check on the
   PR, or a GitHub-visible state (a label, a receipt comment, a branch
   ruleset) an agent naturally hits in the course of normal work. A local
   xtask command is at most an implementation detail *CI* invokes — never
   the gate itself. Every authority below is reached through `gh` state
   (PR comments, labels, branch-protection) or a CI-invoked script, not a
   command an agent has to remember to run.
2. **Authority = observed state, not persona, label, or lane.** (The M4
   "Authoring & Certification Integrity" correction — see the #3807 program
   issue.) A PR's lifecycle state, and whether a given review is
   independent, are *computed* from receipts and the live head SHA — never
   asserted by an agent's self-description ("I am the reviewer", "I am
   done"). Independence is relational:
   `reviewed_head == current_head AND reviewer_run ∉ mutation_provenance(head)
   AND verdict clear AND blocking dispositioned`. Any push invalidates
   every receipt bound to the old head.

## Lifecycle states

Source of truth: **`scripts/reviews/state`** (a read-only wrapper that runs
the canonical closeout below and projects its JSON fields onto a state
machine — it never re-derives any GraphQL query itself).

| State | Meaning |
|---|---|
| `REVIEW_IN_FLIGHT` | A review is running (a `review-run:v1` receipt with `status=running`, or the `needs-deep-review` label is set). No verdict yet. |
| `FINDINGS_CLASSIFIED` | Findings exist; dispositions are being recorded but no fix is yet proven reachable at head. |
| `FIXED_HEAD` | Dispositions present and fix commits reachable at head, but no independent verification receipt at head yet. |
| `VERIFIED_HEAD` | An independent verification receipt exists at head, but the PR is not yet fully converged. |
| `CONVERGED` | The closeout reports `converged:true`. |
| `MERGEABLE` (reserved) | `CONVERGED` plus green CI. CI is a separate gate the closeout does not consult — this state is never emitted by `scripts/reviews/state` today. |

The forbidden transition `FINDINGS_CLASSIFIED → CONVERGED` (skipping
`FIXED_HEAD`/`VERIFIED_HEAD`) is structurally impossible, not just
convention: the closeout only reports `converged:true` once
`verification_receipt_head_match` holds for every substantive disposition,
which requires a `disposition:v1` marker, which only
`scripts/reviews/disposition` can post. `state` merely *surfaces* that
ladder; the closeout *enforces* it.

## The `independent(review, head)` predicate

Source of truth: **`scripts/ci/check-pr-review-convergence`** (831 lines;
the CANONICAL review-convergence authority — every ad-hoc reimplementation
of this check across `pr-ready.md` and elsewhere was retired in favor of
this one script). Read its own header comment for the full "ACTIVE vs
outdated vs resolved threads" and R1-protocol-axis rationale; this section
only names where each piece of the predicate lives.

The predicate decomposes into fields the closeout computes and reports in
its JSON verdict (see the script's final `jq -n` block for the authoritative
field list):

- **`reviewed_head == current_head`** — `deep_review_receipt_head_match` /
  `verification_receipt_head_match`: a `review-run:v1`/`verification:v1`
  receipt only counts if its `head` field equals the PR's current
  `headRefOid`. A receipt bound to an older head reads as a head mismatch,
  never as stale-green.
- **`reviewer_run ∉ mutation_provenance(head)`** — the **writer-identity
  set** (see below): a verification receipt only counts if its `verifier`
  is outside the writer set.
- **`verdict clear`** — `review_runs_in_flight == 0` and
  `independent_review_pending == false` (no `needs-deep-review` label, no
  `status=running` review-run receipt).
- **`blocking dispositioned`** — `resolved_without_disposition == 0` and
  `dispositions_missing_marker == 0`: every resolved thread carries a
  machine-checkable `disposition:v1` marker, not just a resolve-with-no-
  reply (the #3647 "resolved-to-clear" incident this axis exists to catch).

`REVIEW_PROTOCOL_ENFORCE=1` promotes the R1 protocol axes above from
advisory (reported, non-blocking) to hard `BLOCK`; the default (unset) mode
reports every finding but only fails on the pre-R1 axes (unresolved threads,
resolved-without-disposition, pending independent review). See the script's
own "Advisory/enforce dispatcher" section.

## Writer-lease and verifier-set relations

Two *distinct* mechanisms answer "who may not certify this PR/branch",
scoped to different questions — do not conflate them:

1. **Writer-identity set (per-PR, "who may not verify")** — computed
   inline in `scripts/ci/check-pr-review-convergence` as `WRITER_SET`: the
   union of the PR author (`.author.login` from `gh pr view`) and every
   disposer (`disposition:v1`'s `by` field). **A reviewer who posts a fix
   disposition thereby JOINS the writer set for that PR** — their own later
   verification receipt at any head is rejected (`verifier` ∈
   `WRITER_SET`), even though they are not the PR's original author. This
   is the mechanic the two new fixtures below pin (see "New regression
   fixtures").
2. **Branch-editing lease (per-branch, "who may push right now")** —
   `scripts/reviews/lease` writes/reads a durable per-branch lease file
   (`.ops-perl-lsp/review-leases/<sanitized-branch>.json`, schema
   `.ci/receipts/schemas/review-lease.schema.json`). Keyed on branch name
   (mechanically knowable via `git rev-parse --abbrev-ref HEAD`), not agent
   identity. Substrate for the "one editing owner per branch" invariant;
   the hook that *consumes* it (R3) and the audit-driven takeover (R5) are
   later PRs — `lease audit` only emits takeover-CANDIDATE lines today,
   never an automatic takeover.

**Not the same as either of the above:** `xtask/src/tasks/agent_lease.rs`
implements a *worktree/task-allocation* lease (`AgentLease`,
`AgentTask`/`CurrentSnapshot`, schema `.ci/receipts/schemas/agent-lease.schema.json`)
— it answers "which agent owns this worktree slot", a scheduling concern,
not a review-identity concern. It is a local xtask command, not a
gh-integrated gate, and is out of scope for the `independent(review, head)`
predicate above.

## Receipt schemas (the wire format every marker/lease above conforms to)

| Schema | Posted by | Read by |
|---|---|---|
| `.ci/receipts/schemas/review-run.schema.json` | `scripts/reviews/run` (`review-start`/`review-done`) | `check-pr-review-convergence` (`review_runs_in_flight`, `deep_review_receipt_head_match`) |
| `.ci/receipts/schemas/review-verification.schema.json` | `scripts/reviews/run verify` | `check-pr-review-convergence` (`verification_receipt_head_match`) |
| `.ci/receipts/schemas/review-disposition.schema.json` | `scripts/reviews/disposition` | `check-pr-review-convergence` (`WRITER_SET`, `dispositions_missing_marker`, `unreachable_fix_commits`, `followups_without_issue`) |
| `.ci/receipts/schemas/review-lease.schema.json` | `scripts/reviews/lease` | `scripts/reviews/lease audit` (future: an R3 PreToolUse hook) |
| `.ci/receipts/schemas/agent-lease.schema.json` | `xtask/src/tasks/agent_lease.rs` (`acquire`) | `xtask/src/tasks/agent_lease.rs` (`verify`) — worktree scheduling, not review identity |

## New regression fixtures (this PR, #3886)

Added to the existing, gh-integrated test suite
`scripts/tests/test-check-pr-review-convergence.sh` (extends real CI
coverage — no new local-only gate):

- `reviewer-who-fixes-becomes-author-for-new-head-blocks` — a reviewer
  ("alice", not the PR author "bob") posts a `fixed` disposition, joining
  `WRITER_SET`; her own later verification at that head is rejected
  (`verification_receipt_head_match: false`).
- `independent-verifier-after-fix-disposition-passes` — the discriminating
  counter-fixture: a distinct third party ("charlie", neither the PR author
  nor the disposer) verifies the same fix at the same head and
  `verification_receipt_head_match: true`.

## Explicitly out of scope for R1 (see #3886's plan-review)

- No new Rust convergence evaluator (`xtask/src/tasks/work_item.rs` was
  proposed and dropped in plan-review: `scripts/reviews/state` already
  projects lifecycle state from the same JSON the closeout produces — a
  second Rust evaluator computing the same thing from the same source with
  no sync guarantee is the duplicate-authority anti-pattern this doc exists
  to prevent, and it is a local xtask command agents would not naturally
  hit).
- No `policy/work-items.toml` (dropped alongside `work_item.rs` — with no
  consumer it would ship as orphaned static TOML).
- No fixture for `required_check_on_H1_cannot_satisfy_H2` — CI-check-run
  state is a separate gate `check-pr-review-convergence` deliberately does
  not consult (see the script's own comments); a fixture for it inside this
  suite would be false coverage. Deferred to R4/R5 when CI-check-run state
  is actually integrated into the convergence model.
- Arming any blocking enforcement beyond what already ships
  (`REVIEW_PROTOCOL_ENFORCE`) — advisory-only; changes no merge verdict
  today.

## Claim boundary

Advisory-only; this PR changes no merge verdict. Composes over #3693/#3753;
does not modify their authority. Assumes the state-derived model documented
above (`scripts/reviews/state` + `check-pr-review-convergence`) is the
target lifecycle representation, not the already-merged #3808 persona-hook
reframe (orthogonal, not superseded).
