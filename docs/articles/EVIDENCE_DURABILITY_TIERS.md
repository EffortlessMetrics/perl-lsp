# Evidence Durability Tiers

A deliberate three-tier design for what evidence releases (and other multi-stage processes) produce, where it lives, and how long it survives. Crystallized during the v0.13.3 release closeout and the regression-lock work that followed.

## The three tiers

| Tier | Where | Lifetime | Granularity | Audience |
|---|---|---|---|---|
| **Receipts** | `target/receipts/<topic>.md`, local working tree | Ephemeral (gitignored, lost on clean) | Rich, free-form, per-session | Current operator + immediate-next-cycle research partner |
| **Workflow artifacts** | GitHub Actions artifacts, uploaded `if: always()` | 14 days (default) to 90 days (configurable) | Structured, schema-backed (JSON, logs) | Anyone debugging "what failed last week" |
| **Tests** | Repo, committed, run on every PR | Permanent (until deleted in a PR) | Asserted invariants tied to specific code paths | Future contributors and future LLMs touching the same code |

Each tier protects against a different memory failure:

- **Receipts** protect *this conversation* — what the operator believed at decision time, what was tried, what worked.
- **Artifacts** protect *recent operations* — what failed on this run, with enough structure to debug without re-running.
- **Tests** protect *durable invariants* — what we *know* is true and want to keep true.

Most projects produce one or two of these tiers. The three-tier design is a deliberate choice; producing all three is what makes incidents recoverable, debuggable, and non-recurring.

## What goes in which tier

### Receipts (ephemeral, rich)

Free-form markdown summarizing a release, an incident, or a session. Captures things that don't fit the structure of artifacts or tests:

- The operator's mental model at decision points
- Channel-by-channel status with workflow run IDs
- Failures encountered, classified, and recovered
- Outstanding items the next cycle should pick up
- Cross-references to the artifacts and tests that constitute the harder evidence

Example: `target/receipts/release-install-surface-v0.13.3.md` captures what shipped, what failed and recovered, and what's pending. It includes things that no test or artifact alone would convey — the timing-race recovery, the VSIX manual-upload, the orchestration follow-ups worth filing.

Receipts are gitignored because they're working notes. **Do not** try to commit them via `--force`; if they need to be durable, they should be promoted to a forensic note in `docs/forensics/` (medium tier) or codified as tests (highest tier).

### Workflow artifacts (medium durability, structured)

CI workflows upload artifacts on every run, success or failure, via `actions/upload-artifact@v4` with `if: always()`. The artifacts are:

- **Structured JSON** of the actions taken and their outcomes
- **Logs** drained from output channels and process stdout/stderr
- **State snapshots** at decision points (config, file presence, mtime, version)

For the v0.13.3 install smokes:

```
target/receipts/vscode-smoke/<source>/<os>/extension-output.log
target/receipts/vscode-smoke/<source>/<os>/command-results.json
target/receipts/vscode-smoke/<source>/<os>/managed-binary-state.json
```

The state file captures lock-process status, mtime delta, source/destination paths, and structured command results. This is what makes a failure debuggable without spelunking through the editor's output channel — the failure analyst gets `firstReinstallOk`, `secondReinstallOk`, `lockingProcessSpawned`, `binaryRewrittenSecondPass` as JSON fields.

The 14-day default retention is enough for "what failed last week" debugging. For incidents that exceed 14 days, the artifact contents should be captured in a forensic note (`docs/forensics/<date>-<topic>.md`).

### Tests (permanent, asserted)

Tests are the highest-durability tier because they're enforced on every PR. A regression that the test catches *cannot ship* without the team noticing. Receipts can rot; artifacts can age out; tests run forever.

The right test for a given failure:

- **System test** (smoke / integration): does the user-visible behavior work? E.g., reinstall twice, both succeed.
- **Unit test** isolating each failure mode: which code path is broken when the system test fails? E.g., source-lock retry budget, destination-lock versioned-dir, pointer sanitization, singleflight semantics.

