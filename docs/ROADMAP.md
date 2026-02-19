# Perl Parser Project - Roadmap

> **Canonical**: This is the authoritative roadmap. See `CURRENT_STATUS.md` for computed metrics.
> **Stale roadmaps**: Archived at `docs/archive/roadmaps/`; retrieve from git history if needed.

> **Status (2026-02-17)**: v0.9.1 close-out verification completed with receipts; v1.0.x hardening underway with native DAP preview validated.
>
> **Canonical receipt**: `nix develop -c just ci-gate` must be green before merging.
> **CI** is intentionally optional/opt-in; the repo is local-first by design.

---

## Truth Rules (read this first)

This roadmap describes goals, but any **status claim** must be backed by one of:
- `nix develop -c just ci-gate` output
- `bash scripts/ignored-test-count.sh` output
- A tracked feature matrix / snapshot test (e.g., GA-lock capabilities snapshot)

If a number is not backed by a receipt, it must be labeled **UNVERIFIED** or removed.

**Metrics are computed, not hand-edited.** See:
- [`CURRENT_STATUS.md`](CURRENT_STATUS.md) for LSP coverage, corpus counts, test health
- [`features.toml`](../features.toml) for canonical LSP capability definitions
- `just status-check` to verify derived metrics haven't drifted

**Last verified**: Run `just ci-gate` to get current verification status.

---

## Current State (v0.9.0 → v0.9.1)

| Component | Release Stance | Evidence | Notes |
|-----------|----------------|----------|-------|
| **perl-parser** (v3) | Production | `just ci-gate` | Parser v3, statement tracker + heredocs in place |
| **perl-lexer** | Production | `just ci-gate` | Tokenization stable |
| **perl-corpus** | Production | `just ci-gate` | Regression corpus + mutation hardening inputs |
| **perl-lsp** | Production (advertised subset) | capability snapshots + targeted tests | Advertise only what's tested; keep GA-lock stable |
| **perl-dap** | Preview (Native + Bridge) | `cargo test -p perl-dap --features dap-phase2,dap-phase3` | Native adapter implemented (breakpoints/control-flow/attach paths) with BridgeAdapter interoperability fallback |
| **perl-parser-pest** (v2) | Legacy | N/A | Optional crate; keep out of default gate |
| **Semantic Analyzer** | Phase 2-6 Complete | `just ci-gate` | All NodeKind handlers; full semantic analysis pipeline |

*Only features that pass `ci-gate` or have targeted integration tests count as "Production".*

---

## Now / Next / Later (Summary)

**Now (post v0.9.1 close-out)**
- Keep close-out receipts green (`just ci-gate`, targeted state-machine tests, benchmark checks)
- Publish benchmark outputs under `benchmarks/results/` for durable v1.0.x evidence

**Next (v1.0.0)**
- Stability statement (GA-lock + versioning rules)
- Packaging stance (what ships; supported platforms)
- Benchmark publication with receipts under `benchmarks/results/` (in progress)
- Upgrade notes from v0.8.x → v1.0

**Later (post v1.0)**
- DAP preview -> GA hardening (deeper variables/evaluate fidelity, shim distribution strategy, cross-editor receipts)
- Full LSP 3.18 compliance
- Package manager distribution (Homebrew/apt/etc.)

---

## Component Summary

For current metrics (LSP coverage %, corpus counts, test pass rates), see [CURRENT_STATUS.md](CURRENT_STATUS.md).

| Crate | Version | Status | Purpose |
|-------|---------|--------|----------|
| **perl-parser** | v0.8.8 | Production | Main parser library |
| **perl-lsp** | v0.8.8 | Production | LSP server (see `features.toml` for GA coverage) |
| **perl-lexer** | v0.8.8 | Production | Context-aware tokenizer |
| **perl-corpus** | v0.8.8 | Production | Test corpus (see `just status-check` for counts) |
| **perl-dap** | v0.1.0 | Preview (Native + Bridge) | Debug Adapter Protocol (native preview adapter + BridgeAdapter compatibility path) |
| **perl-parser-pest** | v0.8.8 | Legacy | Pest-based parser (maintained) |

---

## Next Releases

### v0.9.0: "Semantic-Ready" Milestone — Release

**Status**: Released (2026-01-18)

**Goal**: A release that external users can try without reading internal docs.

**Completed Deliverables**:

1. **Docs truth pass** ✓
   - README + CURRENT_STATUS + ROADMAP aligned on what's real vs aspirational
   - DAP language corrected to reflect native adapter vs BridgeAdapter
   - All claims linked to computed sources or receipts
   - CI cost tracking documentation added

2. **Release artifacts** ✓
   - Confirmed `cargo install --path crates/perl-lsp` works cleanly
   - Release notes match *advertised* capabilities

