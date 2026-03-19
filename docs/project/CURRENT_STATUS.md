# perl-lsp Current Status

> **Truth contract**: All claims require evidence from:
> - `Cargo.toml` (`workspace.package.version`) for the current release line
> - `nix develop -c just ci-gate` output
> - `bash scripts/ignored-test-count.sh` output
> - `features.toml`, capability snapshots, or targeted tests

---

## What Belongs Here

- This file is the evidence document.
- [ROADMAP.md](ROADMAP.md) is the planning document.
- Generated sections between `<!-- BEGIN: ... -->` and `<!-- END: ... -->` are machine-updated by `just status-update`. Do not hand-edit those blocks.

## Verification Protocol

**Tier A: Merge Gate** (required for all merges)
```bash
just ci-gate  # ~2-5 min
```

**Tier B: Release Confidence** (large changes or release candidates)
```bash
just ci-full  # ~10-20 min
```

**Tier C: Real User Confirmation**
Manual editor smoke test: diagnostics, completion, hover, go-to-definition, rename

### Metric Definitions

**LSP Metrics** (computed from `features.toml` by `scripts/update-current-status.py`):

| Metric | Formula | Meaning |
| --- | --- | --- |
| **LSP Coverage (user-visible)** | `implemented / trackable` where `counts_in_coverage != false` | Headline metric |
| **Protocol Compliance** | `implemented / trackable` (all features) | Wire-level completeness |

Key terms:

- `implemented` (coverage): Features with `maturity in (ga, production)`
- `trackable` (coverage): Features where `advertised = true`, `maturity != planned`, and `counts_in_coverage != false`
- `implemented` (protocol): Features with `maturity in (ga, production, preview)`
- `trackable` (protocol): Features where `maturity != planned`
- `counts_in_coverage = false`: Protocol plumbing that would otherwise inflate coverage artificially

**Other Metrics**:

- **Corpus counts**: `tree-sitter-perl/test/corpus` sections + `test_corpus/*.pl` files
- **Catalog source**: root `features.toml` is canonical

---

## At a Glance

| Metric | Value | Target | Status |
| --- | --- | --- | --- |
| **Current release line** | `v0.11.0` public alpha | Truthful docs and receipts | Active |
| **Active milestone** | `v0.12.0` public-alpha hardening sprint | Exit hardening sprint cleanly | In progress |
| **Merge gate** | `nix develop -c just ci-gate` | Green before merge | Required |
| **Tier A Tests** | 2244 lib tests (discovered), 0 ignores (tracked) | 100% pass | PASS |
| **Tracked Test Debt** | 0 (0 bug, 0 manual) | 0 | Near-zero |
<!-- BEGIN: STATUS_METRICS_TABLE -->
| **LSP Coverage** | 100% (53/53 advertised features, `features.toml`) | 100% | PASS |
<!-- END: STATUS_METRICS_TABLE -->
| **Parser hardening** | CPAN baseline, repo corpus, and hang-risk receipts tracked below | 90%+ CPAN clean next | Active |
| **DAP stance** | Native + Bridge preview | Harden preview flows | Active |
| **Documentation** | perl-parser missing_docs = 0 (baseline 0) | 0 | Ratchet |

---

## What's True Right Now

- **Release posture**: the current release line is `v0.11.0` public alpha; the active milestone is `v0.12.0` hardening, not a shipped release
- **Status discipline**: this file is for evidence, [ROADMAP.md](ROADMAP.md) is for planning, and `just status-update` plus `just status-check` are the anti-drift workflow
- **LSP server**: `features.toml` is the canonical capability catalog; computed coverage is generated from it
- **Test infrastructure**: `nix develop -c just ci-gate` is the canonical merge receipt and `bash scripts/ignored-test-count.sh` is the tracked-test-debt source
- **Parser stack**: the default parser path is the native recursive-descent stack backed by the Rust lexer and parser-core crates
- **Refactoring engine**: inline and move-code flows exist; broader refactoring hardening is still roadmap work
- **Safety ratchets**: production baseline currently at `unwrap/expect=0`, panic-family macros (`panic!/todo!/unimplemented!/unreachable!`) = `0`, explicit `unsafe` syntax = `0`
- **Security**: hardening exists for path traversal, command injection, DAP evaluate, and perldoc/perlcritic argument injection
- **Parser audit receipts (2026-03-17)**: `just parser-audit` reports `91/91` repo-corpus files parse cleanly, `63/68` NodeKinds covered (`92.6%`), `12/12` GA features covered, and one remaining `P2` interpolation-heavy hang-risk candidate in `crates/perl-corpus/src/gen/builtins.rs`
- **CPAN baseline receipts (2026-03-17)**: `just cpan-corpus-check` holds the committed baseline at `3139/4355` clean (`72.1%`) for the full installed corpus and `1579/1579` clean for the strict known-clean manifest
- **Coverage baseline receipts (2026-03-17)**: a path-aware `cargo llvm-cov` workspace summary established a production-code baseline of `44.7%` lines (`44,200/98,811`), `46.9%` functions (`3,921/8,353`), and `42.6%` regions (`68,424/160,806`) with tests, benches, examples, `archive/`, and embedded tree-sitter crates excluded
- **DAP server**: the native adapter preview is implemented and the BridgeAdapter path remains available for Perl::LanguageServer interoperability
- **Index state machine receipts (2026-02-16)**: `just ci-gate` plus targeted state-machine tests and workspace benchmarks validated transitions, instrumentation, and caps

