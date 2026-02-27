# Perl Parser Project - Roadmap

> **Canonical**: This is a summary. See [docs/ROADMAP.md](../../../../docs/ROADMAP.md) for the full roadmap.
> **Metrics**: See [CURRENT_STATUS.md](../../../../docs/CURRENT_STATUS.md) for computed metrics.

> **Status (2026-02-20)**: **Initial Public Alpha (v0.9.1)**. Hardening underway with native DAP preview validated.
>
> **Canonical receipt**: `nix develop -c just ci-gate` must be green before merging.

---

## Alpha Disclaimer

Perl LSP is currently in **Initial Public Alpha**. Version 0.9.1 represents a substantially complete feature set, but APIs and protocols are still evolving. We value early adopter feedback to refine the project toward the v0.15.0 Stability Contract milestone.

---

## Current State (v0.9.1)

| Component | Release Stance | Evidence | Notes |
|-----------|----------------|----------|-------|
| **perl-parser** (v3) | Public Alpha | `just ci-gate` | Parser v3, statement tracker + heredocs |
| **perl-lexer** | Public Alpha | `just ci-gate` | Tokenization stable |
| **perl-corpus** | Public Alpha | `just ci-gate` | Regression corpus + mutation hardening inputs |
| **perl-lsp** | Public Alpha (advertised subset) | capability snapshots + targeted tests | Evolving feature set |
| **perl-dap** | Preview (Native + Bridge) | `cargo test -p perl-dap --features dap-phase2,dap-phase3` | Native adapter foundations with BridgeAdapter fallback |
| **perl-parser-pest** (v2) | Legacy | N/A | Optional legacy crate |
| **Semantic Analyzer** | Phase 2-6 Complete | `just ci-gate` | Full semantic analysis pipeline |

---

## Now / Next / Later

**Now (v0.9.1 Initial Public Alpha)**
- Keep close-out receipts green (`just ci-gate`)
- Public Alpha hardening: focus on correctness and performance for early adopters
- Publish benchmark outputs under `benchmarks/results/`

**Next (v0.10.0)**
- Stability goal refinement: define requirements for v0.15.0 contract
- Moo/Moose semantic depth improvements
- Native DAP enhancements (variables/evaluate)

**Later (Targeting v0.15.0 for Stability Contract)**
- **Stability Contract**: Formal API stability and locked wire protocol
- Full LSP 3.18 compliance
- Finalized shim distribution strategy
- Package manager distribution (Homebrew/apt/etc.)

---

## Component Summary

| Crate | Version | Status | Purpose |
|-------|---------|--------|---------|
| **perl-parser** | v0.9.1 | Public Alpha | Main parser library |
| **perl-lsp** | v0.9.1 | Public Alpha | LSP server |
| **perl-lexer** | v0.9.1 | Public Alpha | Context-aware tokenizer |
| **perl-corpus** | v0.9.1 | Public Alpha | Test corpus |
| **perl-dap** | v0.2.0 | Preview (Native + Bridge) | Debug Adapter Protocol |
| **perl-parser-pest** | v0.9.1 | Legacy | Pest-based parser (maintained) |

---

## Known Gaps

> For the full honest assessment, see [docs/HONEST_ASSESSMENT.md](../../../../docs/HONEST_ASSESSMENT.md).

### Resolved (v0.9.1)
- Comprehensive corpus (732KB, 78 files, 611+ sections)
- Budget-protected lexer, recursion limits, fuzz testing
- Fixed workspace root security boundary

### Open Gaps
- **DAP is preview, not stable**: Native adapter covers breakpoint/control-flow/attach foundations, but runtime variable/evaluate fidelity and packaging strategy need hardening.
- **Moo/Moose semantic blindness**: Parser tokenizes correctly but semantic analyzer does not understand `has` as field declaration. Number one real-world gap.
- **No E2E LSP smoke test**: All testing is unit/integration; no automated test sends real LSP messages end-to-end.
- **Pest parser orphaned**: Compiles and works but excluded from CI, not used in production, significantly slower than v3.

---

## LSP Feature Implementation

The LSP compliance table is auto-generated. Source of truth: `features.toml`

For live metrics, run `just status-check` or see [CURRENT_STATUS.md](../../../../docs/CURRENT_STATUS.md).

---

## Resources

- **[Current Status](../../../../docs/CURRENT_STATUS.md)** - Computed metrics
- **[features.toml](../features.toml)** - Canonical capability definitions
- **[Full Roadmap](../../../../docs/ROADMAP.md)** - Complete release planning and history

<!-- Last Updated: 2026-02-20 -->
