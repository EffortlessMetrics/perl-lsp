# Real-Workspace Baseline: catalyst (windows)

**Date**: 2026-05-19
**Commit**: 07a052fde
**System**: windows
**Project**: catalyst

## Substrate Versions

| Component | Version |
|-----------|---------|
| perl-lsp  | 0.14.0 |
| Rust      | rustc 1.95.0 (59807616e 2026-04-14) |
| Perl      | v5.42.0 |
| OS        | MINGW64_NT-10.0-26200 3.6.7-fb42d713.x86_64 x86_64 |

## Metrics

### Cold-Start to First Hover (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 179 | 480 | 480 | 10 |

### First Completion (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 0 | 0 | 0 | 10 |

### Goto-Definition (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 0 | 0 | 0 | 10 |

### Incremental Reparse (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 5 | 6 | 6 | 10 |

### Workspace Symbol Query (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 0 | 0 | 0 | 10 |

## Project Stats

- **Perl files**: 11 (.pm / .pl / .t)
- **Fixture source**: test_corpus/real_projects/catalyst_skeleton/
- **Resource inventory**: 768 source lines, 18,620 source bytes

## Provider Coverage

| Surface | Status | Receipt |
|---------|--------|---------|
| Cold start / first hover | covered | `cold_start_to_hover` |
| Completion latency | covered | `first_completion` |
| Goto definition latency | covered | `first_goto_definition` |
| Incremental reparse | covered | `incremental_reparse` |
| Workspace symbols | covered | `workspace_symbol_query` |
| Workspace indexing | indirect | initialization, document open, and provider requests exercise the fixture workspace; dedicated index-shape receipts remain in provider/status docs |
| Module resolution | indirect | fixture package layout is exercised through hover, completion, goto, and workspace-symbol requests; dedicated module-resolution receipts remain separate |
| Diagnostics | deferred | latency harness does not wait for publishDiagnostics; use diagnostics/provider receipts for diagnostic correctness claims |
| Resource inventory | covered | `test_real_project_resource_inventory_receipt` records per-fixture Perl file, source line, and source byte counts |
| RSS memory profile | deferred | this harness records wall-clock latency and resource inventory, not heap or RSS; use memory plateau receipts for RSS guardrails |

## Provider Confidence Links

- [Provider cutover](../project/status/provider_cutover.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)
- [Semantic scorecard](../project/status/semantic_scorecard.md)
- [Semantic shadow compare](../project/status/semantic_shadow_compare.md)

## Claim Boundary

This receipt supports a measured editor-latency and fixture resource-shape
claim for the Catalyst skeleton on the recorded host system only. It does not
claim full CPAN compatibility, broader framework coverage, RSS memory ceilings,
diagnostic correctness, or live provider cutover by itself.

## Outliers

None - all p95 values within 500ms threshold.

Outliers are recorded threshold misses for the named metric. They do not block
the receipt, but they do block promotion of a no-outlier latency claim for that
metric until a follow-up run or fix clears the threshold.

## Reproducibility Notes

```bash
# Reproduce this measurement
just real-workspace-baseline catalyst windows

# Windows fallback when just cannot locate its shell
"C:/Program Files/Git/bin/bash.exe" scripts/real-workspace-baseline.sh catalyst windows
```

- Binary built with: `cargo build -p perl-lsp-rs --release`
- Test invoked via: `cargo test -p perl-lsp-rs --test real_project_latency catalyst -- --include-ignored --nocapture`
- Resource receipt invoked via: `cargo test -p perl-lsp-rs --test real_project_latency test_real_project_resource_inventory_receipt --profile agent --locked -- --nocapture`
- Samples per metric: 10 (p50/p95/p99)
- Fixture path: `test_corpus/real_projects/catalyst_skeleton/`

## Notes

Current baseline run for catalyst on windows. Establishes the third
Real Perl Editor Trust real-workspace measurement anchor while keeping the
claim bounded to latency and fixture resource shape.
