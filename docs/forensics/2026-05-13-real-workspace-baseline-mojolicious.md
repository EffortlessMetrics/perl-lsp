# Real-Workspace Baseline: mojolicious (windows)

**Date**: 2026-05-13
**Commit**: a187da840
**System**: windows
**Project**: mojolicious

## Substrate Versions

| Component | Version |
|-----------|---------|
| perl-lsp  | 0.14.0 |
| Rust      | rustc 1.95.0 (59807616e 2026-04-14) |
| Perl      | v5.42.2 |
| OS        | MINGW64_NT-10.0-26200 3.6.7-fb42d713.x86_64 x86_64 |

## Metrics

### Cold-Start to First Hover (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 188 | 1283 | 1283 | 10 |

### First Completion (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 0 | 1 | 1 | 10 |

### Goto-Definition (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 0 | 0 | 0 | 10 |

### Incremental Reparse (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 1 | 2 | 2 | 10 |

### Workspace Symbol Query (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 0 | 0 | 0 | 10 |

## Project Stats

- **Perl files**: 13 (.pm / .pl / .t)
- **Fixture source**: test_corpus/real_projects/mojolicious_skeleton/

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
| Memory profile | deferred | this harness records wall-clock latency, not heap or RSS |

## Provider Confidence Links

- [Provider cutover](../project/status/provider_cutover.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)
- [Semantic scorecard](../project/status/semantic_scorecard.md)
- [Semantic shadow compare](../project/status/semantic_shadow_compare.md)

## Claim Boundary

This receipt supports a measured editor-latency claim for the selected fixture
and host system only. It does not claim full CPAN compatibility, broader
framework coverage, memory/resource ceilings, diagnostic correctness, or live
provider cutover by itself.

## Outliers

- **cold_start_to_hover** p95=1283ms exceeds 500ms threshold


Outliers are recorded threshold misses for the named metric. They do not block
the receipt, but they do block promotion of a no-outlier latency claim for that
metric until a follow-up run or fix clears the threshold.

## Reproducibility Notes

```bash
# Reproduce this measurement
just real-workspace-baseline mojolicious windows

# Windows fallback when just cannot locate its shell
"C:/Program Files/Git/bin/bash.exe" scripts/real-workspace-baseline.sh mojolicious windows
```

- Binary built with: `cargo build -p perl-lsp-rs --release`
- Test invoked via: `cargo test -p perl-lsp-rs --test real_project_latency mojolicious -- --include-ignored --nocapture`
- Samples per metric: 10 (p50/p95/p99)
- Fixture path: `test_corpus/real_projects/mojolicious_skeleton/`

## Notes

Current baseline run for mojolicious on windows. Establishes a Real Perl Editor Trust measurement anchor for the selected fixture and host.
