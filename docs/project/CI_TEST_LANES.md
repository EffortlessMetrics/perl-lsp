# CI Test Lanes

This document maps the current GitHub workflows to the local commands
contributors should run first.

## Canonical Order

```bash
just devex
just pr-fast
nix develop -c just ci-gate
just ci-full
just status-update
just status-check
just release-check
```

## Lane Map

| Lane | Workflow | When it runs | Local equivalent |
|------|----------|--------------|------------------|
| PR smoke | `ci.yml` | Every PR | `just pr-fast` |
| Merge gate | `ci.yml` | `merge-ready`, push to `main` or `master`, manual run | `nix develop -c just ci-gate` |
| Nightly / expensive jobs | `ci-nightly.yml` | Scheduled runs and label-gated PRs | `just ci-full` plus targeted helpers |
| Security | `ci-security.yml` | Path-sensitive pushes and scheduled runs | `just security-audit` |

## Active Labels

The nightly workflow currently uses these labels:

- `ci:coverage`
- `ci:bench`
- `ci:strict`
- `ci:mutation`

Nightly benchmark runs also execute a real-repo LSP latency assertion:
`real_project_first_diagnostics_catalyst_5000_lines_under_5s` in
`crates/perl-lsp-rs/tests/real_project_latency.rs` (`cargo test -p perl-lsp-rs --test real_project_latency ... --include-ignored`).

The broader label catalog lives in [`.github/ci-config.yml`](../../.github/ci-config.yml).
Use that file as the source of truth when you add or rename labels.

## Local Helpers

- `just ci-gate-msrv` and `just ci-full-msrv` validate the same gates on the
  MSRV toolchain.
- `just ci-gate-low-mem` is useful on constrained machines or WSL.
- `just security-audit` runs the local security scan helper.
- `just ci-workflow-audit` checks the repository workflows themselves for drift.
