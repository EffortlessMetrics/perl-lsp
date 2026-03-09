# Perl Parser Project - Roadmap

> **Canonical**: This is the authoritative roadmap. See `CURRENT_STATUS.md` for computed metrics.
> **Stale roadmaps**: Archived at `docs/archive/roadmaps/`; retrieve from git history if needed.

> **Status (2026-03-04)**: **Initial Public Alpha (v0.10.0)**. Post-release hardening and SRP microcrate extractions underway.
>
> **Canonical receipt**: `nix develop -c just ci-gate` must be green before merging.
> **CI** is intentionally optional/opt-in; the repo is local-first by design.

---

## Alpha Disclaimer

Perl LSP is currently in **Initial Public Alpha**. Version 0.10.0 represents a substantially complete feature set, but APIs and protocols are still evolving. We value early adopter feedback to refine the project toward the v0.15.0 Stability Contract milestone.

---

## Current State (v0.10.0)

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

**Now (Post-v0.10.0 Hardening)**
- SRP microcrate extractions (ongoing — PRs #934, #945, #950, #953)
- Moo/Moose semantic depth: `requires` tracking and multi-attribute `has` landed (PR #946)
- Keep close-out receipts green (`just ci-gate`)

**Next (v0.11.0)**
- Complete Moo/Moose/Class::Accessor attribute resolution (partial: PR #946)
- Cross-file type inference via `use parent`/`use base`
- Native DAP enhancements (variables/evaluate)
- Stability goal refinement: define requirements for v0.15.0 contract

**Later (Targeting v0.15.0 for Stability Contract)**
- **Stability Contract**: Formal API stability and contract-locked wire protocol
- Full LSP 3.18 compliance
- Finalized shim distribution strategy
- Package manager distribution (Homebrew/apt/etc.)

---

## Component Summary

For current metrics (LSP coverage %, corpus counts, test pass rates), see [Current Status](current-status.md).

| Crate | Version | Status | Purpose |
|-------|---------|--------|----------|
| **perl-parser** | v0.10.0 | Public Alpha | Main parser library |
| **perl-lsp** | v0.10.0 | Public Alpha | LSP server |
| **perl-lexer** | v0.10.0 | Public Alpha | Context-aware tokenizer |
| **perl-corpus** | v0.10.0 | Public Alpha | Test corpus |
| **perl-dap** | v0.2.0 | Preview (Native + Bridge) | Debug Adapter Protocol |
| **perl-parser-pest** | v0.10.0 | Legacy | Pest-based parser (maintained) |

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

For live metrics, run `just status-check` or see [Current Status](current-status.md).

---

## Completed Work

See [Current Status](current-status.md) for detailed completion history.

**Highlights:**
- Initial project fork (July 15, 2025) from `tree-sitter-perl-better`.
- Statement Tracker & Heredocs (2025-11-20)
- Semantic Analyzer Phase 1 (2025-11-20)
- Semantic Analyzer Phase 2-6 Complete (2026-01-21)
- Refactoring Engine: inline + move_code (2026-01-21)
- Security Hardening: path traversal + command injection (2026-01-21)
- v0.10.0 Initial Public Alpha Preparation (2026-02-28)
- Moo/Moose `requires` tracking and multi-attribute `has` (PR #946, 2026-03)
- SRP microcrate extractions: dead-code (#945), lsp-limits (#934), capability-mapping (#950), subprocess-runtime (#953)
- Feature governance extracted into 9 microcrates (PR #848)

---

## Resources

- **[Current Status](current-status.md)** - Computed metrics
- **[Lessons Learned](../process/lessons.md)** - Project learnings

<!-- Last Updated: 2026-03-04 -->

## Detailed Forward-Looking Roadmap

### v0.11.0: Advanced Semantic Engine
- **Goal:** Deepen semantic understanding of complex Perl constructs.
- **Features:**
  - Full Moo/Moose/Class::Accessor attribute resolution. *(In progress: `requires` tracking and multi-attribute `has` landed in PR #946; `Class::Accessor` not yet started.)*
  - Cross-file type inference across standard import mechanisms (`use parent`, `use base`).
  - Improved bareword disambiguation based on export lists.
  - Constant folding and compile-time evaluation approximations.

### v0.12.0: Diagnostic Hardening
- **Goal:** Parity with `perl -c` without the security risks of actual execution.
- **Features:**
  - Strict mode and warnings pragma emulation.
  - Uninitialized variable detection across function boundaries.
  - Dead code elimination recommendations. *(Foundation: `perl-dead-code` microcrate extracted in PR #945.)*
  - Syntax deprecation warnings (e.g., smartmatch, indirect object notation).

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
