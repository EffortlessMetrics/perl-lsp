# Failure Classification

A small shared taxonomy for triaging CI/release failures. Stability of the labels is what makes the [Agent Handoff Protocol](AGENT_HANDOFF_PROTOCOL.md) work — once a class is assigned, downstream layers trust it.

## Five classes

| Class | What it means | Response pattern |
|---|---|---|
| **product** | Code or user-visible behavior is wrong | Fix-forward in the relevant PR; add regression test |
| **workflow** | Automation / orchestration / job-boundary issue | Fix in workflow files; file follow-up issue if structural |
| **flake** | Runner, network, or transient external dependency | Re-run the failing job; investigate only on repeated occurrence |
| **state-drift** | The packet's premise has changed since it was written | Surface delta; re-sync before continuing |
| **permission/secret** | Access, token, or credential failure | Stop. Don't patch around. Escalate to whoever owns the secret |

## Class-by-class with examples from this codebase

### product

Code is wrong. The behavior the user sees is incorrect; tests prove it.

**Examples:**
- Windows `EBUSY` on first reinstall because the retry budget was too short for Defender's first-time scan window. The fix was code: extend `MANAGED_INSTALL_RETRY_DELAYS_MS`. Regression locked in `vscode-extension/src/test/downloader.test.ts`.
- A running `perllsp.exe` could not be overwritten on reinstall. Fix was code: versioned managed install dirs. Regression locked at the same path.

**Response shape:**
- The owning PR is responsible.
- Add a unit-level regression test that captures the failure mode independently of the surrounding system test.
- If the bug spans multiple substrates (smoke vs. unit vs. release), split the regression into per-layer tests so future breakage is attributable.

### workflow

The job logic is correct in isolation; the failure is in how jobs compose. These are the most common failures in mature systems because individual jobs get tested but join-points usually don't.

**Examples:**
- `release-orchestration.yml`'s `Validate Release` ran `gh api repos/.../commits/$SHA/status` immediately after a squash-merge. Master CI on the new SHA hadn't reported yet, so combined-status returned `pending`, and validation failed. Fix is workflow-side: poll, or accept "no required checks failed yet" as pass-through.
- `Attach VSIX to GitHub Release` raced with release creation in the same workflow. The release didn't exist yet when the attach step ran. Fix is workflow ordering or retry.
- `RELEASE_HISTORY.md` was not updated as part of the v0.13.3 release-prep PR; the drift gate (`just ci-release-history`) now fails on every PR opened against master. Fix is workflow-side: orchestration should auto-append the row after publish, or the release-prep generator should own it.

**Response shape:**
- Identify the join-point (precondition, ordering, retry, idempotency, ledger).
- Fix at the join-point, not inside the job.
- File a follow-up issue if the join-point pattern recurs across workflows.

See [../articles/RELEASES_FAIL_AT_SEAMS.md](../articles/RELEASES_FAIL_AT_SEAMS.md) for the broader pattern.

### flake

Failure does not reproduce. Runner-level, network-level, or third-party-state issue.

**Examples:**
- Marketplace + Open VSX install endpoints return "Extension not found" for ~5-10 minutes after publish, even though the gallery API has indexed the new version. Smokes that run inside the publish workflow fail; the same smokes dispatched 5 min later succeed. The class is "external propagation lag," which is a flake from the workflow's perspective.
- GitHub anonymous API rate limit hit during local repeated dev runs of the integration smoke. CI runs with `GITHUB_TOKEN` are unaffected.

**Response shape:**
- Re-run the failing job once.
- If it fails the same way twice in a row, escalate to workflow class — there's a real issue.
- Document repeated-flake incidents under `docs/forensics/` so the next executor can recognize the pattern fast.

### state-drift

The packet's premise no longer matches reality. This is the dominant failure of the multi-layer protocol — the slow loop writes from a snapshot that's already changed by the time the fast loop reads it.

**Examples:**
- A research-partner packet referenced PR `#7871` as needing rebase; the executor found `#7871` had been auto-closed by GitHub when its base branch was deleted, and a replacement `#7872` already existed.
- A research-partner packet quoted "GraphQL: Could not resolve to a PullRequest with the number of 0" as the validate-release failure mode; the executor's log inspection showed the actual failure was `"Commit ... is not in a successful CI state (pending)"`. Different cause; different fix; the packet was written from a hypothesis, not the actual log.
- A research-partner packet treated `v0.13.3` as unpublished and recommended fixing the validation gate before re-dispatching; the executor had already recovered by re-dispatching after master CI completed, and the release was live with all smokes green.

**Response shape:**
- Surface the delta in the executor's reply (`Ground truth at <time>: ... / Delta from your packet: ... / Now executing: ...`).
- Do **not** silently execute the packet's recovery path on a stale premise — it will either no-op or reintroduce the failure mode the original recovery already addressed.
- Ask whether the rest of the packet still applies given the corrected premise.

The mechanical fix to reduce state-drift is documented in the handoff protocol's "Step 0: verify the premise" rule.

### permission/secret

Token, credential, or access boundary issue. **Stop and escalate.** Do not paper over with workarounds.

**Examples:**
- Workflow can't push a release-history bump PR because it lacks `pull-requests: write` on the GitHub token.
- `crates.io` publish fails with 403 because the token was rotated.
- `vsce` Marketplace publish fails because the PAT expired.

**Response shape:**
- Identify the credential/permission scope that's missing or wrong.
- Surface to whoever owns the secret. Do not pursue workarounds (different token, different action, etc.) without explicit authorization — the failure usually indicates a credential rotation or scope change that the owner needs to know about.

## Stability of labels

Once an executor classifies a failure, the same label persists across cycles. A failure labeled "workflow timing race" stays that label across the diagnosis report, the recovery action, the follow-up issue, and the eventual fix PR. Re-classification is allowed only if **new evidence** changes the picture, in which case the executor surfaces the change explicitly.

This stability is what lets the CTO trust labels without re-deriving classification on every status update.

## Where this gets cited

- [AGENT_HANDOFF_PROTOCOL.md](AGENT_HANDOFF_PROTOCOL.md) — Step 0 verification + executor reply shape.
- [RELEASE_PROOF_PROTOCOL.md](RELEASE_PROOF_PROTOCOL.md) — release-cycle failure triage.
- `docs/forensics/2026-05-03-*.md` — concrete instances of each class from the v0.13.3 closeout.