### Computed Metrics (auto-updated by `just status-update`)

<!-- BEGIN: STATUS_METRICS_BULLETS -->
- **LSP Coverage**: 100% user-visible feature coverage (53/53 advertised features from `features.toml`)
- **Protocol Compliance**: 100% overall LSP protocol support (97/97 including plumbing)
- **Parser Coverage**: ~100% Perl 5 syntax via `tree-sitter-perl/test/corpus` (~611 sections) + `test_corpus/` (73 `.pl` files)
- **Test Status**: 2244 lib tests (Tier A), 0 ignores tracked (0 total tracked debt: 0 bug, 0 manual)
- **Docs (perl-parser)**: missing_docs warnings = 0 (baseline 0)
- **Quality Metrics**: 87% mutation score, <50ms LSP response times, 931ns incremental parsing
- **Production Status**: LSP server public alpha (`just ci-gate` passing)

**Target**: maintain 100% LSP coverage (no regressions)
<!-- END: STATUS_METRICS_BULLETS -->

---

## What's Next

**Now (active milestone: v0.12.0 hardening sprint on top of the v0.11.0 release line)**
- Raise the CPAN top-1000 full-corpus baseline from `72.1%` (`3139/4355`) to `90%+` clean parses while keeping the strict known-clean manifest at `100%`
- Close repo-corpus coverage gaps (`63/68` NodeKinds currently covered) and retire the remaining parser audit `P2` hang-risk candidate
- Land Moo/Moose/Class::Accessor, `use parent`/`use base`, and export-list disambiguation work needed for public-alpha expectations
- Raise workspace production-code coverage from the new baseline of `44.7%` lines / `46.9%` functions / `42.6%` regions
- Burn down the residual coverage-gate blockers in `perl-parser` control-flow tests and `tree-sitter-perl-rs` parser/heredoc/glob suites
- Keep README, roadmap, and agent guidance aligned with the actual release line and evidence sources

**Next (v0.12.x hardening)**
- Ratchet system-corpus and CPAN baselines as parser coverage improves
- Fold internal torture and edge-case suites into routine verification receipts
- Publish benchmark and release-readiness receipts for the alpha burndown

**Later (post v0.12.x)**
- DAP preview hardening (deeper live variables/evaluate, shim packaging, cross-editor native receipts)
- Full LSP 3.18 compliance
- Broader distribution packaging

See [ROADMAP.md](ROADMAP.md) for milestone details.

---

## Known Constraints

- **Tracked test debt**: see `scripts/ignored-test-count.sh`; feature-gated ignores are by design
- **Docs scope**: `perl-parser` `missing_docs` is ratcheted; workspace-wide enforcement is a separate decision
- **Coverage scope**: the workspace baseline intentionally excludes tests, benches, examples, `archive/`, and embedded tree-sitter crates
- **Coverage gate**: `just coverage-summary` still depends on residual workspace test failures found during the March 17 sweep
- **Index state machine**: verification receipts are captured separately and summarized above

## Component Summary

| Component | Status | Notes |
| --- | --- | --- |
| `perl-parser` | Public alpha | Native parser path |
| `perl-lsp` | Public alpha | Coverage tracked via `features.toml` |
| `perl-dap` | Preview (Native + Bridge) | Native adapter is present; compatibility path retained |
| `perl-lexer` | Public alpha | Context-aware tokenizer |
| `perl-corpus` | Public alpha | Corpus counts tracked in computed metrics |

## How to Update This File

1. Run `just status-update` to regenerate computed metrics
2. Run `just status-check` to verify the generated sections are current
3. Run `just ci-gate` to verify the repo-level receipt still passes
4. Keep the current release line and next milestone aligned with `Cargo.toml` and [ROADMAP.md](ROADMAP.md)
5. Edit narrative sections only after the evidence is current

**Historical archives**: see `docs/archive/status_snapshots/` for sprint logs and completion history.

---

*Last Updated: 2026-03-19 (narrative sections only; run `just status-update` to refresh metrics)*
*Canonical docs: [ROADMAP.md](ROADMAP.md), [../../features.toml](../../features.toml)*
