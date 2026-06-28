# Close-Proof Policy

> **This policy prevents the 29-PR false-close class observed on 2026-06-06.**
> Every closer agent loads this doc before acting.

Cross-reference: [docs/reference/QUEUE_CONVERGENCE_DOCTRINE.md](../reference/QUEUE_CONVERGENCE_DOCTRINE.md)
rules 1–2 (added in #1253) encode the queue-wide invariants this policy
operationalizes at the individual-close level.

---

## The Problem

On 2026-06-06, 29 PRs were incorrectly closed by agents that:

1. Observed a commit on a branch and concluded it was "already merged"
2. Cited a PR number without verifying the PR was in ancestry of `main`
3. Closed issues whose content was in-ancestry but had been overwritten by a
   subsequent commit

None of these failures required sophisticated reasoning. All required one
shell command that was not run.

---

## Required Proof for Every Close

### Rule 1 — Merge-base proof is mandatory

Any close that claims "superseded", "already landed", or "duplicate of merged
PR" **MUST** include the following command's output pasted verbatim in the
close comment:

```bash
git merge-base --is-ancestor <commit-sha> origin/main && echo "ANCESTOR" || echo "NOT ANCESTOR"
```

**Commit existence on a branch is NOT evidence of landing on main.** A commit
that exists on `feat/X` but not in `origin/main` ancestry is not landed. The
command above is the minimum proof.

The close comment must contain a block like:

```
Merge-base proof:
  Command: git merge-base --is-ancestor abc1234 origin/main && echo ANCESTOR || echo NOT ANCESTOR
  Output:  ANCESTOR
  Verified: 2026-06-07
```

### Rule 2 — Landed-via-PR requires the merge commit

Any close that claims "landed via PR #N" must cite the **merge commit SHA**
for that PR, not just the PR number. The merge commit is the artifact that
is in ancestry; the PR number alone does not prove it.

```
Landed-via proof:
  PR: #N
  Merge commit: <sha>
  Verified in ancestry: git merge-base --is-ancestor <sha> origin/main → ANCESTOR
```

### Rule 3 — Substance check when content may have been overwritten

When the cited commit is in ancestry but the content may have changed since,
grep the distinctive content in the current tree:

```bash
git grep "<distinctive-string>" -- HEAD
```

If the grep returns nothing, the content was overwritten. Do not close —
reland or update the tracking issue instead.

**2026-06-06 incident classes that require this check:**

| Class | Description |
|-------|-------------|
| `in-ancestry-but-content-overwritten` | Commit A landed, then commit B reverted or replaced the relevant section. Commit A is ancestor; content is gone. |
| `cited-commit-on-unmerged-branch` | Agent cited PR #N as evidence; PR #N's branch was never merged into main. The commit hash exists in the remote but is not in `origin/main` ancestry. |

### Rule 4 — Wrong closes must be reopened or relanded

When a false close is discovered:

1. Reopen the issue immediately (or file a replacement if the original was deleted)
2. Add a comment documenting what went wrong and citing this policy
3. File a follow-up issue if the root content was lost and needs relanding

The trail must be documented. Silent reopens without explanation repeat the
failure.

### Rule 5 — Port before close

Content must reach a canonical surface **before** its source closes:

- A doc must be merged to `main` before the issue tracking that doc closes
- A fix must be in an open or merged PR before the bug issue closes
- A feature flag must be in `features.toml` before its tracking issue closes

Closing the issue is the receipt that the work landed. Do not issue the
receipt before the goods arrive.

---

## Multi-Pass Requirement

Superseded/already-landed/duplicate claims are high-wrong-cost (see
[ORCHESTRATION_ROLES.md](ORCHESTRATION_ROLES.md) multi-pass rule table).
Two independent agents must both reach the same conclusion before the close
action is taken.

The first agent produces the proof artifacts. The second agent verifies them
independently. Both agents' output is cited in the close comment.

---

## Close Comment Template

```markdown
Closing as superseded by #N / already landed in <commit>.

**Merge-base proof**
Command: `git merge-base --is-ancestor <sha> origin/main && echo ANCESTOR || echo NOT ANCESTOR`
Output: `ANCESTOR`

**Substance check** (if applicable)
Command: `git grep "<distinctive-string>" HEAD`
Output: `<file>:<line>:<match>`

**Second-pass verification**
Verified independently by: <agent-role>
Verdict: CONFIRMED LANDED

**Policy**: docs/agents/CLOSE_PROOF_POLICY.md
```

---

## When Proof Is Not Obtainable

If the closer agent cannot run `git merge-base` (no repo access, API-only
context), the close must be deferred. Post a comment describing what would
need to be verified, assign the `needs-plan-review` label, and route to an
agent with repo access. Do not close speculatively.