3. **Capability contracts** ✓
   - GA-lock snapshot stable
   - New capabilities properly advertised
   - 8 features promoted to GA status (completion_item_resolve, code_action_resolve, code_lens_resolve, workspace_symbol_resolve, will_rename_files, did_rename_files, did_delete_files, workspace_edit)

4. **Corpus gap closure (P0)** ✓
   - Fixtures/tests for missing GA constructs
   - Boundedness tests for hang-risk inputs

5. **Production Hardening** ✓
   - Issue #143 resolved: unwrap/expect enforcement in CI
   - Reduced unwrap/expect count from 512 to 377 (26% reduction)
   - Monotonic DAP sequence numbers
   - Robust base-branch detection

**Exit criteria**:
- `nix develop -c just ci-gate` green on MSRV ✓
- `bash scripts/ignored-test-count.sh` shows BUG=0, MANUAL≤1 ✓
- Release notes generated ✓
- Tag cut ✓

**Metrics** (2026-01-23):
- **LSP Coverage**: 100% (53/53 advertised features from `features.toml`)
- **Protocol Compliance**: 100% (88/88 including plumbing)
- **Test Count**: 601 lib tests (discovered), 2 ignored (tracked debt: 0 bug, 1 manual)
- **Parser Coverage**: ~100% Perl 5 syntax
- **Semantic Analyzer**: Phase 2-6 complete (all NodeKind handlers)

### v0.9.1: Post-Release Optimization (January 2026)

**Status**: Close-Out Complete (receipts captured on 2026-02-16)

**Goal**: Performance optimization, semantic analyzer completion, and refactoring infrastructure.

**Completed Deliverables**:

