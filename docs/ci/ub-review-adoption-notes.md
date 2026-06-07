# ub-review Adoption Notes

Adoption log and gap-category taxonomy for the ub-review integration in perl-lsp-swarm.
Cross-reference: [docs/ci/review-gates.md](review-gates.md) — program plan, calibration ledger, upstream tool loop.

---

## Adoption Log

### Datum #1 — Release artifact gap / CX hard-block escalation

**Date:** 2026-06-04
**Issue:** ub-review has no public release artifact for the pinned SHA (`804d198b5a15a0df94bb4f43750dba71165916cd`).
When `install-mode=auto` cannot find a release asset, it falls back to a source build.
On CX43/CX53 self-hosted runners (Docker-only, no host-level Rust), the source build fails with:

```
cargo is unavailable and rustup is not installed
```

The CX runners hard-block because `install-mode=auto` cannot proceed without cargo.

**Upstream tracking:** EffortlessMetrics/ub-review#343 — release artifact for the current pinned SHA.
Until #343 ships, source builds work on GitHub-hosted runners (which have Rust pre-installed)
but fail on Docker-only CX runners.

**Resolution:** See Datum #3.

---

### Datum #2 — Workspace contamination on CX runners

**Date:** 2026-06-04 – 2026-06-05
**Issue:** Incomplete ub-review runs on CX lanes left behind a contaminated workspace state.
Subsequent runs failed not because of a sensor finding but because the workspace was in
an unexpected state from the previous failed run.

**Resolution:** PR #1245 added workspace recovery logic that clears stale ub-review
artifacts before each run. Workspace contamination is now classified as `infra-excluded`
in the calibration ledger.

---

### Datum #3 — cargo-unavailable on CX → GH-only routing

**Date:** 2026-06-05
**Issue:** The CX43/CX53 hard-block (Datum #1) was escalated from advisory failure to
a blocking routing decision: the CX job was causing confusion in the merge-readiness
signal because an infra failure looked like a sensor finding.

**Resolution:** PR #1248 added an explicit runner-routing rule that forces all ub-review
jobs to GitHub-hosted runners until EffortlessMetrics/ub-review#343 ships.
The CX job YAML is preserved with a `# TODO: revert after ub-review#343` comment for a
3-line revert once the release artifact exists.

**Calibration note:** All CX advisory failures before #1245/#1248 are recorded as
`infra-excluded` in `docs/agents/ledgers/ub-review-calibration.jsonl`.

---

### Datum #4 — Secrets preflight prevents silent model-lane failures

**Date:** 2026-06-06
**Issue:** Early runs surfaced a risk: if `MINIMAX_API_KEY` is absent, model lanes
silently produce empty output rather than failing clearly. Empty output could be
misclassified as an expected-quiet run.

**Resolution:** The workflow includes a `Secret preflight` step that fails before any
model call if `MINIMAX_API_KEY` is absent. `OPENCODE` is an optional fallback.
Neither secret value is echoed in logs.

**Calibration note:** Runs where the secret preflight step fails are classified as
`infra-excluded` if the failure is the only issue on the run.

---

### Datum #5 — Advisory FAILURE conclusions poison rollup automation

**Date:** 2026-06-07
**Finding:** When ub-review produces a FAILURE conclusion for an advisory (non-blocking)
finding, downstream rollup automation — CI dashboards, merge-readiness checks, and
status-page aggregators — treats the finding as blocking. This defeats the advisory/required
boundary and causes advisory drift into de-facto blocking behaviour.

**Recommendation:** PR-3 lane must configure advisory findings to produce **neutral**
conclusions, not FAILURE conclusions. The pattern:

- Blocking finding (route policy = required): conclusion = FAILURE with evidence.
- Advisory finding (route policy = advisory): conclusion = neutral summary + receipt
  posted as PR Review comment, not as a check conclusion that blocks merge.

**Upstream ask:** File an upstream issue against EffortlessMetrics/ub-review requesting
a `conclusion-mode: neutral` option for advisory routes. Until this exists, advisory
findings should be posted as PR Review comments only (not check conclusions).

---

## Gap-Category Taxonomy

Use these categories when recording rows in `docs/agents/ledgers/ub-review-calibration.jsonl`.

| Category | Description |
|----------|-------------|
| `runner-routing` | Job failed because the runner type does not support the required toolchain (e.g. Docker-only CX vs GH-hosted). Not a sensor decision. |
| `secret-preflight` | Job failed because a required secret (`MINIMAX_API_KEY`, etc.) was absent. Not a sensor decision. |
| `sensor-artifact` | Sensor produced a malformed or empty artifact. Not a content decision — infra failure in the sensor itself. |
| `comment-quality` | Sensor found a real issue but the comment text was confusing, imprecise, or contained a hallucinated claim. |
| `false-positive` | Sensor fired on safe code without safety-contract evidence. Scout prompt needs tightening. |
| `missing-route-profile` | Finding is real but the route policy has no matching profile entry, so it was not classified correctly. |
| `bad-failure-message` | Finding is real but the FAILURE conclusion message is not actionable (missing evidence, wrong file reference, etc.). |
| `too-slow-fallback` | Source build fallback on the runner exceeded the timeout threshold. |
| `missing-json-field` | Sensor output was missing a required JSON field, causing the aggregator to fail or silently skip the finding. |
| `docs` | Finding is in documentation (prose, comments, PR body). May be advisory-only depending on route policy. |
| `proof-gap` | A claim (in code, PR body, or doc) lacks the evidence artifact required to support it. High-value ub-review finding. |
| `test-gap` | New code path has no test coverage reaching the changed surface. |
| `n/a` | Use for `expected-quiet` rows where no category applies. |
