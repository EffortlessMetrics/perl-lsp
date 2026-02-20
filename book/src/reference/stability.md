# API Stability & Version Policy

**MSRV:** 1.92 | **Edition:** 2024 | **Status:** Initial Public Alpha (v0.9.1)

> For the full stability policy, see [docs/STABILITY.md](../../../../docs/STABILITY.md).

---

## Current Status

Perl LSP v0.9.1 is an **Initial Public Alpha**. While the feature set is substantially complete and the project is well-tested, APIs, protocols, and behaviors are subject to change based on user feedback and technical requirements.

A formal **Stability Contract** (including locked APIs and wire protocol invariants) is targeted for **v0.15.0**.

---

## What "Alpha" Means

- APIs and features are substantially complete but may still evolve
- Breaking changes may occur in minor (0.x) releases without full deprecation cycles
- Wire protocol capabilities are functional but subject to refinement
- We value early adopter feedback to shape the future Stability Contract

---

## Versioning Policy

### Minor Releases (0.Y.0)

Breaking changes are allowed in minor releases (e.g., 0.9.0 to 0.10.0). We provide migration guidance in the CHANGELOG when possible, but full deprecation cycles are not guaranteed until v0.15.0.

### Patch Releases (0.9.Z)

Patch releases are reserved for bug fixes, performance improvements, and documentation updates that do not change public APIs.

---

## Published Crates

| Crate | Version | Status | SemVer Commitment |
|-------|---------|--------|-------------------|
| perl-parser | 0.9.x | Public Alpha | Best-effort |
| perl-lexer | 0.9.x | Public Alpha | Best-effort |
| perl-lsp | 0.9.x | Public Alpha | Best-effort |
| perl-corpus | 0.9.x | Public Alpha | Best-effort |
| perl-dap | 0.2.0 | Preview | No |
| perl-parser-pest | 0.9.x | Legacy/Deprecated | Maintenance only |

---

## Platform Support

### Tier 1 (Targeted)

| Platform | Architecture | Binary Format |
|----------|-------------|---------------|
| Linux (GNU) | x86_64 | ELF (dynamic) |
| Linux (musl) | x86_64 | ELF (static) |
| Linux (GNU) | aarch64 | ELF (dynamic) |
| macOS | x86_64 | Mach-O |
| macOS | aarch64 | Mach-O |
| Windows | x86_64 | PE (MSVC) |

### Tier 2 (Best-Effort)

FreeBSD, NetBSD, OpenBSD, and other platforms with Rust support can be built from source.

---

## Future Stability Contract (v0.15.0)

When the project reaches v0.15.0, we intend to establish a formal Stability Contract:

1. **Strict SemVer**: No breaking changes without major version bump
2. **Protocol Invariants**: LSP capabilities locked for reliable client integration
3. **Deprecation Cycles**: Minimum 6-month warnings for planned changes
4. **Platform Commitment**: Guaranteed support tiers with defined SLAs

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Parse time complexity | O(n) in input size |
| Single-file LSP operations | <50ms for typical files |
| Workspace indexing | <100ms initial, <10ms incremental |
| Go-to-definition | <30ms cross-file |

---

## Security

Security issues should be reported via GitHub Security Advisories or security@perl-lsp.org. We take security seriously even during alpha. See [SECURITY.md](../../../../SECURITY.md) for the full policy.

---

## Verification

```bash
nix develop -c just ci-gate
```

This validates all quality gates before every release.