1. **Semantic Analyzer Phase 2-6** ✓ (PR #389)
   - Complete NodeKind coverage
   - Uninitialized variable detection (PR #396)
   - Type inference improvements

2. **Refactoring Engine** ✓
   - `perform_inline` implementation (PR #391)
   - `perform_move_code` implementation (PR #392)
   - Complete LSP refactoring infrastructure (PR #387)

3. **Performance Optimizations** ✓
   - O(1) symbol lookups (PR #336)
   - Stack-based ScopeAnalyzer (PR #383)
   - Reduced string allocations in parser (PR #367, #372, #368)

4. **LSP Server Enhancements** ✓
   - TCP socket mode (PR #370)
   - Cross-file Package->method resolution (PR #375)
   - Unified position/range types

5. **Security Hardening** ✓
   - Path traversal protection (PR #388)
   - Command injection hardening (PR #332)

6. **DAP Improvements** ✓
   - Async BridgeAdapter with graceful shutdown (PR #369)
   - CLI argument parsing with clap (PR #374)
   - Stdio transport (PR #330)

7. **Test Infrastructure** ✓
   - Comprehensive test corpus (PR #404)
   - Workspace indexing synchronization (PR #394)
   - Syntax highlighting validation (PR #397)

8. **v1.0 Preparation** ✓ (PR #483)
   - Benchmark framework and documentation
   - Code quality improvements
   - Zero-allocation variable lookup (PR #473)
   - Token allocations with Arc<str> (PR #464)
   - Cached built-in function signatures (PR #467)
   - Comprehensive corpus expansion (PR #462)

9. **Additional Security Hardening** ✓
   - DAP evaluate request injection prevention (PR #475)
   - Launch debugger command injection hardening (PR #463)
   - Perlcritic/perltidy argument injection prevention (PR #469)
   - Perldoc lookup injection prevention (PR #466)

10. **VSCode Integration** ✓
    - Markdown descriptions and silent startup (PR #474)
    - Settings with code references (PR #468)
    - Command palette filtering for Perl files (PR #470)

**Close-Out Receipts**:

1. **Index State Machine Verification** ✓
   - Transition and instrumentation tests passed in `perl-workspace-index`
   - Early-exit and transition receipts validated via targeted tests
   - Benchmarks confirm caps with large margin (`~368.7us` initial small, `~721.1us` initial medium, `~212.6us` incremental)

2. **Documentation Cleanup** ✓
   - `cargo test -p perl-parser --features doc-coverage --test missing_docs_ac_tests` passed (25/25)
   - `cargo doc --no-deps -p perl-parser` clean (no rustdoc warnings)
   - LSP compatibility module docs and cross-links aligned to current module layout

**Exit criteria**:
- Index state machine verified with receipts and benchmark caps
- `missing_docs` ratchet clean for perl-parser (baseline 0)
- LSP coverage maintained at 100% ✓
- Tracked test debt ≤ 2 ✓
- Security hardening complete ✓

---

### Not Before v1.0

These items are explicitly deferred:
- Full LSP 3.18 compliance (see CURRENT_STATUS.md for current coverage)
- Benchmark result publication (framework exists, results not committed)
- Package manager distribution (Homebrew, apt, etc.)

---

### v1.0.0: "Boring Promises" (sequence after v0.9.1)

**Goal**: Freeze the surfaces you're willing to support. Close known gaps honestly.

**Deliverables**:

1. **Stability statement**
   - What "GA-lock" means (capabilities + wire protocol invariants)
   - Versioning rules for changes

2. **Packaging stance**
   - What you ship (binaries? crates? both?)
   - Minimum supported platforms (explicit)

3. **Benchmark publication**
   - One canonical benchmark run committed under `benchmarks/results/`
   - Remove "UNVERIFIED" tags where you now have receipts

4. **Gap closing (honest assessment items)**
   - ~~Fix execute_command test failures (7 tests)~~ ✓ Fixed: workspace root security boundary was not set in integration tests
   - ~~Correct `features.toml` DAP maturity levels~~ ✓ Fixed: DAP features are tracked as `maturity = "preview"` with implemented native paths
   - ~~Fix `CURRENT_STATUS.md` DAP narrative~~ ✓ Fixed: now describes native preview + bridge interoperability reality
   - Add E2E LSP smoke test (send initialize→didOpen→completion→shutdown, verify responses)
   - Document Moo/Moose limitations honestly in user-facing docs

**Exit criteria**:
- Capability snapshot + docs aligned
- Benchmarks published
- Upgrade notes exist from v0.8.x → v1.0
- Honest assessment document (`docs/HONEST_ASSESSMENT.md`) exists and is current
- No DAP features marked as GA in `features.toml`

---

### v1.1: Semantic Depth

**Goal**: Close the Moo/Moose gap — the #1 real-world limitation.

**Deliverables**:

1. **Moo/Moose `has` attribute recognition**
   - Semantic analyzer recognizes `has` as a field declaration
   - Hover on Moo attributes shows field type, default, documentation
   - Completion inside `has` blocks suggests type constraints, `is`, `isa`, `default`, etc.

2. **Class::Accessor support**
   - Auto-generated accessor methods visible in completion and go-to-definition

3. **Role composition tracking**
   - `with 'Role'` connects role methods to consuming class
   - Go-to-definition resolves role methods

**Exit criteria**:
- Hover on `has 'name'` in a Moo class returns field information
- Completion inside `has` blocks works
- `features.toml` updated with Moo/Moose semantic coverage

---

### v1.2: DAP Preview -> GA

**Goal**: Harden native debugging from preview to GA without requiring Perl::LanguageServer.

**Deliverables**:

1. **Promote preview breakpoints/inline values** to GA-quality behavior and receipts
2. **Deep variable inspection + evaluate context** in active debugger sessions
3. **Attach stability** across PID and TCP modes with consistent stack/thread semantics
4. **Finalize shim/package strategy** (`Devel::TSPerlDAP` or bundled equivalent) and editor defaults

**Exit criteria**:
- Set breakpoint, step, inspect variables, evaluate expressions — works natively with reliable receipts
- DAP features promoted from `maturity = "preview"` to `maturity = "ga"` where warranted
- VSCode extension debug configuration works out of the box

---

### v2.0: DAP Phase 3 + Polish

**Goal**: Full debugging experience and ecosystem polish.

**Deliverables**:

1. Multi-process attach and child-process tracking
2. Watch expressions and richer evaluation UX
3. Full debug console parity + transcript conformance coverage
4. Pest parser decision: archive to `archive/` or maintain for benchmark comparison
5. Package manager distribution (Homebrew, apt, etc.)

---

## Known Gaps

> For the full honest assessment, see [`docs/HONEST_ASSESSMENT.md`](HONEST_ASSESSMENT.md).

### Resolved (v0.9.1)
- ~~Corpus coverage gaps~~ ✓ Comprehensive corpus (732KB, 78 files, 611+ sections)
- ~~Hang/bounds risks~~ ✓ Budget-protected lexer, recursion limits, fuzz testing
- ~~Execute command test failures~~ ✓ Fixed workspace root security boundary

### Open Gaps
- **DAP is preview, not GA**: Native adapter now covers breakpoint/control-flow/attach foundations, but deep runtime variable/evaluate fidelity and packaging strategy still need hardening. `features.toml` keeps DAP at `maturity = "preview"`.
- **Moo/Moose semantic blindness**: Parser tokenizes correctly but semantic analyzer doesn't understand `has` as field declaration. #1 real-world gap.
- **No E2E LSP smoke test**: All testing is unit/integration; no automated test that starts the server and sends real LSP messages end-to-end.
- **Pest parser orphaned**: Compiles and works but excluded from CI, not used in production, 10-100x slower than v3. Maintained for reference only.

### Known Constraints
- **CI Pipeline**: Issue #211 blocks merge-blocking gates (#210)
- **DAP GA promotion**: Depends on shim distribution decision and cross-editor native debugging receipts

---

## v0.9.0 Blockers (Historical; resolved)

> **Historical**: These were the blockers before the v0.9.0 release (2026-01-18). For current blockers, see [MILESTONES.md](MILESTONES.md).

These issues had to resolve before the v0.9.0 release. Listed in dependency order:

| Order | Issue | Category | Rationale |
| ----- | ----- | -------- | --------- |
| 1 | [#211](https://github.com/EffortlessMetrics/perl-lsp/issues/211) | Trust Surface | CI pipeline cleanup - establishes trusted baseline for enforcement |
| 2 | [#210](https://github.com/EffortlessMetrics/perl-lsp/issues/210) | Enforcement | Merge-blocking gates - depends on #211 for clean CI foundation |
| 3 | [#143](https://github.com/EffortlessMetrics/perl-lsp/issues/143) | Integrity | unwrap()/panic safety - can proceed in parallel with above |

**Dependency rationale**: Trust surface (#211) must be established before enforcement (#210) can be meaningful. Without a clean CI pipeline, merge-blocking gates would be built on unreliable infrastructure. Safety cleanup (#143) is independent and can proceed in parallel with the CI/enforcement work.

---

## Completed Work

See [`CURRENT_STATUS.md`](CURRENT_STATUS.md) for detailed completion history.

**Highlights:**

- Statement Tracker & Heredocs (2025-11-20)
- Semantic Analyzer Phase 1 (2025-11-20)
- Band 1: Semantic Stack Validation (2025-12-27)
- Semantic Analyzer Phase 2-6 Complete (2026-01-21)
- Refactoring Engine: inline + move_code (2026-01-21)
- O(1) Symbol Lookups Optimization (2026-01-21)
- TCP Socket Mode for LSP Server (2026-01-21)
- Security Hardening: path traversal + command injection (2026-01-21)
- v1.0 Preparation: benchmarks + documentation (2026-01-23)
- Performance: zero-allocation lookups + Arc<str> tokens (2026-01-23)
- Security: DAP/perldoc/perlcritic injection hardening (2026-01-23)
- VSCode: improved UX + command filtering (2026-01-23)
- Security: Multi-root workspace path traversal fix (PR #620, 2026-01-28)
- Performance: Scope analysis Rc cloning optimization (PR #621, 2026-01-28)
- VSCode: Organize Imports command + Status Menu (PR #609, 2026-01-28)

---

## LSP Feature Implementation

The LSP compliance table is now auto-generated. Source of truth: `features.toml`

<!-- BEGIN: COMPLIANCE_TABLE -->
| Area | Implemented | Total | Coverage |
|------|-------------|-------|----------|
| debug | 10 | 10 | 100% |
| notebook | 2 | 2 | 100% |
| protocol | 9 | 9 | 100% |
| text_document | 41 | 41 | 100% |
| window | 9 | 9 | 100% |
| workspace | 26 | 26 | 100% |
| **Overall** | **97** | **97** | **100%** |
<!-- END: COMPLIANCE_TABLE -->

**v0.9.0 Metrics**:
- **Advertised GA Coverage**: 100% (53/53 trackable features)
- **Protocol Compliance**: 100% (88/88 including plumbing)
- **Total Cataloged**: 89 features
- **Recent Promotions**: 8 features promoted to GA in v0.9.0

For live metrics, run `just status-check` or see [CURRENT_STATUS.md](CURRENT_STATUS.md).

---

## Benchmarks

**Framework exists; results are not yet published as canonical numbers.**

Until benchmark outputs are committed under `benchmarks/results/`, we do not state performance claims in this roadmap.

To publish:

1. Run benchmark harness: `cargo bench -p perl-parser`
2. Commit `benchmarks/results/<date>-<machine>.json`
3. Update `benchmarks/BENCHMARK_FRAMEWORK.md` with machine + command line

---

## Historical Roadmap

See `docs/archive/roadmaps/` for historical roadmap versions.

Older targets (Q1-Q4 2025, 2026 vision) have been archived. Current focus is v0.9/v1.0 milestones above.

---

## Resources

**Start here:** [`INDEX.md`](INDEX.md) - Routes you to the right doc.

- **[Current Status](CURRENT_STATUS.md)** - Computed metrics (the only place with numbers)
- **[features.toml](../features.toml)** - Canonical capability definitions
- **[LESSONS.md](LESSONS.md)** - What went wrong and what changed

<!-- Last Updated: 2026-02-17 -->
