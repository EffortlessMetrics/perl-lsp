---
tags: [external-truth-gate, product-docs, fact-verification, cross-pr-reference, ci-noise, non-required-checks, flaky-gates]
repos: [perl-lsp-swarm]
related: ["#3319", "#3315", "#3308", "#3276", "#3324"]
portable: true
article_asset: true
search_terms: [critic.include, critic.severity, perlcritic_severity, perl-lsp.critic, product-surface-docs, cross-PR feature reference, external-truth check, doc fact verification, unit_routed_full, non-required check, cancelled run]
---

# External-truth review of product-surface documentation and CI noise triage

**Date**: 2026-07
**Hazard class**: External-truth-gate / product-surface-docs
**Portable lesson**: [docs/concepts/external-truth-gate.md](../concepts/external-truth-gate.md)

## What happened

Two distinct findings in the native-stack campaign (#3276):

1. **External-truth review of fact doc (#3319)**: The native perlcritic rule-matrix documentation misstated three facts:
   - Documented `critic.include` as a cross-profile force-enable when it's actually a whitelist WITHIN a profile
   - Put `severity` under `[critic]` when it's actually under `[diagnostics] perlcritic_severity`
   - Named VS Code settings `perl-lsp.critic.*` that only existed in sibling PR #3308 and had not yet merged

   Codex secondary review caught all three on first read. Each was re-verified against the source code and LSP spec before fixing. The lesson: a user-visible fact doc must be verified against the code, not a recon summary — the external-truth gate applies to docs.

2. **CI noise triage (meta-pattern)**: Throughout the campaign, nearly every "CI failure" webhook was one of three types:
   - Non-required checks — the **two** merge-blocking required checks are `Perl LSP Rust Small Result` and `ripr+ New Gap Gate` (per `.ci/policies/required-checks.toml`); everything else (`CI Gate (Merge-Blocking)`, `PR Smoke`, `droid-review`, and — despite CLAUDE.md's stale "three required checks" line — **`Codecov / Patch 95`, which is `required = false` / advisory**) does not block merge.
   - The known 66%-flaky `unit_routed_full` gate (now tracked in #3324)
   - A cancelled/superseded run (prior push superseded by a new one), or a CX43 self-hosted-runner Docker-image-missing failover to the GitHub-hosted fallback

   Distinguishing real required-check failures from noise required careful read of the CI log. The incident: noise was treated as signal, causing unnecessary retries and branch churn.

   > **Meta-instance of this very lesson:** an earlier draft of this doc listed `Codecov / Patch 95` among the required checks because it followed CLAUDE.md's summary. The authoritative source (`.ci/policies/required-checks.toml`) marks Codecov `required = false`, and `CI_GATE_PLAYBOOK.md` calls it advisory. Verify the required-check set against the policy inventory, not a doc summary — CLAUDE.md's list is stale (a follow-up should reconcile it).

## Why

1. **Doc facts are not code**: A documentation summary can be correct in spirit but wrong in detail. Cross-profile vs. within-profile is a semantic distinction that only the code or the spec can answer. Recon-based writing (summarizing what the feature does) is faster but error-prone for fact docs. The external-truth gate — verification against the source — applies to user-visible facts in docs, not just code.

2. **Cross-PR feature reference**: When a doc references a feature in a sibling in-flight PR, the reference is only valid after that PR merges. Documenting `perl-lsp.critic.*` before #3308 landed meant the doc was temporarily false. Fact-checking must account for merge order.

3. **CI noise**: Non-required checks are exactly that — not required. The merge gate has **two** required checks (`Perl LSP Rust Small Result`, `ripr+ New Gap Gate`); all others are advisory. A failed advisory check is not a blocker. A cancelled run is not a failure — it's a superseded attempt. Without this triage, the CI dashboard looks red when the required gates are actually green.

## Fix

In PRs #3319 and #3315 + #3324:

1. **Doc fact verification**: Fact-checked all three statements against source:
   - `critic.include`: read the config handler code and LSP spec to confirm it's a whitelist within the profile
   - `severity`: traced from VS Code config through LSP notification to the backend, confirming `perlcritic_severity` under `diagnostics`
   - `perl-lsp.critic.*`: verified against the PR #3308 diff and cross-referenced the exact commit on the branch

   The doc was corrected to match the verified facts.

2. **Cross-PR reference deferral**: For features that don't exist yet in `main` (e.g., native support added by PR #3308), the doc reference is either:
   - Included only in the sibling PR's doc section (localized, no cross-repo reference)
   - Deferred to a follow-up PR that lands after the feature PR merges

   In this case, native-surface docs were included in PR #3319 because the feature was already merged in #3308 at review time.

3. **CI noise triage (procedural)**: The distinction is already documented — [docs/reference/CI_GATE_PLAYBOOK.md](../reference/CI_GATE_PLAYBOOK.md) and the authoritative [`.ci/policies/required-checks.toml`](../../.ci/policies/required-checks.toml) are the source of truth:
   - Required checks: **two** gates (`Perl LSP Rust Small Result`, `ripr+ New Gap Gate`), `required = true`
   - Advisory checks: all others (`Codecov / Patch 95`, `CI Gate (Merge-Blocking)`, `PR Smoke`, `droid-review`), failures are informational
   - Cancelled runs: not failures, caused by a new push or a superseding run

   Tracking in issue #3324: future improvement to label or filter non-required checks in the CI dashboard.

## Spec impact

Follow-up tracked (not yet written — this PR only touched `docs/learnings/`):

- Add a row to [docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md): "Product-surface documentation: every user-visible fact (config keys, settings, version-gated behavior, diagnostic wording) must be verified against the source (code, spec, or running behavior) before merge; cross-PR feature references must account for merge order."
- The [external-truth-gate](../concepts/external-truth-gate.md) concept already states the gate applies to user-visible facts; a small addition could make its application to **docs** (not just code) explicit.

## Portable lesson

**Product-surface docs require fact-checking like code**: A user-visible fact in documentation is a claim that can be wrong. It must be verified against the external oracle (the code, the spec, the running system).

- **Pattern**: [docs/concepts/external-truth-gate.md](../concepts/external-truth-gate.md)
- **Class**: External-truth-gate. A claim is verified against the source.
- **Generalization**: A fact doc is not "done" when it's written; it's done when it's verified against the code. The verification must name the oracle ("verified against `config/critic.rs` line 42" or "verified against LSP spec 3.17 §ConfigurationChange").

**Cross-PR feature references must account for merge order**: When a doc references a feature added in a sibling in-flight PR, the reference becomes valid only after that PR merges. Pre-merge documentation is temporarily false.

- **Pattern**: [docs/concepts/external-truth-gate.md](../concepts/external-truth-gate.md) + procedural coupling awareness
- **Generalization**: Feature cross-references are temporal. Either localize the reference (doc in the same PR as the feature), or defer (doc PR lands after feature PR), or mark the reference as forward-looking ("coming in #XXXX").

**CI noise triage**: Non-required checks are advisory. Distinguish real failures (required checks) from noise (advisory checks, cancelled runs) before deciding to retry or churn the branch.

- **Pattern**: [docs/reference/CI_GATE_PLAYBOOK.md](../reference/CI_GATE_PLAYBOOK.md) (required-check list and noise categories)
- **Class**: Observability / gate output honesty. A failed optional check is not a blocker.
- **Generalization**: Required checks are the binding constraint. Advisory checks are free to fail without blocking merge. Cancelled runs are not failures. The CI dashboard is only actionable if it separates signal (required) from noise (advisory).

## Related PRs

- [#3319](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/3319) — native perlcritic rule matrix docs with fact-checking verification
- [#3315](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/3315) — strict native-product-surface scanner
- [#3308](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/3308) — native perlcritic config integration (sibling, feature referenced in #3319)
- [#3324](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3324) — tracking: CI noise triage and non-required-check filtering
- [#3276](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3276) — epic: native-stack product-surface campaign
