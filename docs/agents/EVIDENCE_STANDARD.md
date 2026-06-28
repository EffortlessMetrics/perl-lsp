# Evidence Standard

Every claim an agent makes must be backed by a concrete artifact. This document
defines what counts as evidence and what does not.

Cross-reference:
- [CLOSE_PROOF_POLICY.md](CLOSE_PROOF_POLICY.md) — mandatory proof before closing as superseded
- [../reference/QUEUE_CONVERGENCE_DOCTRINE.md](../reference/QUEUE_CONVERGENCE_DOCTRINE.md) — Rule 1 (merge-base check), Rule 3 (claim-guarded deletion)

---

## Ban List

The following phrases are **banned as standalone evidence**. Using one in a
close comment, label justification, or routing decision without a concrete
artifact is grounds for the action to be reverted.

| Banned phrase | Why it fails |
|---------------|--------------|
| "looks green" | CI state must be read from a check URL, not inferred from appearance. Green badge can be stale (cached from a previous SHA). |
| "seems merged" | Merge state must be confirmed via `gh pr view --json state,mergedAt`. "Seems" is not a command. |
| "probably fixed" | Fix claims require a test name and run output. Probability is not evidence. |
| "already included" | Inclusion requires a merge-base proof. A commit can exist on a branch without being in main ancestry. |
| "label says X" | Labels are set by agents and can be stale or wrong. Label state is never the terminal evidence for a factual claim. |
| "the PR says" | PR body and title are written at open time and may not reflect current state. |
| "I believe" / "I think" | Belief is not an artifact. Run the command. |

---

## Required Evidence by Claim Type

| Claim type | Required artifact |
|------------|-------------------|
| **Merge claim** ("this is merged", "already landed") | Merge commit SHA. Verified by: `gh pr view NNN --json mergedAt,mergeCommit` or `git log origin/main --oneline | grep <sha>`. |
| **CI claim** ("CI is green", "checks pass") | CI check run URL + conclusion field. Example: `gh pr checks NNN --json name,state,detailsUrl`. |
| **Superseded claim** ("superseded by PR N", "already fixed upstream") | Reachability output from `git merge-base --is-ancestor <sha> origin/main && echo ANCESTOR || echo NOT ANCESTOR`. Must be pasted verbatim. See [CLOSE_PROOF_POLICY.md](CLOSE_PROOF_POLICY.md). |
| **Fix claim** ("this test now passes", "bug is fixed") | Test name + test run output (pass line from `cargo test` or CI log). |
| **Release claim** ("shipped in v0.N.M", "available in latest release") | Receipt file path (e.g. `.receipts/v0.N.M-release.md`) or public channel URL (crates.io, GitHub Releases page). |
| **Dead code claim** ("this function is unused", "no references") | Output of `just dead-code` or `cargo machete` confirming zero references. See QUEUE_CONVERGENCE_DOCTRINE Rule 3. |
| **Duplicate claim** ("duplicate of #N") | Diff comparison showing the two PRs cover the same change, plus canonical PR number. |
| **File path claim** ("file exists at docs/X") | Confirmation from `ls` or `git ls-tree` — not from another doc referencing the path. |
| **Version claim** ("current version is 0.N.M") | `grep '^version' Cargo.toml` output, not README or docs. |

---

## Evidence Strength Levels

Evidence is graded. When multiple agents review the same claim, all must use
evidence of equal or greater strength than the original.

| Level | Description | Example |
|-------|-------------|---------|
| **Hard** | Command output pasted verbatim, no interpretation needed | `git merge-base` output: `ANCESTOR` |
| **Traced** | URL that resolves to the artifact | CI check URL + `"conclusion": "success"` |
| **Derived** | Command output that requires one step of interpretation | `git log --oneline | grep <sha>` hit |
| **Asserted** | Agent states a fact without running a command | "The PR was merged" — **insufficient alone** |

Claims that block a close, merge, or release require **Hard** or **Traced** evidence.
Claims that inform routing (not final decisions) may use **Derived** evidence with
a confidence qualifier.

---

## Propagation Rule

When an agent cites another agent's claim as evidence, the citing agent must
re-verify the artifact at the time of citation. Evidence does not transfer
between agent invocations. A claim verified at 09:00 may be stale by 11:00 if
a CI rerun or force-push intervened.

---

## Reporting Format

All evidence must be reported in the JSON output schema used by scout templates:

```json
{
  "claim": "<claim text>",
  "evidence": [
    "<artifact description>: <verbatim output or URL>",
    "<artifact description>: <verbatim output or URL>"
  ],
  "confidence": "high|medium|low",
  "recommended_action": "<action>",
  "blocked_by": "<number or null>"
}
```

Do not summarize evidence in prose outside this structure. The orchestrator
reads the `evidence` array; prose is ignored.
