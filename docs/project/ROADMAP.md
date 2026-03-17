# Perl Parser Project - Roadmap

> **Canonical**: This is the authoritative roadmap. See `CURRENT_STATUS.md` for computed metrics.
> **Stale roadmaps**: Archived at `docs/archive/roadmaps/`; retrieve from git history if needed.

> **Status (2026-03-17)**: **Public Alpha (v0.12.0)**. Release preparation, parser-quality ratchets, and documentation alignment underway.
>
> **Canonical receipt**: `nix develop -c just ci-gate` must be green before merging.
> **CI** is intentionally optional/opt-in; the repo is local-first by design.

---

## Alpha Disclaimer

Perl LSP is currently in **Public Alpha**. Version 0.12.0 represents a substantially complete feature set, but APIs and protocols are still evolving. We value early adopter feedback to refine the project toward the v0.15.0 Stability Contract milestone.

---

## Current State (v0.12.0)

| Component | Release Stance | Evidence | Notes |
|-----------|----------------|----------|-------|
| **perl-parser** (v3) | Public Alpha | `just ci-gate` | Parser v3, statement tracker + heredocs in place |
| **perl-lexer** | Public Alpha | `just ci-gate` | Tokenization stable |
| **perl-corpus** | Public Alpha | `just ci-gate` | Regression corpus + mutation hardening inputs |
| **perl-lsp** | Public Alpha (advertised subset) | capability snapshots + targeted tests | Evolving feature set |
| **perl-dap** | Preview (Native + Bridge) | `cargo test -p perl-dap --features dap-phase2,dap-phase3` | Native adapter foundations with BridgeAdapter fallback |
| **perl-parser-pest** (v2) | Legacy | N/A | Optional legacy crate |
| **Semantic Analyzer** | Phase 2-6 Complete | `just ci-gate` | Full semantic analysis pipeline |
| **Parser Coverage** | 72.4% system corpus (5,139/7,095) | 90%+ CPAN top 1000 | System ratchet committed; CPAN baseline bootstrap next |

---

## Now / Next / Later (Summary)

