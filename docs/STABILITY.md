# API Stability & Version Policy - v0.9.1 Public Alpha

**MSRV:** 1.92 • **Edition:** 2024 • **Status:** Initial Public Alpha

## Executive Summary

This document defines the stability goals for Perl LSP. **Please note that v0.9.1 is an initial public alpha.** While we strive for a reliable experience, the project is still evolving and APIs, protocols, and behaviors are subject to change based on user feedback and technical requirements.

Formal **Stability Contract** guarantees (including contract-locked APIs and wire protocol invariants) are currently targeted for the **v0.15.0** milestone.

**What "Alpha" means for v0.9.1:**
- APIs and features are substantially complete but may still evolve
- Breaking changes may occur in minor (0.x) releases without full deprecation cycles
- Wire protocol capabilities are experimental and subject to refinement
- Documentation is a work in progress
- We highly value early adopter feedback to shape the future 0.15.0 Stability Contract

---

## Table of Contents

1. [Published Artifacts](#published-artifacts)
2. [Stability Goals](#stability-goals)
3. [Versioning Policy](#versioning-policy)
4. [Platform Support Matrix](#platform-support-matrix)
5. [API Surface](#api-surface)
6. [Future Stability Contract (v0.15.0)](#future-stability-contract)
7. [Feature Flags](#feature-flags)
8. [Performance](#performance)
9. [Security Support](#security-support)

---

## Published Artifacts

### What We Ship (v0.9.x (Public Alpha)+)

| Distribution | Format | Support Level | Update Cadence |
|--------------|--------|---------------|----------------|
| **Binaries** | GitHub Releases (tar.gz, zip) | Alpha | Every release |
| **Crates** | crates.io | Alpha | Every release |
| **VS Code Extension** | VS Marketplace | Alpha | Every release |
| **Homebrew** | Formula (macOS/Linux) | Alpha | Automated on release |
| **Source** | GitHub releases + tags | Alpha | Every release |

### Published Crates (v0.9.1 Alpha)

| Crate | Version | Purpose | Status | SemVer Commitment |
|-------|---------|---------|--------|-------------------|
| [perl-parser](https://crates.io/crates/perl-parser) | 0.9.x (Public Alpha) | Parser & AST | Evolving | Best-effort |
| [perl-lexer](https://crates.io/crates/perl-lexer) | 0.9.x (Public Alpha) | Tokenizer | Evolving | Best-effort |
| [perl-lsp](https://crates.io/crates/perl-lsp) | 0.9.x (Public Alpha) | LSP Server Binary | Evolving | Best-effort |
| [perl-corpus](https://crates.io/crates/perl-corpus) | 0.9.x (Public Alpha) | Test corpus | Evolving | Best-effort |
| [perl-dap](https://crates.io/crates/perl-dap) | 0.2.0 | Debug Adapter | **Preview** | No - pre-0.15 |
| [perl-parser-pest](https://crates.io/crates/perl-parser-pest) | 0.9.x (Public Alpha) | Legacy parser | **Deprecated** | Maintenance only |

**Note:** Formal stability contracts are deferred until v0.15.0.

---

## Stability Goals

During the alpha phase, our goals are:

1. **Functional Completeness**: Covering 100% of Perl 5 syntax
2. **Reliability**: Eliminating crashes and panics on arbitrary input
3. **Performance**: Maintaining sub-millisecond parsing and responsive LSP interactions
4. **Iterative Refinement**: Improving APIs and protocol handlers based on real-world usage

---

## Versioning Policy

Perl LSP currently follows a "pre-stability" versioning model:

### Minor Releases (0.Y.0)

**Breaking changes are allowed** in minor releases (e.g., 0.9.0 → 0.10.0). We will provide migration guidance in the CHANGELOG whenever possible, but full multi-release deprecation cycles are not guaranteed until v0.15.0.

### Patch Releases (0.9.Z)

Patch releases are reserved for bug fixes, performance improvements, and documentation updates that do not change public APIs.

---

## Future Stability Contract (Target: v0.15.0)

When the project reaches **v0.15.0**, we intend to implement a formal **Stability Contract** including:

1. **Strict SemVer**: No breaking changes without a major version bump
2. **Contract-Locked Wire Protocol**: Guaranteed compatibility for LSP clients
3. **Deprecation Cycles**: Minimum 6-month warnings for any planned changes
4. **Long-Term Support**: Maintenance releases for specific stable milestones

---

## Platform Support Matrix

### Tier 1 Platforms (Targeted for Support)

Tier 1 platforms are our primary targets for testing and binary distribution:

| Platform | Architecture | Toolchain | Binary Format |
|----------|-------------|-----------|---------------|
| **Linux (GNU)** | x86_64 | stable | ELF (dynamic) |
| **Linux (musl)** | x86_64 | stable | ELF (static) |
| **Linux (GNU)** | aarch64 | stable | ELF (dynamic) |
| **macOS** | x86_64 | stable | Mach-O |
| **macOS** | aarch64 | stable | Mach-O |
| **Windows** | x86_64 | stable-msvc | PE (MSVC) |

---

## Performance

### Alpha Goals

**Parser performance targets:**
- **Time complexity:** O(n) in input size for valid Perl code
- **Space complexity:** O(n) for AST construction
- **Streaming:** Support incremental parsing for sub-millisecond updates

**LSP response time targets:**
- **Single-file operations:** <50ms for typical files
- **Workspace indexing:** <100ms initial index, <10ms incremental
- **Go-to-definition:** <30ms cross-file

---

## Security Support

### Security Policy

**Report security issues to:** security@perl-lsp.org (or GitHub Security Advisories)

We take security seriously even in alpha. Coordinated disclosure is preferred for any vulnerabilities discovered.

---

## Summary Checklist: v0.9.1 Initial Public Alpha

✅ **Functional Completeness:** Handles almost all modern Perl constructs
✅ **LSP Protocol:** Broad coverage of LSP 3.18 methods (alpha-validated)
✅ **Platform Support:** Focused on major Linux, macOS, and Windows environments
✅ **Performance:** High-performance O(n) parsing
✅ **Security:** Focused on path validation and memory safety
✅ **MSRV Policy:** Rust 1.92+ (2024 edition)
✅ **Testing:** 600+ tests, comprehensive E2E suite

**Verification command:**
```bash
nix develop -c just ci-gate
```
