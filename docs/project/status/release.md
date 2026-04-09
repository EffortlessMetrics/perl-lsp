# Release Readiness

> Human-owned. Edit this file to update release call and blocker status.
> Do **not** add `<!-- BEGIN: -->` markers — this file is narrative only.

## Current Release Call

**Latest published release**: `v0.12.2` (2026-04-07)
**Release target**: `v0.12.3` pipeline-rehearsal cut
**Ship readiness**: the 11-PR launch-prep drain merged cleanly on 2026-04-08, master carries the full publish/UX/CI hardening wave, and the only remaining explicit launch gate is the `#3302` demo-assets recording before the `v0.13.0` public alpha announcement

## Active Blockers

- `#3302` demo GIFs are the only human-owned blocker before the `v0.13.0` public alpha announcement
- Public install guidance must stay tied to the crates.io truth at release time, not just the local workspace version line

## 0.12.3 Pipeline-Rehearsal Receipts (2026-04-08)

- `v0.12.2` is live on GitHub Releases and crates.io as of 2026-04-07, post-publish smoke test green
- 11 launch-prep PRs merged in a controlled drain sequence on 2026-04-08 (master green at `a5680401`)
- workspace version line bumped to `v0.12.3`; `check-version-sync` reports all 140 sites in agreement
- `CHANGELOG.md` already carries the `[0.12.3]` entry; tagging is a `git tag` away

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

- **Ubuntu system Perl baseline (`.ci/parser-corpus-baseline.json`, 2026-04-09)**: `6866/7095` clean (`96.8%`) on Perl `5.038002`; refreshed after fixing the `unexpected_fat_arrow_expr` and `unexpected_slash_expr` corpus regressions
- **CPAN top 1000 baseline (`.ci/cpan-corpus-baseline.json`, 2026-03-20)**: `3717/4355` clean (`85.4%`) against the installed top-1000 corpus; the install lane now reuses `target/cpan-corpus/.cpanm` so refreshes do not redownload from scratch
- **Repo-owned corpus (`just parser-audit` / `status/parser.md`)**: `91/91` clean, `64/68` NodeKinds covered, `12/12` GA features covered across `test_corpus/` plus `crates/perl-corpus/src/gen`
- `.ci/cpan-corpus-manifest.txt` carries `4337` modules in the strict known-clean list (expanded from the `1849` v0.12.0 snapshot via #2981)
- The committed baselines now reflect two refresh cadences for the 0.12.3 release posture: Ubuntu system Perl was rerun on 2026-04-09 after parser fixes, while the CPAN top-1000 receipt remains the 2026-03-20 baseline until the full install lane is rerun

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
