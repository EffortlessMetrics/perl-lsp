# Live Signals vs Label Signals

## Principle

Where ground truth exists as a queryable live signal, the live signal is authoritative. Labels are bookkeeping for agent activity, not state machines that compete with live truth.

This means: when the GitHub API can answer "is CI green right now for this HEAD SHA?", that answer supersedes any label's claim about CI state. Labels record what an agent did ("green-ci ran a pass"). They do not record what is currently true when a live source can tell us directly.

---

## The Audit Table

Every label in the repo, classified by whether live ground truth exists for it:

| Label | Has live ground truth? | Live source | Use of label |
|-------|----------------------|-------------|--------------|
| `ci-green` | Yes | `statusCheckRollup` for current HEAD SHA | Informational: "green-ci agent ran a pass and it was green at that time" |
| `needs-ci-fix` | Yes | Same | Informational: "green-ci flagged this needs follow-up at time of check" |
| `mergeable` | Yes | `mergeStateStatus` on the PR | Not a label — queried directly from the API |
| `needs-plan-review` | No | — | Authoritative: routing entry to verification pipeline |
| `accuracy-reviewed` | No | — | Authoritative: signoff that accuracy-scout completed its pass |
| `research-reviewed` | No | — | Authoritative: signoff that research-verifier completed its pass |
| `oppositional-reviewed` | No | — | Authoritative: signoff that oppositional-planner completed its pass |
| `diaboli-reviewed` | No | — | Authoritative: signoff that advocatus-diaboli completed its pass |
| `architecture-reviewed` | No | — | Authoritative: signoff that architecture-reviewer completed its pass |
| `maintainer-issue-reviewed` | No | — | Authoritative: signoff that maintainer-issue completed its pass |
| `plan-reviewed` | No | — | Authoritative: signoff that plan-reviewer completed its pass |
| `spec-reviewed` | No | — | Authoritative: spec-planner created impl branch with `.spec/` files |
| `red-tdd-reviewed` | No | — | Authoritative: red-tdd committed failing tests to impl branch |
| `green-tdd-reviewed` | No | — | Authoritative: green-tdd added edge case and regression tests |
| `review-reviewed` | No | — | Authoritative: standards reviewer completed its pass |
| `maintainer-pr-reviewed` | No | — | Authoritative: maintainer-pr completed project-fit check |
| `pr-responded` | No | — | Authoritative: pr-responder addressed bot comments and CI failures |
| `refactor-planner-reviewed` | No | — | Authoritative: refactor-planner posted simplification plan |
| `green-refactor-reviewed` | No | — | Authoritative: green-refactor simplified implementation |
| `deep-reviewed` | No | — | Authoritative: reviewer-deep correctness check passed |
| `needs-deep-review` | No | — | Authoritative: routing flag for deep review pass |
| `diff-audited` | No | — | Authoritative: diff-auditor signoff on coherence and cleanliness |
| `needs-diff-fix` | No | — | Authoritative: diff-auditor bounce — diff has artifacts or drift |
| `needs-builder-fix` | No | — | Authoritative: routing flag back to builder (typed split planned) |
| `builder-ready` | No | — | Authoritative: spec finalized, build pipeline may start |
| `in-build` | No | — | Authoritative: builder actively working |
| `in-review` | No | — | Authoritative: PR in review process |
| `merge-ready` | No | — | Authoritative: composite signoff, all gates passed |
| `already-fixed` | No | — | Authoritative: close without build |
| `structural-blocker` | No | — | Authoritative: blocks parallel work |
| `follow-up-recommended` | No | — | Authoritative: needs follow-up issue |

**The key split:** `ci-green` and `needs-ci-fix` are the only labels with live ground truth. Every other label is the only signal that the corresponding agent activity occurred.

---

## Implications for Tooling

### Reconciler grounds CI-pair decisions in live state

For labels with live ground truth (`ci-green`, `needs-ci-fix`), the reconciler does not use timeline-based "later applied wins" logic. It queries live CI:

- Live CI green and `needs-ci-fix` exists: flag is stale — strip `needs-ci-fix`
- Live CI red: leave `needs-ci-fix`; `ci-green` is stale but harmless — the live red blocks merge
- Live CI green and neither label exists: PR may still be mergeable; absence of `ci-green` means "green-ci hasn't formally signed off" — not that CI is red

For no-live-signal labels (every other label), the reconciler uses GitHub timeline: if both `deep-reviewed` and `needs-deep-review` exist, the later-applied label wins.

The reconciler implementation: `xtask/src/tasks/queue_reconciler.rs`

### Anti-patterns

**Wrong: merge gate based on `ci-green` label**

```
# Do not do this
if has_label("ci-green") { allow_merge() }
```

Gate based on live `statusCheckRollup` for the current HEAD SHA. The label is informational about when the agent ran, not whether CI is currently green.

**Wrong: treat `ci-green` and `needs-ci-fix` as a contradiction to resolve**

