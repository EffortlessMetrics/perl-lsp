# Release Readiness

> Human-owned. Edit this file to update release call and blocker status.
> Do **not** add `<!-- BEGIN: -->` markers — this file is narrative only.

## Current Release Call

**Latest published release**: `v0.12.0`
**Release target**: `v0.12.1` fix-forward cut
**Ship readiness**: `master` carries the README and hook cleanup needed for a clean `v0.12.1` cut; install guidance must still stay tied to the actual GitHub release and crates.io state rather than the workspace version alone

## Active Blockers

- Release-truth drift must stay closed: repo docs and runbooks must match the actual `perllsp` / `perl-lsp-rs` shipped surfaces, not stale pre-rename examples
- Public install guidance must stay tied to the crates.io truth at release time, not just the local workspace version line

## Fix-Forward Receipts (2026-03-30)

- `v0.12.0` is live on GitHub Releases as of 2026-03-30
- the README and hook-fixture regressions from the first `v0.12.0` tag were fixed on `master`
- this branch bumps the operator and version-sync surfaces to `v0.12.1`

## Component Summary

| Component | Status | Notes |
| --- | --- | --- |
| `perl-parser` | Public alpha | Native parser path |
| `perl-lsp` | Public alpha | Coverage tracked via `features.toml` |
| `perl-dap` | Preview (Native + Bridge) | Native adapter is present; compatibility path retained |
| `perl-lexer` | Public alpha | Context-aware tokenizer |
| `perl-corpus` | Public alpha | Corpus counts tracked in computed metrics |

## DAP Stance

Native + Bridge preview. Harden preview flows is active work.

## Parser Audit Receipts (2026-03-17)

- `just parser-audit` reports `91/91` repo-corpus files parse cleanly
- `63/68` NodeKinds covered (`92.6%`)
- `12/12` GA features covered
- One remaining `P2` interpolation-heavy hang-risk candidate in `crates/perl-corpus/src/gen/builtins.rs`

## CPAN Baseline Receipts (2026-03-17)

- `just cpan-corpus-check` holds the committed baseline at `3139/4355` clean (`72.1%`) for the full installed corpus
- `1579/1579` clean for the strict known-clean manifest

## Coverage Baseline Receipts (2026-03-17)

- Path-aware `cargo llvm-cov` workspace summary established a production-code baseline of:
  - `44.7%` lines (`44,200/98,811`)
  - `46.9%` functions (`3,921/8,353`)
  - `42.6%` regions (`68,424/160,806`)
- Tests, benches, examples, `archive/`, and embedded tree-sitter crates excluded

## Index State Machine Receipts (2026-02-16)

`just ci-gate` plus targeted state-machine tests and workspace benchmarks validated transitions, instrumentation, and caps.

---

*Last Updated: 2026-03-30*
