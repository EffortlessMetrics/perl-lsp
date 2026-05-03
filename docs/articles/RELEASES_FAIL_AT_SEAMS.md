# Releases Fail at Seams, Not at Logic

A reframe of where to invest hardening effort in release pipelines, learned the expensive way during the v0.13.3 closeout.

## The observation

Of the failures that surfaced during the v0.13.3 release pipeline run, **the individual jobs were almost all correct**. What failed was the seams between them.

Concrete instances:

| Seam | What failed | Class |
|---|---|---|
| `Validate Release` ↔ master CI on squash-merge | `gh api .../commits/$SHA/status` returned `pending` because master CI hadn't reported on the just-merged SHA yet | timing race |
| `Attach VSIX to GitHub Release` ↔ release creation | "Release not available yet" — both were in the same workflow, attach ran before the release was queryable | ordering race |
| Smokes in publish workflow ↔ install-endpoint propagation | Marketplace + Open VSX gallery API showed 0.13.3 instantly; install endpoints lagged by minutes | propagation lag |
| Release-prep version bump ↔ `RELEASE_HISTORY.md` row | Drift gate fails on every subsequent PR because the ledger doesn't list the just-released version | bundling drift |

Each is a different join-point. Each was a perfectly valid step in isolation. The failure mode lives between them, not inside them.

## Why this matters

Most CI investment goes into job correctness. Linters check the syntax of each job. Test gates check the behavior of each step. Schemas validate each artifact. **None of those tools test what happens between jobs.**

The release-orchestration workflow has been green many times. Each individual step has been exercised. The seams that bit us this session had been latent for prior releases — they only manifested under specific timing patterns:

- The validate-race needs a release dispatched within ~1 minute of merge. Earlier releases waited longer (or weren't dispatched immediately after merge).
- The VSIX-attach race needs the parallel-job graph to schedule attach before the release-creation step has propagated through GitHub's API. Hosted-runner scheduler variance.
- The propagation-lag failure needs smokes to run inside the publish workflow rather than after. Earlier published-smoke workflows were always dispatched separately, so the issue was invisible.
- The ledger-drift failure needs the version bump and the ledger update to be split across two PRs. Earlier releases bundled them.

When the system is faster, more parallel, or more decomposed, seams that were latent become live. That's a *good* problem to have — it means the architecture is being exercised — but it requires a different testing surface.

## Job correctness is not orchestration correctness

Tools that catch:

- Job correctness: linters, type checks, unit tests, schema validators, individual step exit codes.
- **Orchestration correctness**: ?

The second category is mostly absent from standard CI tooling. The closest analogs are:

- Idempotency tests (run the workflow twice; second run should no-op or recover gracefully).
- Race-window probes (run two related workflows back-to-back with no gap; observe whether downstream sees stale state).
- State-precondition assertions (each step explicitly verifies the conditions it expects, rather than assuming the previous step "made it work").

None of these are common practice. Release-engineering teams typically discover them empirically, the way we just did.

## What harder seams look like

Patterns that survive timing pressure:

**Idempotent join-points.** If `Attach VSIX` is idempotent, the race ordering doesn't matter — whichever runs first or second produces the same end state. This is the hardest of the four to retrofit because it requires both upstream steps (the publish that produces the VSIX) and downstream observers (the smoke that consumes the release) to tolerate either order.

**Explicit precondition checks with retry windows.** `Validate Release` should not just call `gh api .../status` once. It should poll for ~5 minutes accepting `pending` as "wait, retry," failing only when something *actively* fails or the timeout exceeds. The fix is small (5-line change) and would have prevented the entire incident.

**Artifacts as recovery points.** When a join-point fails, the workflow's artifacts are the only thing left to recover from. The VSIX attach was rescued because the publish step had stored the VSIX as a workflow artifact for *separate reasons* (debug visibility). Without that artifact, the only recovery would have been a re-publish. **Storing intermediates as artifacts at every join-point is in-flight backup, not just logging.**

**Bundled invariants.** Release-prep should not be able to land a version bump *without* a corresponding ledger row. Either the orchestration appends the row after publish, or the release-prep generator owns the row before merge, or the publish step itself fails if the ledger is stale. Splitting the invariants across PRs creates the bug; bundling them prevents it.

## The recursion

The observation generalizes beyond release engineering. Multi-agent LLM workflows have the same shape: ChatGPT writes a packet, the executor reads it, GitHub state changes between, and the *failure modes are at the seams* — premise drift, stale snapshot, auto-retarget vs. auto-close, etc.

Releases fail at seams. Workflows fail at seams. Multi-agent loops fail at seams. The job logic is rarely the bug. The boundary is.

## Implications for this codebase

The two follow-up issues that should land before the next release:

1. **`fix(release): validate-release polls combined-status before failing pending`** — fix the timing-race seam.
2. **`fix(release): orchestration auto-appends RELEASE_HISTORY row on publish, OR release-prep generator owns the row`** — fix the bundling-drift seam.

Both are small. Both prevent recurrence of failures that have already cost a session each. Neither would have been caught by adding more tests to the existing jobs — they require seam-level thinking.

The broader investment is **testing orchestration as a distinct surface**: race-window probes, idempotency assertions, precondition checks with retry tolerance, artifact-coverage at every join-point. Most of this tooling doesn't exist yet. It's worth building.

## Provenance

Pattern crystallized during the v0.13.3 install-reliability release closeout (2026-05-03). Concrete failures documented in `docs/forensics/2026-05-03-validate-release-squash-timing-race.md`, `docs/forensics/2026-05-03-release-orchestration-attach-vsix-race.md`, `docs/forensics/2026-05-03-marketplace-publish-vs-install-endpoint-lag.md`, `docs/forensics/2026-05-03-release-history-ledger-drift.md`.
