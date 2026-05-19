# Real-Workspace Provider Baseline Burndown

> **Substrate (already built)**: semantic scorecard (`cargo xtask semantic-scorecard`), semantic shadow compare (`cargo xtask semantic-shadow-compare`), and a CPAN-style seed fixture exercised across crates.
> **Connector gap**: provider-facing completion / goto / hover / diagnostics expectations exercised over the fixture. The substrate measures parser fidelity; what is missing is a provider-level baseline that proves the LSP surface (not just the parser) works on realistic Perl trees.
> **0.14.0 upside**: a single command — once green — that says "the editor surface works on a realistic CPAN-shaped workspace," which is the v0.14.0 editor-trust headline.

## Status

| Phase | Issue | Builder-ready? | PR | Receipt |
|---|---|---|---|---|
| 1. Real-workspace baseline suite | [#8848](https://github.com/EffortlessMetrics/perl-lsp/issues/8848) | yes | current Real Perl Editor Trust baseline PR | [2026-05-13 Mojolicious Windows receipt](../forensics/2026-05-13-real-workspace-baseline-mojolicious.md), [2026-05-14 Dancer2 Windows receipt](../forensics/2026-05-14-real-workspace-baseline-dancer2.md), [2026-05-19 Catalyst Windows receipt](../forensics/2026-05-19-real-workspace-baseline-catalyst.md), and `.ci/metrics/real_project_latency.json` |
| 2. Editor-trust roadmap rollup | [#7952](https://github.com/EffortlessMetrics/perl-lsp/issues/7952) | not yet (`needs-spec`, `v0.14.0`, `umbrella`) | _pending_ | `cargo xtask semantic-shadow-compare --check` |

## Exit criteria

- [ ] All phases land or are explicitly deferred with a successor.
- [x] Phase 1 has a current Mojolicious receipt with explicit covered/deferred
  provider surfaces.
- [x] Phase 1 has a second Dancer2 receipt with the same latency claim boundary.
- [x] Phase 1 has a third Catalyst receipt with latency metrics and fixture
  resource inventory under the same claim boundary.
- [x] Receipt commands in this doc reproduce the closeout proof, with a Git Bash
  fallback documented for Windows agents whose `just` shell is unavailable.
- [ ] Status doc updated (`docs/project/status/semantic_capability_dashboard.md` and `docs/project/status/semantic_scorecard.md` regenerated post-merge).
- [x] Claim boundary recorded in the current receipt.

## Claim boundary

This rail proves that **the LSP provider surface (completion, goto, hover, diagnostics) functions over a CPAN-style real-workspace fixture** with stable, ratcheting baselines.

This rail does **NOT** prove:

- The provider surface is correct on every possible real-world workspace shape. The fixture is representative, not exhaustive.
- The semantic scorecard or shadow-compare numbers are at any target threshold — those thresholds are ratchets owned by their respective xtasks, not by this rail.
- Performance characteristics under load. Latency is owned by other rails (see `MODULE_COMPLETION_RAIL.md`).

## Receipts

```bash
# Phase 1 closeout — provider baseline over the real-workspace fixture
cargo test -p perl-lsp-rs --lib real_workspace

# Phase 2 closeout — shadow-compare delta stays within the ratchet
cargo xtask semantic-shadow-compare --check
```

Both must pass at merge. Regressions are gated by the ratchet logic inside `semantic-shadow-compare`; treat any failure as a hard stop, not as a flake.

## Related

- Umbrella issue: [#7952 — roadmap(editor-trust): track availability, completion, diagnostics, proof, and ratchets](https://github.com/EffortlessMetrics/perl-lsp/issues/7952) (`v0.14.0`, `umbrella`)
- Tracker for this rollout doc: #8627
- Phase 1 issue: [#7949 — test(semantic): add real-workspace baseline suite](https://github.com/EffortlessMetrics/perl-lsp/issues/7949) (`keystone`, `area:tests`)
- Architecture / spec docs: `crates/perl-lsp-rs/` real-workspace test module; `xtask/src/bin/semantic_scorecard.rs`; `xtask/src/bin/semantic_shadow_compare.rs`
- Status docs: [docs/project/status/semantic_capability_dashboard.md](../project/status/semantic_capability_dashboard.md), [docs/project/status/semantic_scorecard.md](../project/status/semantic_scorecard.md)
- Adjacent rails:
  - `MODULE_COMPLETION_RAIL.md` — must not regress provider completion latency on this fixture
  - `IMPORTS_RAIL.md` — should include a literal-require fixture once that rail lands

## Do not combine

Do **not** roll this rail's PRs into:

- Parser-fidelity scorecard work. Those numbers belong to the parser rail, not the provider rail; combining them confuses what is being measured.
- Workspace-discovery refactors. Discovery is upstream substrate; this rail consumes it.
- Any change to the fixture's content (adding modules, tweaking shapes) — that lives under its own PR so the baseline diff is reviewable.

## Lane assignment

The older #7949 phase-1 tracker is historical. The current Real Perl Editor
Trust baseline receipt is tracked by #8848 and records the first covered
Mojolicious fixture/host proof. Phase 2 (#7952) remains an umbrella for broader
editor-trust rollout work and should spawn focused child rails instead of being
built directly.
