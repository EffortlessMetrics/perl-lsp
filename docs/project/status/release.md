# Release Readiness

> Human-owned. Edit this file to update release call and blocker status.
> Do **not** add `<!-- BEGIN: -->` markers — this file is narrative only.

## Current Release Call

**Latest release candidate**: `v0.13.0-rc1` (2026-04-30)
**crates.io line**: `0.13.0-rc1` across 32 published crates
**Release target**: `v0.13.0` public alpha
**Ship readiness**: RC1 validated GitHub Releases, crates.io, and Docker Hub. Public alpha release is pending Marketplace-compatible extension versioning, independent Open VSX publish, docs/version finalization, and one final release verification receipt.

## Active Blockers

- VS Marketplace must publish the non-prerelease extension version `0.13.0`, not prerelease `0.13.0-rc1`
- Open VSX must run independently rather than cascading behind Marketplace
- Master needs one clean release-verification cycle before tagging `v0.13.0`

## 0.13.0-rc1 Ship Receipts (2026-04-30)

- GitHub release `v0.13.0-rc1` published with cross-platform `perllsp`/`perl-dap` archives and `SHA256SUMS`
- crates.io published all 32 crates listed in `[workspace.metadata.publish.allow]`, including `perl-semantic-facts`, at `0.13.0-rc1`
- Docker Hub published multi-arch images for the RC
- VS Marketplace rejected the prerelease suffix; non-prerelease `0.13.0` is the Marketplace publish version
- Open VSX was skipped because it was sequenced behind Marketplace; the `0.13.0` path must report Open VSX separately

## Component Summary

| Component | Status | Notes |
| --- | --- | --- |
| `perl-parser` | Public alpha 0.13.0 | Native parser path |
| `perl-lsp` | Public alpha 0.13.0 | Coverage tracked via `features.toml` |
| `perl-dap` | Preview (Native + Bridge) | Native adapter is present; compatibility path retained |
| `perl-lexer` | Public alpha 0.13.0 | Context-aware tokenizer |
| `perl-corpus` | Public alpha 0.13.0 | Corpus counts tracked in computed metrics |

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

*Last Updated: 2026-05-01*
