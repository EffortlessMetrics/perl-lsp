# PLSP-SPEC-0003: Real-workspace editor baseline

Status: proposed
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md)
Linked ADRs: planned `PLSP-ADR-0002`
Linked plan: planned `plans/real-perl-editor-trust/implementation-plan.md`
Status impact: provider cutover, semantic dashboards, real-workspace receipts

## Contract

Synthetic fixtures are necessary but insufficient for editor trust. A
real-workspace baseline proves that parser, workspace, semantic, and provider
behavior still hold together on a representative CPAN-style project tree.

A valid real-workspace baseline must record:

- project name and fixture/source provenance
- host system and toolchain context
- cold start or initialize-to-first-answer timing
- workspace indexing behavior
- module resolution behavior
- completion latency and candidate quality boundary
- goto definition behavior
- hover behavior
- diagnostics behavior
- workspace symbol behavior when relevant
- memory or resource profile when the harness supports it
- provider confidence/freshness state for compiler-backed paths
- raw receipt path or committed forensic/status link

Real-workspace receipts do not replace focused fixtures. They bridge focused
fixtures to user-scale confidence and must feed provider confidence, support
claim, or release-readiness status before claims broaden.

## First Baseline Rule

The first baseline may use one representative project. It should not require
all top-N CPAN projects before the first receipt.

Mojolicious is an acceptable first project because the repo already has a
Mojolicious skeleton fixture and a prior Windows receipt in
[2026-04-28 real-workspace baseline](../forensics/2026-04-28-real-workspace-baseline-mojolicious-windows.md).
Another CPAN-style project is acceptable when the implementation plan records
why it was selected.

## Provider Bridge

Real-workspace baselines must connect to provider confidence work.

At minimum, a baseline PR should state which provider surfaces were exercised:

- completion
- goto definition
- hover
- diagnostics
- workspace symbols
- module resolution
- workspace indexing

When a provider uses compiler-backed facts, the baseline must link to the
provider confidence status that explains source, provenance, confidence,
freshness, fallback, and blocker behavior.

The baseline may report that a surface is not covered yet. Missing coverage is
acceptable only when it is explicit and linked to the next proof target.

## Acceptance

A real-workspace baseline PR satisfies this spec when:

- the project fixture/source is named
- the host system and toolchain context are recorded
- the receipt or forensic doc records cold start and provider response metrics
- completion, goto, hover, diagnostics, module resolution, and indexing are
  either covered or explicitly deferred
- provider confidence/freshness links are present for compiler-backed paths
- any latency or memory threshold failures are explained without weakening the
  threshold
- generated or human-owned status docs link to the receipt when the claim is
  promoted
- the PR body states what user-facing claim is allowed and what remains
  unproven

## Proof Commands

Primary baseline command:

```bash
just real-workspace-baseline mojolicious
```

Targeted latency proof:

```bash
cargo test -p perl-lsp-rs --test real_project_latency mojolicious -- --include-ignored --nocapture
```

Semantic/provider status proof:

```bash
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare --check
git diff --check
```

Use an explicit project argument when the baseline is not Mojolicious, for
example `just real-workspace-baseline dancer2`.

## Non-goals

- no public claim of all-CPAN support
- no requirement to baseline all top-N projects before the first receipt
- no hidden network dependency in ordinary PRs
- no live provider cutover from one real-workspace baseline
- no replacement for parser bucket fixtures or provider confidence receipts
- no release claim unless the support/status surface links the receipt and
  states limitations

## Claim Boundaries

A baseline PR may claim that `perl-lsp` has measured editor behavior on the
specific project fixture and host system recorded in the receipt.

It may not claim broad CPAN compatibility, all-framework support, or live
provider cutover by itself.

A baseline may support a promotion only when the relevant status doc links the
receipt and the provider confidence requirements for that surface are met.

## Status Links

Relevant status and rail docs:

- [Real-workspace provider baseline rail](../development/REAL_WORKSPACE_BASELINE_RAIL.md)
- [Provider cutover](../project/status/provider_cutover.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)
- [Semantic scorecard](../project/status/semantic_scorecard.md)
- [Semantic shadow compare](../project/status/semantic_shadow_compare.md)
