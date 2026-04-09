# Release Readiness

> Human-owned. Edit this file to update release call and blocker status.
> Do **not** add `<!-- BEGIN: -->` markers — this file is narrative only.

## Current Release Call

**Latest GitHub/editor release**: `v0.12.3` (2026-04-09)
**crates.io line**: `v0.12.2` (2026-04-07)
**Release target**: `v0.13.0` public alpha announcement
**Ship readiness**: `v0.12.3` is live on GitHub Releases, VS Code Marketplace, and Open VSX; the remaining launch work is announcement-path cleanup, with `#3302` demo assets still the main human-owned blocker before the `v0.13.0` public alpha announcement

## Active Blockers

- `#3302` demo GIFs remain the main human-owned blocker before the `v0.13.0` public alpha announcement
- Public install guidance must keep the channel split explicit while crates.io remains on `v0.12.2`

## 0.12.3 Ship Receipts (2026-04-09)

- GitHub release `v0.12.3` published 2026-04-09 against `cc801735`
- `Release` workflow completed successfully and attached the cross-platform `perllsp` archives plus `SHA256SUMS`
- `Publish VSCode Extension` completed successfully; `perl-lsp-rs` `0.12.3` is live on both VS Code Marketplace and Open VSX
- workspace version line is `v0.12.3`; `check-version-sync` still expects all 140 version sites to agree with it
- crates.io intentionally remains on `v0.12.2` as of 2026-04-09, so docs and install guidance must keep that split explicit

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

## Corpus Tracking Receipts

- **Compatibility baseline (`just corpus-sweep-check`)**: Ubuntu system Perl in `.ci/parser-corpus-baseline.json` is the "does this still parse what ships on a stock Linux box?" receipt. Current committed floor: `6890/7095` clean (`97.1%`) on Perl `5.038002`, refreshed 2026-04-09 after the parser regression fixes.
- **Ecosystem-breadth baseline (`just cpan-corpus-check`)**: `.ci/cpan-corpus-baseline.json` tracks the cached CPAN top-1000 install as the broad ecosystem receipt. Current committed floor: `8931/9372` clean (`95.3%`), baseline dated 2026-04-09. The install lane reuses `target/cpan-corpus/.cpanm` so reruns ratchet instead of redownloading from scratch.
- **Deterministic regression baseline (`just parser-audit`)**: the repo-owned corpus stays at `91/91` clean, `64/68` NodeKinds covered, `12/12` GA features covered across `test_corpus/` plus `crates/perl-corpus/src/gen`.
- **Strict-clean subsets**: `just common-corpus-check` enforces the pinned common manifest, and `.ci/cpan-corpus-manifest.txt` currently carries `4488` CPAN modules that must stay clean inside `just cpan-corpus-check`.
- **Automation discipline**: the post-merge CPAN workflow now refreshes both the full baseline receipt and the ratcheted manifest, then reruns the CPAN gate before attempting to commit either artifact.
- **Cadence discipline**: the three baselines do not need identical refresh dates, but the committed system-Perl and CPAN top-1000 receipts are now both refreshed through 2026-04-09 after the `v0.12.3` parser/corpus fixes.

## Coverage Baseline Receipts (2026-03-17)

- Path-aware `cargo llvm-cov` workspace summary established a production-code baseline of:
  - `44.7%` lines (`44,200/98,811`)
  - `46.9%` functions (`3,921/8,353`)
  - `42.6%` regions (`68,424/160,806`)
- Tests, benches, examples, `archive/`, and embedded tree-sitter crates excluded

## Index State Machine Receipts (2026-02-16)

`just ci-gate` plus targeted state-machine tests and workspace benchmarks validated transitions, instrumentation, and caps.

---

*Last Updated: 2026-04-09*
