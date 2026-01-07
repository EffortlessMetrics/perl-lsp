# Perl Parser Project - Roadmap

> **Canonical**: This is the authoritative roadmap. See `CURRENT_STATUS.md` for computed metrics.
> **Stale roadmaps**: Archived at `docs/archive/roadmaps/`; retrieve from git history if needed.

> **Status (2026-01-07)**: Validation + packaging phase toward v0.9.0 ("semantic-ready") and v1.0.
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

## Current State (v0.8.8 → v0.9.0)

| Component | Release Stance | Evidence | Notes |
|-----------|----------------|----------|-------|
| **perl-parser** (v3) | Production | `just ci-gate` | Parser v3, statement tracker + heredocs in place |
| **perl-lexer** | Production | `just ci-gate` | Tokenization stable |
| **perl-corpus** | Production | `just ci-gate` | Regression corpus + mutation hardening inputs |
| **perl-lsp** | Production (advertised subset) | capability snapshots + targeted tests | Advertise only what's tested; keep GA-lock stable |
| **perl-dap** | Experimental (bridge mode) | manual smoke | Bridges to Perl::LanguageServer; not "full" native DAP |
| **perl-parser-pest** (v2) | Legacy | N/A | Optional crate; keep out of default gate |
| **Semantic Analyzer** | Phase 1 Complete | `just ci-gate` | 12/12 handlers; lexical scoping + textDocument/definition |

*Only features that pass `ci-gate` or have targeted integration tests count as "Production".*

---

## Component Summary

For current metrics (LSP coverage %, corpus counts, test pass rates), see [CURRENT_STATUS.md](CURRENT_STATUS.md).

| Crate | Version | Status | Purpose |
|-------|---------|--------|----------|
| **perl-parser** | v0.8.8 | Production | Main parser library |
| **perl-lsp** | v0.8.8 | Production | LSP server (see `features.toml` for GA coverage) |
| **perl-lexer** | v0.8.8 | Production | Context-aware tokenizer |
| **perl-corpus** | v0.8.8 | Production | Test corpus (see `just status-check` for counts) |
| **perl-dap** | v0.1.0 | Phase 1 | Debug Adapter Protocol (bridge mode) |
| **perl-parser-pest** | v0.8.8 | Legacy | Pest-based parser (maintained) |

---

## Next Releases

### v0.9.0: "Semantic-Ready" Milestone (2-5 focused PRs)

**Goal**: A release that external users can try without reading internal docs.

**Deliverables**:

1. **Docs truth pass**
   - README + CURRENT_STATUS + ROADMAP agree on what's real and what's aspirational
   - DAP language corrected to "bridge mode"
   - All claims linked to computed sources or receipts

2. **Release artifacts**
   - Confirm `cargo install --path crates/perl-lsp` works cleanly
   - Release notes that match *advertised* capabilities

3. **Capability contracts**
   - Keep GA-lock snapshot stable
   - Any new capability is either behind feature flag or not advertised

4. **Corpus gap closure (P0)**
   - Fixtures/tests for any missing GA constructs
   - Boundedness tests for hang-risk inputs

**Exit criteria**:
- `nix develop -c just ci-gate` green on MSRV
- `bash scripts/ignored-test-count.sh` shows BUG=0, MANUAL≤1
- Release notes generated, tag cut

### Not Before v0.9

These items are explicitly deferred:
- Full LSP 3.18 compliance (current: 82% advertised GA)
- Semantic Analyzer Phase 2/3 (closures, multi-file resolution, imports)
- Native DAP (currently bridge mode to Perl::LanguageServer)
- Benchmark result publication (framework exists, results not committed)
- Package manager distribution (Homebrew, apt, etc.)

---

### v1.0.0: "Boring Promises" (sequence after v0.9.0)

**Goal**: Freeze the surfaces you're willing to support.

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

**Exit criteria**:
- Capability snapshot + docs aligned
- Benchmarks published
- Upgrade notes exist from v0.8.x → v1.0

---

## Known Gaps (v0.9 Hardening)

These gaps are tracked in [`docs/issues/`](issues/) and need closure before v0.9:

### Corpus Coverage Gaps
- See `docs/issues/corpus/` for NodeKind reachability and GA feature alignment

### Hang/Bounds Risks (P0)
- Deep nesting boundedness
- Slash ambiguity (division vs regex)
- Regex literal handling

### Known Constraints
- **CI Pipeline**: Issue #211 blocks merge-blocking gates (#210)
- **Semantic Phase 2/3**: Advanced features deferred (closures, multi-file, imports)

---

## Completed Work

See [`CURRENT_STATUS.md`](CURRENT_STATUS.md) for detailed completion history.

**Highlights:**
- Statement Tracker & Heredocs (2025-11-20)
- Semantic Analyzer Phase 1 (2025-11-20)
- Band 1: Semantic Stack Validation (2025-12-27)

---

## LSP Feature Implementation

The LSP compliance table is now auto-generated. Source of truth: `features.toml`

<!-- BEGIN: COMPLIANCE_TABLE -->
| Area | Implemented | Total | Coverage |
|------|-------------|-------|----------|
| debug | 0 | 2 | 0% |
| notebook | 0 | 2 | 0% |
| text_document | 21 | 26 | 81% |
| window | 3 | 4 | 75% |
| workspace | 5 | 7 | 71% |
| **Overall** | **29** | **41** | **71%** |
<!-- END: COMPLIANCE_TABLE -->

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

<!-- Last Updated: 2026-01-07 -->