System tests prove the user path works. Unit tests **attribute** failures to specific code paths. After a release that fixed two architecturally distinct failure modes (source-side AV lock + destination-side running-binary lock), the unit tests in `vscode-extension/src/test/downloader.test.ts` split them apart so future breakage points to the right fix:

```
- Long-tail retry budget tests (covers source-lock fix)
- Versioned managed install layout tests (covers destination-lock fix)
- Singleflight tests (covers concurrency fix)
- Legacy migration test (covers the upgrade path that the smoke can't easily exercise)
```

The smoke could prove "reinstall twice works" but couldn't say which fix was responsible. The unit tests can.

## In-flight backup

A specific use of artifacts that became load-bearing during the v0.13.3 release: **artifacts as recovery points at every join-point in a multi-step orchestration.**

The `Publish VSCode Extension` workflow stored the VSIX as a build-step artifact for debug visibility. When the downstream `Attach VSIX to GitHub Release` step lost its race with release creation, the recovery was:

```bash
gh api repos/.../actions/runs/<id>/artifacts --jq '.artifacts[] | select(.name == "perl-lsp-rs-0.13.3.vsix")'
gh api .../artifacts/<id>/zip > vsix-artifact.zip
unzip vsix-artifact.zip
gh release upload v0.13.3 perl-lsp-rs-0.13.3.vsix --repo <owner>/<repo> --clobber
```

Without the artifact, the only recovery was a re-publish — risky because re-publishing to Marketplace + Open VSX requires bumping the build number or unpublishing first.

**The pattern**: every join-point in a multi-step workflow should produce an artifact that can be replayed downstream. For releases:

| Join-point | Artifact that enables recovery |
|---|---|
| Build → Publish | Built binaries, signed if applicable |
| Publish (Marketplace/Open VSX) → Attach to Release | The VSIX |
| Create Release → Attach assets | The release body, the SHA256SUMS |
| Attach assets → Smoke | Asset list with checksums |
| Smoke → Receipt | Smoke run logs and structured results |
| Receipt → Tap bump | The release URL and tag commit |

This is **in-flight backup**, not just logging. The cost is uploading an artifact (cheap). The benefit is recovery without re-running the entire train.

## When to promote across tiers

The lifecycle of an observation:

```
Encounter the failure
       ↓
Capture in a receipt   ← ephemeral, rich, free-form
       ↓
If it's a recurring class or has long-term debugging value:
   Codify as a forensic note   ← medium durability, structured
       ↓
If a code path can lock the regression:
   Add a unit test            ← permanent, asserted
```

Each tier has a cost (more permanent = more careful authoring required) and a benefit (more permanent = larger blast radius if missed). The right balance depends on:

- How likely is recurrence?
- Can the failure be expressed as a code-path assertion?
- Will the next person hitting this need the rich context (receipt) or the structured assertion (test)?

For v0.13.3, the lifecycle ran: incident receipts → forensic notes (`docs/forensics/2026-05-03-*.md`) → unit tests (in `#7874`). The receipts captured the operator's view; the forensic notes captured the structural lesson; the tests locked the regression.

## Anti-patterns

- **Receipts that try to be permanent.** If something belongs in `target/receipts/`, it shouldn't also live as a committed doc. Pick a tier.
- **Tests that assert system behavior without unit-level attribution.** If the smoke fails, you should know which fix is broken. If you don't, add the unit test.
- **Artifacts without `if: always()`.** Artifacts that only upload on success are not recovery points. They're celebration markers.
- **Skipping forensic notes because "it's already in the PR body."** PR bodies move with the PR. Forensic notes survive the PR being squashed and the branch being deleted.

## Provenance

Three-tier design articulated during the v0.13.3 install-reliability release closeout (2026-05-03). The receipt at `target/receipts/release-install-surface-v0.13.3.md` is the worked example for tier 1. The smoke artifacts uploaded under `target/receipts/vscode-smoke/` are the worked examples for tier 2. The 25 regression-lock unit tests in `#7874` are the worked examples for tier 3.
