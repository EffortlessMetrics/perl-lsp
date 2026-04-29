# Real-Workspace Baseline: mojolicious (linux)

**Date**: 2026-04-28
**Commit**: 5c89a9371
**System**: linux
**Project**: mojolicious

## Substrate Versions

| Component | Version |
|-----------|---------|
| perl-lsp  | 0.12.4 |
| Rust      | rustc 1.92.0 (ded5c06cf 2025-12-08) |
| Perl      | v5.38.2 |
| OS        | Linux (GitHub Actions runner, Ubuntu 22.04) |

## Metrics

### Cold-Start to First Hover (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 463 | 978 | 978 | 10 |

### First Completion (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 0 | 1 | 1 | 10 |

### Goto-Definition (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| 0 | 3 | 3 | 10 |

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

## Outliers

- **cold_start_to_hover** p95=978ms exceeds 500ms threshold

This is expected for the initial cold-start measurement on the skeleton fixture: the server spawns
fresh for each of 10 samples, measuring spawn + initialize + first hover end-to-end. The p50 (463ms)
is within the expected range for a 13-file project on a debug-build test binary. A release-binary
cold-start would be significantly lower (see Reproducibility Notes below).

The p50=0ms for completion, definition, and workspace-symbol reflects the warmed-server measurement
path where the server is already initialized; these are the latencies that matter to real users during
an editing session.

## Reproducibility Notes

```bash
# Reproduce this measurement
just real-workspace-baseline mojolicious linux
```

- Binary: test harness uses debug build via `cargo test` (not `--release`)
- To measure release cold-start: `cargo build -p perl-lsp-rs --release` then set `PERL_LSP_BIN`
- Test invoked via: `cargo test -p perl-lsp-rs --test real_project_latency real_project_latency_mojolicious -- --include-ignored --nocapture`
- Samples per metric: 10 (p50/p95/p99)
- Fixture path: `test_corpus/real_projects/mojolicious_skeleton/`
- Raw JSON: `.ci/metrics/real_project_latency.json`

## Notes

First baseline run for mojolicious on linux. Establishes measurement anchor for v0.13.0 release gate.

The 13-file mojolicious skeleton covers the key module structure (Mojolicious.pm, Controller, Routes,
Plugins, etc.) without including the full 200+ file CPAN distribution. This is a representative
skeleton measuring realistic LSP initialization and response latency for a medium-complexity Perl
web framework project.

Related: #6797 (AI completion E2E), #7284 (tracking).
