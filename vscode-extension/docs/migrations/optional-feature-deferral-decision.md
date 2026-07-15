# Optional-feature deferral decision

Status: defer runtime loading changes until feature-attributable receipts show
a material cold-path benefit.

## Current measurement

The measurement ran from the current extension tree on 2026-07-13 with the
repository's npm authority:

```text
npx --yes npm@10.8.2 run compile
exit=0, elapsed=1.59 s
rolldown=1.1.5, out/extension.js=1,303,747 bytes
source-relative imports=23 (20 runtime, 3 type-only)
dynamic imports=0
```

The production configuration is a single, non-minified CommonJS artifact with
`codeSplitting: false`. The current entrypoint therefore has no existing
optional chunk boundary or per-feature bundle receipt. The eager runtime
surface includes these candidate features:

- debugger and test adapter support;
- onboarding and workspace guidance;
- health checks and the health widget;
- include-path, file-creation, and formatting guidance;
- POD and Gherkin providers;
- run-test-at-cursor support;
- streaming completion and MCP support; and
- What's New presentation.

These are candidates for future measurement, not a claim that each one is on
the activation critical path. The three-sample exact-source smoke from [PR
#4132](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/4132) measured
activation and provider/restart/shutdown milestones, but did not attribute
elapsed time to individual static imports. Its receipt therefore cannot prove
that deferring any named feature would improve startup.

The extension now records `feature_activation_metrics.v1` in first-hour
receipts for explicit registration/construction boundaries: MCP, providers,
configuration, debugger, What's New, and onboarding. Each entry records
activation-criticality, registration status and duration, first-use status, and
the fact that static module evaluation is not observable from the current
single eager bundle. Missing first-use marks remain missing rather than being
treated as zero cost.

## Decision

Do not add dynamic imports, new chunks, or feature-registration deferrals in
this slice. The current evidence proves a single bundled artifact and a clean
compile, but it does not provide a before/after comparison or a feature-level
cold-path cost. A speculative deferral could change activation ordering,
command availability, VS Code mocks, published packaging, or provider
readiness without a measured benefit.

This decision does not establish a startup budget or claim that the current
bundle is optimal. It keeps runtime behavior stable while preserving a clear
measurement boundary.

## Re-evaluation proof

A future runtime-deferral PR must select one candidate seam and provide:

1. a receipt that identifies the feature-registration milestone and whether it
   is on the activation critical path;
2. matched same-host current-source smoke runs with and without the deferral,
   including activation, first useful request, warm request, restart, and
   shutdown milestones;
3. package and VSIX inventory parity, failure-path coverage, and command/provider
   availability checks; and
4. a documented decision based on repeated distributions, without introducing
   a performance threshold until the measurements are stable.

Until that proof exists, all current static imports remain intentional and no
optional-feature deferral is part of the extension runtime contract.