**Now (Release Preparation + Parser Quality)**
- Release preparation: crates.io publish validation, VS Code marketplace packaging
- Documentation cleanup and alignment for launch (article series, workspace snapshots)
- Final CI validation and release-blocker fixes (rustdoc, clippy, crate metadata)
- CPAN top-1000 corpus tooling (PR #1469 — `cargo xtask cpan-corpus`)
- Parser fix PRs for Wave 2-4 error buckets
- Merge and ratchet parser coverage baseline
- Seed the first committed CPAN full-corpus baseline (`just cpan-corpus-baseline-update`)
- Keep internal edge-case, parser stress, and hang-risk suites green
- Keep close-out receipts green (`just ci-gate`)

**Next (v0.12.0 exit criteria)**
- CPAN corpus: achieve 90%+ clean parse rate on top 1000 distributions
- Expand common corpus manifest as parser fixes land
- Expand the CPAN known-clean manifest as clean modules accumulate
- Protect system/common corpus ratchets while error buckets shrink
- Internal parser torture coverage: edge-case, stress, and hang-risk suites stay green
- No parser hangs, stack overflows, or crash regressions in corpus sweeps
- Complete Moo/Moose/Class::Accessor attribute resolution (foundation: `requires` tracking and multi-attribute `has` landed in PR #946)
- Cross-file type inference via `use parent`/`use base`
- Native DAP enhancements (variables/evaluate)
- Stability goal refinement: define requirements for v0.15.0 contract

**Later (Targeting v0.15.0 for Stability Contract)**
- **Stability Contract**: Formal API stability and contract-locked wire protocol
- Full LSP 3.18 compliance
- Finalized shim distribution strategy
- Package manager distribution (Homebrew/apt/etc.)

> **v0.12.0 framing**: This milestone is the **Public Alpha Epic Sprint**. Parser coverage, parser boundedness, semantic framework support, and release readiness are being pushed together rather than as isolated tracks.

---

## Component Summary

For current metrics (LSP coverage %, corpus counts, test pass rates), see [CURRENT_STATUS.md](CURRENT_STATUS.md).

| Crate | Version | Status | Purpose |
|-------|---------|--------|----------|
| **perl-parser** | v0.12.0 | Public Alpha | Main parser library |
| **perl-lsp** | v0.12.0 | Public Alpha | LSP server |
| **perl-lexer** | v0.12.0 | Public Alpha | Context-aware tokenizer |
| **perl-corpus** | v0.12.0 | Public Alpha | Test corpus |
| **perl-dap** | v0.12.0 | Preview (Native + Bridge) | Debug Adapter Protocol |
| **perl-parser-pest** | v0.12.0 | Legacy | Pest-based parser (maintained) |

---

## Future Milestone: v0.15.0 Stability Contract

When the project reaches **v0.15.0**, we will establish a formal **Stability Contract**:

1. **API Stability**: Public APIs in published crates will follow strict Semantic Versioning.
2. **Protocol Invariants**: LSP capabilities will be contract-locked for reliable client integration.
3. **Deprecation Policy**: Formal multi-release deprecation cycles for any breaking changes.
4. **Platform Commitment**: Guaranteed support tiers for major operating systems.

---

## LSP Feature Implementation

The LSP compliance table is auto-generated from `features.toml`.

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

> **Note:** All 97 features are implemented (maturity: GA). Of these, 96/97 are advertised to clients;
> `lsp.notebook_cell_execution` is implemented but not advertised. See `features.toml` for details.

For live metrics, run `just status-check` or see [CURRENT_STATUS.md](CURRENT_STATUS.md).

---

## Completed Work

See [`CURRENT_STATUS.md`](CURRENT_STATUS.md) for detailed completion history.

**Highlights:**
- Project began (forked from `tree-sitter-perl-better` July 15, 2025) as a validation harness, evolved into native Rust implementation.
- Statement Tracker & Heredocs (2025-11-20)
- Semantic Analyzer Phase 1 (2025-11-20)
- Semantic Analyzer Phase 2-6 Complete (2026-01-21)
- Refactoring Engine: inline + move_code (2026-01-21)
- Security Hardening: path traversal + command injection (2026-01-21)
- v0.10.0 Initial Public Alpha Preparation (2026-02-28)
- Moo/Moose `requires` tracking and multi-attribute `has` (PR #946, merged 2026-03)
- SRP microcrate extractions: dead-code (#945), lsp-limits (#934), capability-mapping (#950), subprocess-runtime (#953) -- all merged
- Feature governance extracted into 9 microcrates (PR #848)
- Additional SRP extractions: workspace-symbol provider (#1237), line-index (#1234), folding (#1238), import-management (#1239, #1242) -- all merged
- Launch article series: parser evolution, workspace architecture, LSP implementation, quality infrastructure, agentic development history, and more (2026-03)
- Release-prep fixes: crates.io metadata, rustdoc warnings, clippy blockers, VS Code marketplace packaging (2026-03)

---

## Resources

**Start here:** [`INDEX.md`](INDEX.md) - Routes you to the right doc.

- **[Current Status](CURRENT_STATUS.md)** - Computed metrics
- **[features.toml](../features.toml)** - Canonical capability definitions
- **[LESSONS.md](LESSONS.md)** - Project learnings

<!-- Last Updated: 2026-03-17 -->

## Detailed Forward-Looking Roadmap

### v0.11.0: Advanced Semantic Engine
- **Goal:** Deepen semantic understanding of complex Perl constructs.
- **Features:**
  - Full Moo/Moose/Class::Accessor attribute resolution. *(Foundation landed: `requires` tracking and multi-attribute `has` merged in PR #946; `Class::Accessor` not yet started.)*
  - Cross-file type inference across standard import mechanisms (`use parent`, `use base`).
  - Improved bareword disambiguation based on export lists.
  - Constant folding and compile-time evaluation approximations.

### v0.12.0: Public Alpha Epic Sprint

- **Goal:** Make the alpha credible on real-world Perl by raising clean-parse coverage, keeping ratchets green, proving bounded behavior on pathological inputs, and covering the mainstream Moo/Moose-style semantic surface users expect.
- **Features:**
  - CPAN top-1000 corpus sweeps with ratchet-only-forward enforcement. See [CPAN_CORPUS_STRATEGY.md](CPAN_CORPUS_STRATEGY.md).
  - Parser Wave 2-4 fixes prioritized by first-error-per-file analysis. See [PARSER_EDGE_CASE_ROADMAP.md](PARSER_EDGE_CASE_ROADMAP.md).
  - Common corpus promotion pipeline so clean modules become permanent zero-error commitments.
  - Internal parser torture coverage spanning edge-case fixtures, parser stress cases, and hang-risk suites.
  - Moo/Moose/Class::Accessor attribute handling, including maintained coverage for `has`, `requires`, and common attribute forms.
  - Cross-file type and inheritance inference via `use parent` / `use base`, plus export-list-driven bareword disambiguation.
  - Release hardening: keep `just ci-gate` green while parser quality improves.
- **Sprint Tracks:**
  - **Corpus and ratchets**: `just corpus-sweep`, `just corpus-sweep-check`, `just cpan-corpus-baseline-update`, `just cpan-corpus-sweep`, `just cpan-corpus-check`, and `just cpan-corpus-ratchet`.
  - **Parser robustness**: Wave 2-4 fixes, recovery improvements, and boundedness work proven against `cargo xtask test-edge-cases` plus hang-risk suites.
  - **Semantic frameworks**: Moo/Moose/Class::Accessor coverage, inheritance resolution, and export-list-aware disambiguation.
  - **Release readiness**: docs, packaging, and `nix develop -c just ci-gate` stay green while the parser work lands.
- **Exit Criteria:**
  - 90%+ of `.pm` files in the CPAN top-1000 corpus parse with zero `ERROR` nodes.
  - `.ci/cpan-corpus-baseline.json` is committed so the CPAN lane has a real ratchet floor.
  - Common corpus remains strict zero-error and only grows.
  - CPAN known-clean manifest grows from `.ci/cpan-corpus-manifest.txt` without regressions once seeded.
  - System corpus ratchet shows no regressions in crash count, unreadable files, clean-file count, total `ERROR` nodes, or per-bucket counts.
  - Internal edge-case, parser stress, and hang-risk suites pass with no hang, stack-overflow, or infinite-loop regressions.
  - Moo attribute resolution covers the maintained test corpus; Moose/Class::Accessor support handles core attribute and inheritance flows used in real code.
  - Exporter/Sub::Exporter-style export lists improve bareword disambiguation for semantic analysis and navigation.
  - Release receipts remain green during the hardening push.

### v0.13.0: Complete Refactoring Suite
- **Goal:** Safe, reliable automated code modification.
- **Features:**
  - Safe rename across entire workspaces with boundary detection.
  - Extract Method / Extract Variable refactorings.
  - Inline Method / Inline Variable refactorings.
  - Automated translation of older constructs to modern Perl 5.38+ syntax.

### v0.14.0: Native Debugging Excellence
- **Goal:** A first-class native debugging experience.
- **Features:**
  - Fully stabilized Native DAP replacing the bridge entirely.
  - Conditional breakpoints and logpoints evaluated without blocking the debugger.
  - Rich variable inspection with support for complex nested data structures (e.g., blessed references, tied variables).
  - Multi-process / fork-aware debugging.

### v0.15.0: The Stability Contract
- **Goal:** Enterprise-ready stability guarantees.
- **Deliverables:**
  - 1.0.0 semantic versioning applied to public APIs.
  - Contract-locked LSP features.
  - Formal deprecation policy (N-2 release support minimum).
  - Certified support for Linux, macOS, and Windows.
