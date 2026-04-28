# ADR-0043: Module Provenance Detection for Perl Module Resolution

**Status**: Proposed
**Date**: 2026-04-17
**Related**: [ADR-0015](0015-supply-chain-security.md), [ADR-0035](0035-deterministic-module-resolution.md), [SUPPLY_CHAIN_SECURITY.md](../reference/SUPPLY_CHAIN_SECURITY.md)

## Context

### The Problem

perl-lsp's module resolution layer (`crates/perl-module/src/resolution/uri.rs`) trusts workspace paths, `use lib` paths, and optionally system `@INC` when resolving Perl modules, but performs no signature or provenance verification on resolved modules. This means the LSP indexes and reasons about Perl modules from potentially untrusted sources without any trust boundary enforcement.

The existing supply-chain security story (ADR-0015, `SUPPLY_CHAIN_SECURITY.md`) covers **release artifact** attestation (SBOM, SLSA provenance for Rust binaries) but explicitly does not cover the Perl modules that the LSP imports and analyzes at runtime. This scope boundary is implied rather than explicit, creating ambiguity about what the project's security posture covers.

### Why This Is In Scope

1. **ADR-0015 scope is intentionally narrow**: It covers "all release artifacts" (the Rust binary), not indexed Perl modules. This gap was identified in [GitHub Issue #3621](https://github.com/EffortlessMetrics/perl-lsp/issues/3621).
2. **Path security is orthogonal**: `perl-path-security` handles filesystem-level attacks (traversal, null-byte injection), but module **trust** is a separate concern. A path can be "safe" (no traversal) but still come from an untrusted CPAN distribution.
3. **`RuntimeDerived` was reserved for this**: `IncRootKind::RuntimeDerived` is marked "reserved for future trusted runtime mode" but never instantiated — suggesting the architecture anticipated trust classification.
4. **Additive only**: The fix must not change existing resolution precedence or break existing behavior.

### Constraints

- **No Perl runtime dependency**: Cryptographic signature verification via `Module::Signature` requires a running Perl interpreter. This is infeasible in the LSP context due to timeout/availability constraints.
- **Non-blocking**: Trust metadata must not add latency to the resolution hot path.
- **Opt-in**: Users who don't care about trust signals should see no additional noise.

## Decision

We implement an **additive, opt-in provenance detection layer** for Perl module resolution. This is **not** cryptographic verification — it detects the **presence** of CPAN distribution markers (META.json/yml, SIGNATURE, CHECKSUMS files) without verifying them.

### 1. Provenance Data Model

We define a new `Provenance` struct attached to `IncRoot` (not modifying `IncRootKind`):

```rust
/// Represents CPAN distribution markers found adjacent to a resolved module.
/// This is NOT cryptographic verification — presence of files is detected,
/// not their validity.
pub struct Provenance {
    /// True if META.json or META.yml exists adjacent to the module
    pub has_meta: bool,
    /// True if SIGNATURE file exists (Module::Signature attestation)
    pub has_signature: bool,
    /// True if CHECKSUMS file exists (SHA/MD5 integrity hash)
    pub has_checksums: bool,
}

impl Default for Provenance {
    fn default() -> Self {
        Self {
            has_meta: false,
            has_signature: false,
            has_checksums: false,
        }
    }
}
```

The `Provenance` struct is stored in a new field on `IncRoot`:

```rust
pub struct IncRoot {
    pub kind: IncRootKind,
    pub path: PathBuf,
    pub precedence: usize,
    pub source: String,
    /// CPAN distribution markers found adjacent to modules in this root.
    /// Populated lazily on first access; None means "not yet scanned".
    pub provenance: Option<Provenance>,
}
```

**Design rationale**: Storing provenance on `IncRoot` (not on each resolved module result) avoids per-resolution I/O. The `Option<Provenance>` allows lazy evaluation — provenance is only scanned when explicitly requested.

### 2. Trust Classification (Informational Only)

Trust classification is based on provenance metadata, **not** source location:

| Classification | Criteria | Interpretation |
|----------------|----------|----------------|
| `Signed` | `has_signature == true` | Module claims CPAN cryptographic attestation |
| `KnownDistributor` | `has_meta == true && has_signature == false` | Module has CPAN metadata but no cryptographic signature |
| `Unknown` | `has_meta == false` | No CPAN distribution markers found |

**Important**: These classifications are informational only. They do **not** affect resolution precedence or block resolution. A module classified as `Unknown` is not rejected — the classification is purely for reporting.

**Why not location-based trust?**:
- Workspace-local modules could be malicious (developer mistake or supply-chain attack on a private module)
- System `@INC` modules are signed by the Perl distribution but could still be compromised
- Location is a weak proxy for trust; provenance metadata is more meaningful

### 3. Distribution Marker Detection (Filesystem-Only)

Detection runs **lazily** and **opt-in**:

```rust
/// Scans a module directory for CPAN distribution markers.
/// This is filesystem-only; no cryptographic verification is performed.
pub fn detect_provenance(module_dir: &Path) -> Provenance {
    Provenance {
        has_meta: module_dir.join("META.json").is_file()
            || module_dir.join("META.yml").is_file(),
        has_signature: module_dir.join("SIGNATURE").is_file(),
        has_checksums: module_dir.join("CHECKSUMS").is_file(),
    }
}
```

Detection is triggered by:
1. A new `IncRoot::detect_provenance()` method called explicitly by consumers
2. **NOT** during the normal resolution path (to avoid adding latency)

### 4. LSP Reporting Setting

A new optional LSP setting (default: `false`):

```rust
/// In workspace settings (e.g., .vscode/settings.json equivalent):
/// {
///   "perl": {
///     "workspace": {
///       "reportUnverifiedModules": true  // default: false
///     }
///   }
/// }
```

When enabled, the LSP surfaces informational diagnostics for modules classified as `Unknown` (no CPAN markers). This is purely informational — it does not block resolution or indexing.

### 5. What This Does NOT Cover

- **Cryptographic verification**: We detect `SIGNATURE` file presence; we do **not** verify the signature using `Module::Signature`. This requires a Perl interpreter and is explicitly **deferred to a future phase**.
- **Trust-based resolution gating**: A module with `Unknown` provenance is not rejected. The classification is for user reporting only.
- **perl-workspace-index integration**: The workspace index is decoupled from module resolution and has no direct dependency on `perl-module`. Any trust annotation integration happens at the LSP layer, not as a direct hook in `perl-workspace-index`.

## Alternatives Considered

### Alternative 1: Cryptographic Verification via Module::Signature (Rejected)

**Approach**: Actually verify CPAN signatures using `Module::Signature` during resolution.

**Why rejected**: Requires a running Perl interpreter with `Module::Signature` installed. This is infeasible in the LSP context due to:
- Timeout constraints (signature verification can be slow)
- Availability constraints (the module may not be installed)
- Complexity (would need to spawn a Perl subprocess per module)

**This remains a future enhancement** for when the project supports trusted runtime mode.

### Alternative 2: Location-Based Trust Classification (Rejected)

**Approach**: Classify modules as `Trusted`/`Untrusted` based on whether they come from workspace, system `@INC`, or external paths.

**Why rejected**: Location is a weak proxy for trust. A module in `use lib '/path/to/darkpan'` from a company's internal CPAN mirror is more trusted than a random workspace module, but would be classified as "untrusted" by location-based rules. Provenance metadata (CPAN distribution markers) provides a more meaningful signal.

### Alternative 3: Do Nothing / Document Gap (Rejected)

**Approach**: Add a note to `SUPPLY_CHAIN_SECURITY.md` explicitly stating that Perl module trust verification is out of scope.

**Why rejected**: The issue correctly identifies a real security gap. Documenting it as out-of-scope without addressing it leaves the project vulnerable to criticism that its supply-chain story is incomplete. An additive, opt-in solution with clear limitations is better than ignoring the problem.

## Consequences

### Positive

1. **Closes the security gap**: Provides provenance visibility for modules the LSP indexes.
2. **Additive and non-breaking**: No changes to existing resolution behavior.
3. **No Perl runtime dependency**: Uses only filesystem presence detection.
4. **Opt-in by default**: Users who don't care about trust signals see no noise.
5. **Clear limitations**: Documents that this is not cryptographic verification.

### Negative

1. **Weak security signal**: Presence of `META.json` or `SIGNATURE` is easy to forge. Users could be misled into thinking a module is "trusted" when it's not.
2. **No blocking**: A malicious module is not prevented from being resolved or indexed.
3. **Additional complexity**: New `Provenance` struct, lazy scanning logic, and LSP setting wiring.
4. **False positives**: System `@INC` modules like `strict.pm` and `warnings.pm` are `Unknown` (no CPAN markers) even though they are more trustworthy than most workspace modules.

### Mitigations for Negatives

- Document explicitly that provenance detection is **not** a security guarantee.
- Default to `reportUnverifiedModules: false` to avoid noise.
- Consider a future enhancement to exclude core Perl modules from `Unknown` classification.

## Implementation Notes

### Files to Modify

1. **`crates/perl-module/src/resolution/uri.rs`**: Add `Provenance` struct and `provenance` field to `IncRoot`. Add `IncRoot::detect_provenance()` method.
2. **`crates/perl-lsp/src/runtime/lifecycle/module_resolution.rs`** (if exists): Wire `reportUnverifiedModules` setting.
3. **`docs/adr/0015-supply-chain-security.md`**: Add explicit note about Perl module scope boundary.
4. **New ADR**: This document.

### Verification Commands

```bash
cargo test -p perl-module
cargo test -p perl-lsp --lib
```

### Note on Crate Architecture

ADR-0035 describes separate microcrates (`perl-module-resolution-uri`, `perl-module-resolution-path`) but the actual codebase has a single `perl-module` crate with internal submodules. The resolution code lives at `crates/perl-module/src/resolution/{mod.rs,uri.rs,path.rs,use_lib.rs}`. Implementation must use the actual crate structure, not the ADR-0035 names.
