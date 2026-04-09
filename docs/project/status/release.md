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

## Parser Audit Receipts (2026-03-17)

- `just parser-audit` reports `91/91` repo-corpus files parse cleanly
- `63/68` NodeKinds covered (`92.6%`)
- `12/12` GA features covered
- One remaining `P2` interpolation-heavy hang-risk candidate in `crates/perl-corpus/src/gen/builtins.rs`

## CPAN Baseline Receipts (2026-03-20)

- `.ci/cpan-corpus-baseline.json` holds the full-corpus baseline at `3717/4355` clean (`85.4%`) against the installed CPAN top 1000 distributions
- `.ci/cpan-corpus-manifest.txt` carries `4337` modules in the strict known-clean list (expanded from the `1849` v0.12.0 snapshot via #2981)
- Baseline is current for the 0.12.3 release: zero parser/lexer source changes have landed since the baseline was generated

## Coverage Baseline Receipts (2026-03-17)

- Path-aware `cargo llvm-cov` workspace summary established a production-code baseline of:
  - `44.7%` lines (`44,200/98,811`)
  - `46.9%` functions (`3,921/8,353`)
  - `42.6%` regions (`68,424/160,806`)
- Tests, benches, examples, `archive/`, and embedded tree-sitter crates excluded

## Index State Machine Receipts (2026-02-16)

`just ci-gate` plus targeted state-machine tests and workspace benchmarks validated transitions, instrumentation, and caps.

---

*Last Updated: 2026-04-08*
