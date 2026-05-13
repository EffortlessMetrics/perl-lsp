# Release Readiness

> Human-owned. Edit this file to update release call and blocker status.
> Do **not** add `<!-- BEGIN: -->` markers — this file is narrative only.

## Current Release Call

**Current release train**: `v0.14.0` public-alpha patch prep
**Workspace version line**: `v0.14.0`
**Published crate surface**: 31 crates
**Release target**: channel verification across GitHub Releases, crates.io, Docker, VS Code Marketplace, Open VSX, and the owned Homebrew tap
**Ship readiness**: release dispatch is intentionally pending. Run the release-prep checks, then use release orchestration only after the prep PR is merged.

## Active Blockers

- No known install-surface blocker after the Homebrew, GNU/musl, installer, and VS Code managed-binary guards landed
- Remaining work is operational: finish `v0.14.0` prep verification, then publish and record final channel receipts

## 0.14.0 Prep Receipts (2026-05-12)

- Release notes file: `docs/releases/0.14.0.md`
- Changelog entry: `CHANGELOG.md` `[0.14.0]`
- Version surfaces: workspace crates, feature catalog metadata, and VS Code extension package staged at `0.14.0`
- Required pre-dispatch checks: version sync, release-history gating, release surface verification, release artifact checks, and install/receipts coverage (see [0.14.0 readiness queue](../../releases/0.14.0-readiness.md))

## 0.13.2 Prep Receipts (2026-05-02)

- Release notes file: `docs/releases/v0.13.2.md`
- Changelog entry: `CHANGELOG.md` `[0.13.2]`
- Version surfaces: workspace crates, feature catalog metadata, and VS Code extension package staged at `0.13.2`
- Required pre-dispatch checks: install-surface check, release-history check, installer target-selection self-test, release-note chooser extraction, and version sync

## 0.13.1 Release Receipts (2026-05-01)

- Release-channel hardening landed in `#7676`: Marketplace-safe version handling, independent Open VSX publishing, explicit token/channel status, and CI timeout classification
- Homebrew tap targeting landed in `#7781`: owned `EffortlessMetrics/homebrew-tap`, `Formula/perllsp.rb`, `class Perllsp`, strict release asset validation, and `brew install effortlessmetrics/tap/perllsp`
- Release naming/docs landed in `#7782`: `docs/releases/v0.13.1.md`, concise changelog entry, and public-alpha wording
- Version bump landed in `#7791`: workspace and package surfaces moved to `0.13.1`; release checks reported 32 published crates
- Release orchestration run `25209777861` created tag `v0.13.1` at `6ef20484` and dispatched GitHub Release, crates.io, extension, and Docker workflows
- crates.io publish run `25209810591` completed and verified all 32 crates at `0.13.1`
- GitHub Release run `25209810124` published `v0.13.1` binaries, `SHA256SUMS`, and SBOM; the VSIX was attached after the release object existed
- VS Code Marketplace and Open VSX publish jobs completed independently; the failed attach job exposed a missing `GH_REPO` environment setting and is patched in the follow-up release-hygiene PR
- Homebrew manual bump run `25210359468` reached the owned-tap path but exposed the same missing `GH_REPO` setting for release asset downloads outside the source checkout; the follow-up release-hygiene PR patches that before rerunning the tap bump
- The first patched Homebrew run validated the `v0.13.1` release asset layout
  and checksums, then stopped because `HOMEBREW_TAP_TOKEN` was not configured;
  subsequent tap hardening configured the token, routed formula generation
  through `cargo xtask update-homebrew`, and added public tap smoke coverage for
  the owned `effortlessmetrics/tap/perllsp` path

## Historical 0.12.3 Ship Receipts (2026-04-09)

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

- **Compatibility baseline (`just corpus-sweep-check`)**: Ubuntu system Perl in [`.ci/parser-corpus-baseline.json`](../../../.ci/parser-corpus-baseline.json) is the "does this still parse what ships on a stock Linux box?" receipt. Current counts live in [`parser.md`](parser.md), not this narrative release note.
- **Ecosystem-breadth baseline (`just cpan-corpus-check`)**: [`.ci/cpan-corpus-baseline.json`](../../../.ci/cpan-corpus-baseline.json) tracks the cached CPAN top-1000 install as the broad ecosystem receipt. The install lane reuses `target/cpan-corpus/.cpanm` so reruns ratchet instead of redownloading from scratch. Current counts live in [`parser.md`](parser.md).
- **Deterministic regression baseline (`just parser-audit`)**: the repo-owned corpus spans [`test_corpus/`](../../../test_corpus/) plus [`crates/perl-corpus/src/gen`](../../../crates/perl-corpus/src/gen). Current clean counts, NodeKind coverage, and GA feature coverage live in [`parser.md`](parser.md).
- **Strict-clean subsets**: `just common-corpus-check` enforces the pinned common manifest, and [`.ci/cpan-corpus-manifest.txt`](../../../.ci/cpan-corpus-manifest.txt) carries the CPAN known-clean set checked inside `just cpan-corpus-check`. Current strict-clean counts live in [`parser.md`](parser.md).
- **Automation discipline**: the post-merge CPAN workflow now refreshes both the full baseline receipt and the ratcheted manifest, then reruns the CPAN gate before attempting to commit either artifact.
- **Cadence discipline**: the three baselines do not need identical refresh dates; use [`parser.md`](parser.md) and the [`.ci/`](../../../.ci/) receipts for current baseline dates.

## Coverage Baseline Receipts (2026-03-17)

- Path-aware `cargo llvm-cov` workspace summary established a production-code baseline of:
  - `44.7%` lines (`44,200/98,811`)
  - `46.9%` functions (`3,921/8,353`)
  - `42.6%` regions (`68,424/160,806`)
- Tests, benches, examples, `archive/`, and embedded tree-sitter crates excluded

## Index State Machine Receipts (2026-02-16)

`just ci-gate` plus targeted state-machine tests and workspace benchmarks validated transitions, instrumentation, and caps.

---

*Last Updated: 2026-05-12*
