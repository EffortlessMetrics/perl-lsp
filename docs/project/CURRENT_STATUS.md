# perl-lsp Current Status

> **Truth contract**: All claims require evidence from:
> - `nix develop -c just ci-gate` output
> - `bash scripts/ignored-test-count.sh` output
> - Capability snapshots or targeted tests

---

## Verification Protocol

**Tier A: Merge Gate** (required for all merges)
```bash
just ci-gate  # ~2-5 min
```

**Tier B: Release Confidence** (large changes/release candidates)
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
- `trackable` (protocol): Features where `maturity != planned` (excludes future work)
- `counts_in_coverage = false`: Protocol plumbing (lifecycle, sync) that inflates coverage artificially

**Other Metrics**:

- **Corpus counts**: `tree-sitter-perl/test/corpus` sections + `test_corpus/*.pl` files (fixture counts)
- **Catalog source**: Root `features.toml` is canonical

**Generated Sections**: Blocks between `<!-- BEGIN: X -->` and `<!-- END: X -->` are machine-updated by `just status-update`. Do not hand-edit.

---

## At a Glance

| Metric | Value | Target | Status |
| --- | --- | --- | --- |
| **Tier A Tests** | 2400 lib tests (discovered), 0 ignores (tracked) | 100% pass | PASS |
| **Tracked Test Debt** | 0 (0 bug, 0 manual) | 0 | Near-zero |
<!-- BEGIN: STATUS_METRICS_TABLE -->
| **LSP Coverage** | 100% (53/53 advertised features, `features.toml`) | 100% | PASS |
<!-- END: STATUS_METRICS_TABLE -->
| **Parser Coverage** | ~100% | 100% | Complete |
| **Semantic Analyzer** | Phase 1, 2, 3 Complete (100%) | Complete | All NodeKind handlers |
| **Mutation Score** | 87% | 90%+ | Ratchet to 90% |
| **Documentation** | perl-parser missing_docs = 0 (baseline 0) | 0 | Ratchet |

---

## What's True Right Now

- **Parser**: Perl 5 syntax coverage, 1-150us parsing, 931ns incremental updates
- **LSP Server**: Capability catalog is `features.toml`; Tier A gate is `just ci-gate`; TCP socket mode available
- **Semantic Analyzer**: Phase 1, 2, 3 complete with all NodeKind handlers (100% AST node coverage), `textDocument/definition` integrated, uninitialized variable detection
- **Refactoring Engine**: `perform_inline` and `perform_move_code` implemented
- **Test Infrastructure**: Tier A suite is the only merge-blocking truth (see At a Glance + computed metrics)
- **Quality**: 87% mutation score, comprehensive UTF-16 handling, path validation, O(1) symbol lookups, zero-allocation variable lookups
- **Safety Ratchets**: production baseline currently at `unwrap/expect=0`, panic-family macros (`panic!/todo!/unimplemented!/unreachable!`) = `0`, explicit `unsafe` syntax = `0`
- **Security**: Comprehensive hardening complete (path traversal, command injection, DAP evaluate, perldoc/perlcritic argument injection)
- **Parser Audit Receipts (2026-03-17)**: `just parser-audit` reports `91/91` repo-corpus files parse cleanly, `63/68` NodeKinds covered (`92.6%`), `12/12` GA features covered, and one remaining `P2` interpolation-heavy hang-risk candidate in `crates/perl-corpus/src/gen/builtins.rs`
- **CPAN Baseline Receipts (2026-03-17)**: `just cpan-corpus-check` holds the committed baseline at `3139/4355` clean (`72.1%`) for the full installed corpus and `1579/1579` clean for the strict known-clean manifest
- **Coverage Baseline Receipts (2026-03-17)**: a path-aware `cargo llvm-cov` workspace summary established a production-code baseline of `44.7%` lines (`44,200/98,811`), `46.9%` functions (`3,921/8,353`), and `42.6%` regions (`68,424/160,806`) with tests, benches, examples, `archive/`, and embedded tree-sitter crates excluded
- **DAP Server**: Native adapter preview is implemented (breakpoints with AST validation via `perl-dap-breakpoint`, step/pause/continue handlers, safe-eval guards, stdio+socket transport, PID/TCP attach modes); BridgeAdapter remains available for Perl::LanguageServer interoperability
- **Index State Machine Receipts (2026-02-16)**: `just ci-gate` + targeted state-machine tests and workspace benchmarks validated transitions, instrumentation, and caps (`~368.7us` initial small index, `~721.1us` initial medium index, `~212.6us` incremental update)

