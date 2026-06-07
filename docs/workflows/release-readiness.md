# release-readiness Workflow

Adversarial release-readiness check for perl-lsp-swarm. Six sequential phases
with structured-output schemas, culminating in a sonnet adversarial synthesizer
that actively tries to refute readiness.

**Human approval is always required.** This workflow produces a recommendation;
it never dispatches a release, creates a tag, or publishes a crate.

---

## What It Checks

| Phase | Model | What it verifies |
|-------|-------|-----------------|
| 1 Ancestry | haiku | `merge-base --is-ancestor` — swarm SHA reaches release branch. Tree diff explained; only documented exclusions accepted. |
| 2 Consistency | haiku | Version sites agree (`cargo xtask check-version-sync`), tag absent, CHANGELOG entry present, release-history script passes. |
| 3 Receipts | haiku | Quality-gate, coverage, and ripr receipts are fresh (< 24 h from release commit). Active proof exceptions enumerated with expiry dates. |
| 4 Smoke | sonnet | Release build (perl-lsp-rs + perl-dap), workspace lib tests, LSP-UX smoke, package verification (archive + checksums + install script). |
| 5 Claims | haiku | Release notes and channel claims audited against evidence. Flags "live", "production", absolute claims without public verification artifact. |
| 6 Verdict | sonnet (adversarial) | Synthesizer starts from "not ready" and tries to REFUTE. Returns `{dispatch_recommendation: go\|no-go, blockers[], evidence[], explicitly_requires_human_approval: true}`. |

Each phase returns a structured JSON object. Phase 6 receives all prior phase outputs
as inputs and cross-examines them adversarially.

---

## How to Invoke

Use the Claude Code **Workflow** tool with:

```json
{
  "name": "release-readiness",
  "args": {
    "swarmSha": "<40-char commit SHA on swarm main>",
    "sourceRepo": "EffortlessMetrics/perl-lsp-swarm",
    "version": "0.16.1"
  }
}
```

The workflow is defined in `.claude/workflows/release-readiness.js`.

---

## Human-Approval Boundary

The workflow **always** returns `explicitly_requires_human_approval: true`.

No action beyond the recommendation is taken by the workflow or any agent it
spawns. The release captain reads the phase outputs and verdict, then decides
whether to proceed. Proceeding means:

1. Reviewing the blockers list — it must be empty for a `go` recommendation.
2. Reading the evidence list and phase verdicts.
3. Explicitly authorizing the tag and publish sequence outside this workflow.

A `go` recommendation from this workflow is a necessary but not sufficient
condition for a release. The release captain makes the final call.

---

## Blocker Criteria

A finding is a **blocker** (forces `no-go`) when it meets any of these:

- **Correctness:** data loss, crash, or wrong output in any smoke test.
- **CI gate:** any required check is failing on the release commit SHA.
- **API contract:** a published interface has breaking changes without a semver major bump.
- **Ancestry:** the swarm SHA is not in the release branch ancestry (phase 1 FAIL).
- **Version inconsistency:** version sites disagree (phase 2 FAIL).
- **Stale receipt:** any quality-gate, coverage, or ripr receipt is > 24 hours old (phase 3 FAIL).
- **Unverified claim:** any release note uses absolute language without a cited artifact (phase 5 FAIL).

---

## 0.16.0-Cycle Lessons

This workflow encodes lessons from the 2026-06 convergence-to-release cycle:

- **QUEUE_CONVERGENCE_DOCTRINE.md Rule 1** — evidence requires a merge-base check,
  not a label or comment assertion. Phase 1 (Ancestry) operationalizes this.
  See [docs/reference/QUEUE_CONVERGENCE_DOCTRINE.md](../reference/QUEUE_CONVERGENCE_DOCTRINE.md).

- **QUEUE_CONVERGENCE_DOCTRINE.md Rule 2** — consolidation merges diff against
  merge-time master. Phase 1 checks the tree diff and requires unexplained exclusions
  to be empty.

- **The four-layer plan** (queue convergence → trust floor → product closure → release)
  identifies ancestry and receipts as the two most commonly missing gate artifacts.
  Phases 1 and 3 address these directly.
  See [docs/project/plans/2026-06-convergence-to-release.md](../project/plans/2026-06-convergence-to-release.md).

- **Claim fabrication** (CLOSE_PROOF_POLICY.md, ub-review calibration datum #1) —
  fabricated breakdown numbers in release notes go undetected without explicit
  cross-referencing against evidence. Phase 5 (Claims) audits every claim.

- **Advisory FAILURE conclusions poisoning rollup** (ub-review adoption datum #5) —
  the adversarial synthesizer in phase 6 distinguishes advisory from blocking findings
  and requires explicit blocker evidence before issuing a `no-go`.

---

## Related Files

| File | Purpose |
|------|---------|
| `.claude/workflows/release-readiness.js` | Workflow definition (phases, schemas, adversarial instructions) |
| `docs/reference/QUEUE_CONVERGENCE_DOCTRINE.md` | Durable rules from the 0.16.0-cycle convergence |
| `docs/project/plans/2026-06-convergence-to-release.md` | The four-layer plan |
| `docs/agents/ledgers/workflow-outcomes.jsonl` | Historical workflow outcome data |
| `docs/reference/RELEASE_PROOF_PROTOCOL.md` | Full release proof requirements |
| `docs/agents/ORCHESTRATION_ROLES.md` | Release Captain role definition |
