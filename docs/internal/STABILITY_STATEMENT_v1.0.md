# Perl LSP v1.0 Stability Statement

> **Status**: DRAFT - For Review
> **Last Updated**: 2026-01-24
> **Applies To**: perl-parser 1.0.x, perl-lsp 1.0.x, perl-lexer 1.0.x

---

## Executive Summary

This document communicates the commitments the Perl LSP project makes to users for the v1.0 release and beyond. It defines what is stable, what may change, and how changes will be communicated.

**Core Promise**: Perl LSP v1.0 is production-ready. You can depend on it for daily development work.

---

## Table of Contents

1. [Compatibility Commitments](#compatibility-commitments)
2. [Supported Platforms](#supported-platforms)
3. [Configuration Stability](#configuration-stability)
4. [Performance Guarantees](#performance-guarantees)
5. [Security Commitments](#security-commitments)
6. [What is NOT Guaranteed](#what-is-not-guaranteed)
7. [Upgrade Path from v0.9.x](#upgrade-path-from-v09x)
8. [Long-Term Support](#long-term-support)
9. [How to Report Issues](#how-to-report-issues)

---

## Compatibility Commitments

### Minimum Supported Rust Version (MSRV)

| Version | MSRV | Rust Edition |
|---------|------|--------------|
| v1.0.x | 1.92 | 2024 |

**MSRV Policy**:
- The MSRV will only increase in **minor** releases (e.g., v1.1.0), never in patch releases
- At least **6 months notice** before any MSRV increase
- MSRV increases will be documented in release notes and CHANGELOG
- Users can pin to older minor versions if MSRV is a concern

### Semantic Versioning

Perl LSP strictly follows [Semantic Versioning 2.0.0](https://semver.org/):

| Release Type | Version Pattern | What Changes |
|--------------|-----------------|--------------|
| **Major** | X.0.0 | Breaking changes allowed |
| **Minor** | 1.Y.0 | New features, deprecations, MSRV bumps (with notice) |
| **Patch** | 1.0.Z | Bug fixes, security patches, documentation |

**Breaking Change Examples** (Major release only):
- Removing or renaming public API functions
- Changing function signatures incompatibly
- Removing LSP capabilities
- Changing wire protocol message formats
- Removing feature flags

**Non-Breaking Changes** (Minor/Patch):
- Adding new `NodeKind` variants (enums are `#[non_exhaustive]`)
- Adding new LSP capabilities
- Adding new configuration options with defaults
- Performance improvements
- Diagnostic message changes

### LSP Protocol Version

| Protocol | Status | Support Duration |
|----------|--------|------------------|
| LSP 3.18 | Primary | Full support through v1.x |
| LSP 3.17 | Supported | Full support through v1.x |
| LSP 3.16 | Supported | Full support through v1.x |
| LSP 3.0-3.15 | Best-effort | Core features work |

**Wire Protocol Guarantees**:
- All capabilities advertised in v1.0 remain available through v1.x
- JSON-RPC message formats follow LSP specification
- LSP method names never change within a major version
- New capabilities may be added in minor releases
- Error codes follow LSP specification (may add new codes, never remove)

**Capability Coverage** (v1.0.0):
- **Text Document**: 41 capabilities (100% implemented)
- **Workspace**: 26 capabilities (100% implemented)
- **Window**: 9 capabilities (100% implemented)
- **Protocol**: 9 capabilities (100% implemented)
- **Notebook**: 2 capabilities (preview, not GA-locked)
- **Debug (DAP)**: 2 capabilities (preview, not GA-locked)

See `features.toml` for the authoritative capability list.

### Extension API Stability

The VS Code extension maintains these guarantees:

| Component | Stability |
|-----------|-----------|
| Settings schema | Stable - existing settings never removed |
| Commands | Stable - existing commands never removed |
| Keybindings | Stable - defaults never changed |
| Status bar items | May change in minor releases |

---

## Supported Platforms

### Tier 1 Platforms (Guaranteed Support)

Tier 1 platforms receive:
- Pre-built binaries for every release
- CI testing on every commit
- Bug fixes within 7 days of confirmed issue
- Security patches within 24 hours of disclosure

| Platform | Architecture | Notes |
|----------|-------------|-------|
| Linux (glibc 2.35+) | x86_64 | Ubuntu 22.04+, Fedora 36+, etc. |
| Linux (musl) | x86_64 | Alpine Linux, static binary |
| Linux (glibc) | aarch64 | ARM64 (Raspberry Pi 4+, AWS Graviton) |
| macOS 10.15+ | x86_64 | Intel Mac |
| macOS 11.0+ | aarch64 | Apple Silicon |
| Windows 10+ | x86_64 | MSVC toolchain |

### Tier 2 Platforms (Best-Effort Support)

Tier 2 platforms receive:
- Source code builds are documented and work
- Community-contributed bug fixes accepted
- No pre-built binaries
- No CI coverage guarantee

| Platform | Notes |
|----------|-------|
| Linux (glibc) | i686 (32-bit x86) |
| Linux (musl) | aarch64 |
| FreeBSD | Community supported |
| NetBSD | Community supported |
| OpenBSD | Community supported |
| Windows (MinGW) | x86_64, GNU toolchain |

### Tier 3 Platforms (Community-Maintained)

Tier 3 platforms may work but have no official support:
- RISC-V (planned upgrade when Rust stabilizes)
- WebAssembly (parser may work, LSP server does not)
- Other Unix variants

### Unsupported Platforms

- 32-bit ARM (armv7)
- MIPS architectures
- PowerPC architectures

---

## Configuration Stability

### Stable Configuration Options

These settings are **GA-locked** and will not change behavior within v1.x:

| Setting | Type | Description |
|---------|------|-------------|
| `perl-lsp.trace.server` | `"off" \| "messages" \| "verbose"` | Server trace level |
| `perl-lsp.diagnostics.enable` | boolean | Enable/disable diagnostics |
| `perl-lsp.completion.enable` | boolean | Enable/disable completion |
| `perl-lsp.formatting.enable` | boolean | Enable/disable formatting |

### Configuration Change Policy

1. **New options**: May be added in any release with sensible defaults
2. **Deprecation**: Existing options may be deprecated with 6-month notice
3. **Removal**: Deprecated options removed only in major releases
4. **Communication**: Changes documented in CHANGELOG and release notes

### Deprecation Format

Deprecated configuration will be communicated via:
```json
{
  "deprecatedMessage": "Use 'perl-lsp.newSetting' instead. Will be removed in v2.0.",
  "markdownDeprecationMessage": "**Deprecated**: Use `perl-lsp.newSetting` instead."
}
```

---

## Performance Guarantees

### Parser Performance

| Metric | Target | Measured (v0.9.0) |
|--------|--------|-------------------|
| 100-line file parse | <20us | 6-8us |
| 1K-line file parse | <50us | 12-18us |
| 10K-line file parse | <500us | 150-200us |
| Incremental update | <5ms | 931ns |
| Time complexity | O(n) | O(n) |
| Space complexity | O(n) | O(n) |

**Guarantees**:
- No exponential time or space blowups on valid Perl code
- Parser never hangs on malformed input (bounded execution)
- Memory usage proportional to input size

### LSP Response Times

| Operation | Target (p95) | Notes |
|-----------|--------------|-------|
| Completion | <50ms | Single file |
| Hover | <20ms | Single file |
| Go-to-definition | <30ms | Cross-file |
| Find references | <100ms | Workspace-wide |
| Rename | <200ms | Workspace-wide |
| Initial indexing | <100ms | Per workspace |
| Incremental indexing | <10ms | Per file change |

**Note**: Performance is not part of the API contract. These are targets, not guarantees. Performance improvements may change timing characteristics.

### What We Measure

Performance benchmarks are tracked for:
- Linux x86_64 (primary)
- macOS aarch64 (secondary)

Run benchmarks locally:
```bash
cargo bench -p perl-parser
cargo bench -p perl-lsp
```

---

## Security Commitments

### Vulnerability Response Policy

| Severity | Response Time | Release Type |
|----------|---------------|--------------|
| **Critical** (RCE, privilege escalation) | 24 hours | Emergency patch |
| **High** (data exposure, DoS) | 7 days | Expedited release |
| **Medium** (info disclosure) | 30 days | Regular release |
| **Low** (minor issues) | Next scheduled | Regular release |

### Security Reporting

Report security issues to: **security@perl-lsp.org** (or GitHub Security Advisories)

**Disclosure Process**:
1. Reporter sends encrypted email (PGP key available on project website)
2. Maintainers acknowledge within 24 hours
3. Investigation and patch development (private)
4. Coordinated disclosure 7-14 days after patch release
5. CVE assignment and public advisory

### Security Hardening

Production hardening commitments:
- **No panics on invalid input**: Parser returns `Result` for all malformed input
- **No unwrap/expect in production**: Enforced by CI (`clippy::unwrap_used`, `clippy::expect_used` = deny)
- **Path traversal protection**: All file operations validate against workspace boundaries
- **Command injection hardening**: No shell interpolation in command execution
- **Memory safety**: Rust memory safety + additional bounds checking
- **Resource limits**: Configurable limits on recursion depth, file size, workspace size

### Supported Versions

| Version | Security Support | Duration |
|---------|------------------|----------|
| v1.x (latest) | Full support | Ongoing |
| v1.x (previous minor) | Security fixes | 6 months after next minor |
| v0.9.x | Critical only | Until 2027-01-01 |
| v0.8.x and older | No support | End of life |

---

## What is NOT Guaranteed

The following may change in any release without notice:

### Internal APIs
- `#[doc(hidden)]` items
- `pub(crate)` items
- Test-only APIs (`#[cfg(test)]`)
- Items behind unstable feature flags

### Diagnostic Output
- Warning and error message text
- Diagnostic severity levels
- Diagnostic codes (new codes may be added)
- Suggestion text in code actions

### Output Formatting
- Debug formatting output
- Pretty-printed error displays
- S-expression output whitespace
- Log message formats

### Resource Usage
- Memory usage patterns (may change with optimizations)
- CPU usage patterns
- Thread count and scheduling
- Temporary file locations

### Experimental Features
- Features behind feature flags marked "unstable" or "experimental"
- Notebook support (preview)
- DAP/Debug support (preview)

### Build Artifacts
- Binary size
- Symbol names
- Internal crate structure
- Build script behavior

---

## Upgrade Path from v0.9.x

### Breaking Changes in v1.0.0

**Good news**: v1.0.0 is primarily a stability release. There are no API breaking changes from v0.9.x.

| Area | Change | Migration |
|------|--------|-----------|
| MSRV | 1.92 (unchanged from 0.9.x) | No action needed |
| API | No breaking changes | No action needed |
| Configuration | No breaking changes | No action needed |
| Wire protocol | No breaking changes | No action needed |

### Recommended Upgrade Process

1. **Update dependency**:
   ```toml
   # Cargo.toml
   perl-parser = "1.0"
   perl-lsp = "1.0"
   ```

2. **Run tests**:
   ```bash
   cargo test
   ```

3. **Verify LSP behavior**:
   - Test completion, hover, go-to-definition in your editor
   - Verify diagnostics appear correctly
   - Test workspace-wide operations (find references, rename)

### Known Issues from v0.8.x

If upgrading from v0.8.x:

| Issue | v0.8.x Behavior | v1.0 Behavior | Migration |
|-------|-----------------|---------------|-----------|
| Position helpers | Different signature | Stable signature | See MIGRATION.md |
| `DeclarationProvider` | No version tracking | Version required | Add version param |
| Error types | `Option<Node>` | `Result<Node, Error>` | Use `?` operator |

See [MIGRATION.md](MIGRATION.md) for detailed v0.8.x migration guidance.

### Compatibility with v0.9.x Extensions

VS Code extensions built for v0.9.x will work with v1.0 without modification:
- All settings are preserved
- All commands work identically
- Protocol compatibility is maintained

---

## Long-Term Support

### Support Timeline

```
2026-01 |---- v1.0.0 LTS ------------------------------------> 2028-01
2026-07 |---- v1.1.0 -----> 2027-01
2027-01 |---- v1.2.0 -----> 2027-07
2028-01 |---- v2.0.0 LTS ------------------------------------> 2030-01

Legend:
|-----> Full support (features, bugs, security)
----> Bug fixes + security
...> Security fixes only
```

### LTS Policy

- **LTS releases**: First release of each major version (v1.0, v2.0, etc.)
- **LTS duration**: 24 months from release date
- **LTS updates**: Security fixes and critical bugs only
- **Standard releases**: 6 months of bug fixes, 12 months of security fixes

### End of Life

When versions reach end of life:
- Documented in release notes 3 months before EOL
- Security advisory posted if critical vulnerabilities exist
- Users encouraged to upgrade
- Source code remains available (archived releases)

---

## How to Report Issues

### Stability Issues

A stability issue is:
- API breakage in patch or minor release
- LSP capability removed without major version bump
- Performance regression >20%
- MSRV increase in patch release
- Wire protocol incompatibility

**How to report**:
1. File issue with label `api-stability`
2. Include minimal reproduction
3. Specify version numbers
4. Reference relevant section of this document

### Regular Bug Reports

For bugs that don't involve stability guarantees:
1. File issue on GitHub
2. Include:
   - Perl LSP version
   - Editor and version
   - Minimal reproduction
   - Expected vs actual behavior

### Feature Requests

- File issue with label `enhancement`
- Describe use case and expected behavior
- Note if this requires capability additions (minor release)

---

## Verification

### How to Verify Stability

```bash
# Run the full CI gate
nix develop -c just ci-gate

# Check capability snapshot
just status-check

# Verify LSP protocol compliance
cargo test -p perl-lsp --test lsp_comprehensive_3_17_test

# Run semantic versioning check
cargo semver-checks check-release
```

### Release Checklist

Every release verifies:
- [ ] `cargo semver-checks` passes
- [ ] `just ci-gate` green on all Tier 1 platforms
- [ ] LSP capability snapshot matches `features.toml`
- [ ] CHANGELOG documents all changes
- [ ] Performance benchmarks within 20% of baseline

---

## Document History

| Date | Version | Change |
|------|---------|--------|
| 2026-01-24 | Draft | Initial v1.0 stability statement |

---

## Summary

| Commitment | Status |
|------------|--------|
| SemVer compliance | Guaranteed |
| MSRV stability (patch releases) | Guaranteed |
| LSP capability stability | Guaranteed for GA features |
| Platform support (Tier 1) | Guaranteed |
| Security response | Committed timelines |
| Performance targets | Best-effort |
| Internal APIs | No guarantee |
| Diagnostic text | No guarantee |

**Verification**:
```bash
nix develop -c just ci-gate
```

This command validates all stability guarantees are met.
