# Perl Parser Project - Roadmap

> **Canonical**: This is the authoritative roadmap. See `CURRENT_STATUS.md` for computed metrics.
> **Stale roadmaps**: Archived at `docs/archive/roadmaps/`; retrieve from git history if needed.

> **Status (2026-02-20)**: **Initial Public Alpha (v0.9.1)**. Hardening underway with native DAP preview validated.
>
> **Canonical receipt**: `nix develop -c just ci-gate` must be green before merging.
> **CI** is intentionally optional/opt-in; the repo is local-first by design.

---

## Alpha Disclaimer

Perl LSP is currently in **Initial Public Alpha**. Version 0.9.1 represents a substantially complete feature set, but APIs and protocols are still evolving. We value early adopter feedback to refine the project toward the v0.15.0 Stability Contract milestone.

---

## Current State (v0.9.1 → v0.10.0)

| Component | Release Stance | Evidence | Notes |
|-----------|----------------|----------|-------|
| **perl-parser** (v3) | Public Alpha | `just ci-gate` | Parser v3, statement tracker + heredocs in place |
| **perl-lexer** | Public Alpha | `just ci-gate` | Tokenization stable |
| **perl-corpus** | Public Alpha | `just ci-gate` | Regression corpus + mutation hardening inputs |
| **perl-lsp** | Public Alpha (advertised subset) | capability snapshots + targeted tests | Evolving feature set |
| **perl-dap** | Preview (Native + Bridge) | `cargo test -p perl-dap --features dap-phase2,dap-phase3` | Native adapter foundations with BridgeAdapter fallback |
| **perl-parser-pest** (v2) | Legacy | N/A | Optional legacy crate |
| **Semantic Analyzer** | Phase 2-6 Complete | `just ci-gate` | Full semantic analysis pipeline |

---

## Now / Next / Later (Summary)

**Now (v0.9.1 Initial Public Alpha)**
- Keep close-out receipts green (`just ci-gate`, targeted state-machine tests, benchmark checks)
- Public Alpha hardening: focus on correctness and performance for early adopters
- Publish benchmark outputs under `benchmarks/results/`

**Next (v0.10.0)**
- Stability goal refinement: define requirements for v0.15.0 contract
- Moo/Moose semantic depth improvements
- Native DAP enhancements (variables/evaluate)

**Later (Targeting v0.15.0 for Stability Contract)**
- **Stability Contract**: Formal API stability and contract-locked wire protocol
- Full LSP 3.18 compliance
- Finalized shim distribution strategy
- Package manager distribution (Homebrew/apt/etc.)

---

## Component Summary

For current metrics (LSP coverage %, corpus counts, test pass rates), see [CURRENT_STATUS.md](CURRENT_STATUS.md).

| Crate | Version | Status | Purpose |
|-------|---------|--------|----------|
| **perl-parser** | v0.9.1 | Public Alpha | Main parser library |
| **perl-lsp** | v0.9.1 | Public Alpha | LSP server |
| **perl-lexer** | v0.9.1 | Public Alpha | Context-aware tokenizer |
| **perl-corpus** | v0.9.1 | Public Alpha | Test corpus |
| **perl-dap** | v0.2.0 | Preview (Native + Bridge) | Debug Adapter Protocol |
| **perl-parser-pest** | v0.9.1 | Legacy | Pest-based parser (maintained) |

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

For live metrics, run `just status-check` or see [CURRENT_STATUS.md](CURRENT_STATUS.md).

---

## Completed Work

See [`CURRENT_STATUS.md`](CURRENT_STATUS.md) for detailed completion history.

**Highlights:**
- Initial project fork (July 15, 2025) from `tree-sitter-perl-better`.
- Statement Tracker & Heredocs (2025-11-20)
- Semantic Analyzer Phase 1 (2025-11-20)
- Semantic Analyzer Phase 2-6 Complete (2026-01-21)
- Refactoring Engine: inline + move_code (2026-01-21)
- Security Hardening: path traversal + command injection (2026-01-21)
- v0.9.1 Initial Public Alpha Preparation (2026-02-20)

---

## Resources

**Start here:** [`INDEX.md`](INDEX.md) - Routes you to the right doc.

- **[Current Status](CURRENT_STATUS.md)** - Computed metrics
- **[features.toml](../features.toml)** - Canonical capability definitions
- **[LESSONS.md](LESSONS.md)** - Project learnings

<!-- Last Updated: 2026-02-20 -->