### Computed Metrics (auto-updated by `just status-update`)

<!-- BEGIN: STATUS_METRICS_BULLETS -->
- **LSP Coverage**: 100% user-visible feature coverage (53/53 advertised features from `features.toml`)
- **Protocol Compliance**: 100% overall LSP protocol support (97/97 including plumbing)
- **Parser Coverage**: ~100% Perl 5 syntax via `tree-sitter-perl/test/corpus` (~611 sections) + `test_corpus/` (73 `.pl` files)
- **Test Status**: 2400 lib tests (Tier A), 0 ignores tracked (0 total tracked debt: 0 bug, 0 manual)
- **Docs (perl-parser)**: missing_docs warnings = 0 (baseline 0)
- **Quality Metrics**: 87% mutation score, <50ms LSP response times, 931ns incremental parsing
- **Production Status**: LSP server public alpha (`just ci-gate` passing)

**Target**: maintain 100% LSP coverage (no regressions)
<!-- END: STATUS_METRICS_BULLETS -->

---

## What's Next

**Now (v0.12.0 Public Alpha Epic Sprint)**
- Raise the CPAN top-1000 full-corpus baseline from `72.1%` (`3139/4355`) to `90%+` clean parses while keeping the strict known-clean manifest at `100%`
- Close repo-corpus coverage gaps (`63/68` NodeKinds currently covered) and retire the remaining parser audit `P2` hang-risk candidate
- Land Moo/Moose/Class::Accessor, `use parent`/`use base`, and export-list disambiguation work needed for public-alpha expectations
- Raise workspace production-code coverage from the new baseline of `44.7%` lines / `46.9%` functions / `42.6%` regions
- Burn down the residual coverage-gate blockers in `perl-parser` control-flow tests and `tree-sitter-perl-rs` parser/heredoc/glob suites

**Next (v0.12.x hardening)**
- Ratchet system-corpus and CPAN baselines as parser coverage improves
- Fold internal torture/edge-case suites into routine verification receipts
- Publish benchmark and release-readiness receipts for the alpha burndown

**Later (post v0.12.x)**
- DAP preview hardening (deeper live variables/evaluate, shim packaging, cross-editor native receipts)
- Full LSP 3.18 compliance
- Package manager distribution (Homebrew/apt/etc.)

See [ROADMAP.md](ROADMAP.md) for milestone details.

---

## Known Constraints

- **Tracked test debt**: see `scripts/ignored-test-count.sh`; feature-gated ignores are by design
- **CI Pipeline (#211)**: Blocks merge-blocking gates (#210)
- **Docs scope**: perl-parser missing_docs is ratcheted (see `ci/check_missing_docs.sh`); workspace-wide enforcement is a separate decision
- **Coverage scope**: the workspace baseline intentionally excludes tests, benches, examples, `archive/`, and embedded tree-sitter crates so `just coverage-summary` measures production code instead of harnesses
- **Coverage gate**: `just coverage-summary` still depends on residual workspace test failures found during the March 17 sweep: `perl-parser` (`nodekind_combination_control_flow`), `tree-sitter-perl-rs` (`parser_tests`, `pure_rust_parser_tests`, `special_context_heredoc_tests`, `test_missing_edge_cases`, `test_real_world_heredocs`), plus a live long-run/hang-risk in `tree-sitter-perl-rs` `test_stacker_fix` (>17 minutes in a plain workspace sweep)
- **Index State Machine**: Verification complete (2026-02-16 receipts captured with `just ci-gate` + targeted tests/benchmarks)

---

## Component Summary

| Component | Status | Notes |
| --- | --- | --- |
| perl-parser | Public Alpha | ~100% Perl 5, 87% mutation score |
| perl-lsp | Public Alpha | Coverage tracked via `features.toml` |
| perl-dap | Preview (Native + Bridge) | Native adapter implemented/tested (phase2+phase3 suites); BridgeAdapter retained for compatibility |
| perl-lexer | Public Alpha | Context-aware, sub-microsecond |
| perl-corpus | Public Alpha | Corpus counts tracked in computed metrics |

---

## How to Update This File

1. Run `just status-update` to regenerate computed metrics
2. Run `just ci-gate` to verify all gates pass
3. Edit "What's True Right Now" and "What's Next" sections as needed

**Historical archives**: See `docs/archive/status_snapshots/` for sprint logs and completion history.

---

*Last Updated: 2026-03-17 (narrative sections only; run `just status-update` to refresh metrics)*
*Canonical docs: [ROADMAP.md](ROADMAP.md), [features.toml](../features.toml)*