These labels are not contradictory. They record agent activity at different points in time. The live CI state is what resolves them — not arithmetic over which label was applied later.

**Wrong: strip `merge-ready` whenever `needs-ci-fix` exists**

Only strip `merge-ready` if live CI is actually red. If `needs-ci-fix` is stale (live CI is green), the correct action is to strip `needs-ci-fix`, not `merge-ready`.

---

## Implications for Agents

**Skills that apply `ci-green` may simultaneously apply `needs-ci-fix`** when CI is red. These are not contradictory from the skill's perspective — the skill records "I ran a pass (ci-green) and flagged a problem (needs-ci-fix)." The reconciler resolves the apparent tension on the next pass by querying live CI.

**Skills that bounce should not withhold their signoff label.** A bounce and a signoff coexisting briefly is fine; the reconciler resolves on the next pass. Do not withhold `review-reviewed` because you also set `needs-builder-fix` — that creates a gap in the audit trail.

**Exception: one-decision-per-pass still applies.** Agents should not intentionally apply contradictory no-live-signal labels. The above "brief coexistence is fine" applies only to the transition period before the reconciler runs, not to deliberately conflicting outputs. An agent that sets both `deep-reviewed` and `needs-deep-review` in the same pass has made no decision — that is a bug, not a feature.

The one-decision-per-pass rule from CLAUDE.md applies to authoritative (no-live-signal) labels. For informational (live-signal) labels, the live state is the decision, and agent labels are audit trail entries.

---

## Implications for Operators

**Reading a PR's state?**

Query live CI first; trust live > labels for the CI dimension. Labels tell you what agents have done; live CI tells you what is true now.

```bash
# Live CI state for a PR
gh pr view <number> --json statusCheckRollup --jq '.statusCheckRollup[]'

# Latest-per-check (filter stale entries)
gh pr view <number> --json statusCheckRollup \
  --jq '[.statusCheckRollup | group_by(.name) | .[] | sort_by(.completedAt) | last]'
```

**Debugging "why didn't this PR merge"?**

Check in this order:
1. Live `mergeStateStatus` — is the PR actually mergeable (no conflicts, no branch protection failures)?
2. Live `statusCheckRollup` — is CI actually green on the current HEAD SHA?
3. Labels — are required signoff labels present? Are routing labels blocking?

Live state comes first. Labels come second. A PR blocked by live CI failure cannot be unblocked by any label manipulation.

**Emergency override?**

Live state still wins. Labels cannot override a real CI failure. The correct path is to fix the CI failure, not to manipulate labels.

---

## Worked Examples

### Example 1: `ci-green + needs-ci-fix`, live CI green

**Situation:** A PR has both `ci-green` and `needs-ci-fix`. The green-ci agent ran, found a problem, flagged it (`needs-ci-fix`), then a pr-responder fixed the CI failure and pushed. CI re-ran and is now green. No one stripped `needs-ci-fix`.

**What's true:** Live CI is green. `needs-ci-fix` is stale — it recorded a problem that has since been resolved.

**Reconciler action:** Query live CI → green. Strip `needs-ci-fix`. The `ci-green` label stands. `merge-ready` can stand if other gates are complete.

**Do not:** Strip `merge-ready` because `needs-ci-fix` exists. That would block a valid merge on stale label state.

---

### Example 2: `ci-green + needs-ci-fix`, live CI red

**Situation:** Same label state. But this time a new commit was pushed after green-ci ran, and the new commit broke a test. Live CI is red.

**What's true:** Live CI is red. `ci-green` is stale — it recorded a green state that no longer applies to the current HEAD SHA.

**Reconciler action:** Query live CI → red. Leave `needs-ci-fix`. Do not strip it (the problem is real). `ci-green` is stale but stripping it is not urgent — the live red blocks merge regardless.

**Operator action:** Dispatch green-ci agent to address the failure. Do not merge.

---

### Example 3: No `ci-green` label, live CI green

**Situation:** A PR has passed all review gates (`deep-reviewed`, `diff-audited`, `merge-ready`) but the green-ci agent has never formally run — possibly because the gate order was compressed or the agent was skipped for a docs-only PR.

**What's true:** Live CI is green. `merge-ready` is present. No signoff label from green-ci.

**Reconciler/operator action:** Live CI is green. The absence of `ci-green` means "green-ci agent hasn't formally signed off." For the merge gate, live CI is what actually gates the merge. If ops policy requires a formal green-ci pass, dispatch green-ci to do a quick confirmation. If the PR is docs-only and the gate was intentionally skipped, the live green is sufficient.

**Do not:** Block the merge solely because `ci-green` is absent when live CI is green. The label is informational; the live signal is authoritative.

---

## See Also

- [ORCHESTRATION_DOCTRINE.md](ORCHESTRATION_DOCTRINE.md) — the broader design philosophy behind why live signals take priority over labels
- Reconciler implementation: `xtask/src/tasks/queue_reconciler.rs` — where this principle is encoded as automated logic
