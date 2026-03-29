# Release Readiness

> Human-owned. Edit this file to update release call and blocker status.
> Do **not** add `<!-- BEGIN: -->` markers — this file is narrative only.

## Current Release Call

**Latest published release**: `v0.11.0`
**Release target**: `v0.12.0` initial public alpha
**Ship readiness**: Initial-public-alpha release prep is green on `master`; remaining work is release-day orchestration and publication

## Active Blockers

- Release-truth drift must stay closed: repo docs can say `v0.12.0` for `main`, but published-install guidance must stay tied to the latest tagged release until `v0.12.0` ships

## Final Release Receipts (2026-03-29)

- `just release-check` passed on merged `master`
- `just status-check` passed after regenerating computed status docs
- `CHANGELOG.md` already contains a dated `## [0.12.0] - 2026-03-24` section and keeps `## [Unreleased]`

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

*Last Updated: 2026-03-29*
