# Specification: Module Provenance Detection

**Work Item**: work-7debb2f8
**Feature**: gap(security): module resolution has no signature or provenance verification for Perl modules
**Issue**: [GitHub #3621](https://github.com/EffortlessMetrics/perl-lsp/issues/3621)

---

## Feature Description

An additive, opt-in provenance detection layer for Perl module resolution that detects the presence of CPAN distribution markers (META.json/yml, SIGNATURE, CHECKSUMS files) adjacent to resolved modules. This is **not** cryptographic verification — it only detects file presence. Cryptographic signature verification is explicitly deferred to a future phase.

The feature attaches a `Provenance` struct to each `IncRoot`, storing whether CPAN distribution markers were found. A new LSP setting `workspace.reportUnverifiedModules` (default: `false`) surfaces informational diagnostics for modules with no CPAN markers.

---

## Non-Goals

1. **No cryptographic verification**: We do NOT verify CPAN signatures using `Module::Signature`. That requires a Perl interpreter and is explicitly out of scope for this phase.
2. **No trust-based resolution gating**: Modules with `Unknown` provenance are NOT rejected or blocked from resolution.
3. **No perl-workspace-index integration**: The workspace index is decoupled from module resolution and has no direct dependency on `perl-module`. Trust reporting happens at the LSP layer only.
4. **No changes to resolution precedence**: The existing `IncRootKind` enum and resolution order are unchanged.

---

## Acceptance Criteria

### AC1: Provenance Struct Added to IncRoot

**Given** an `IncRoot` instance for a module path
**When** provenance detection is triggered
**Then** the `IncRoot` has a `provenance: Option<Provenance>` field populated with:
- `has_meta: true` if `META.json` or `META.yml` exists in the module's parent directory
- `has_signature: true` if `SIGNATURE` file exists in the module's parent directory
- `has_checksums: true` if `CHECKSUMS` file exists in the module's parent directory

**Verification**: `cargo test -p perl-module` passes with new provenance tests.

### AC2: Provenance Detection is Lazy and Non-Blocking

**Given** a module resolution call (e.g., `resolve_module_uri`)
**When** the module is resolved
**Then** provenance detection does NOT run synchronously during resolution
**And** the `provenance` field is `None` (not yet scanned)

**Rationale**: Provenance detection adds filesystem I/O; running it during resolution would add latency to the hot path.

### AC3: reportUnverifiedModules Setting (Default: false)

**Given** the LSP server with `reportUnverifiedModules: false` (default)
**When** modules are resolved
**Then** no additional diagnostics are surfaced

**Given** the LSP server with `reportUnverifiedModules: true`
**When** a module with `has_meta: false` is resolved
**Then** an informational diagnostic is surfaced (not an error/warning)

**Verification**: `cargo test -p perl-lsp --lib` passes with new setting tests.

### AC4: Distribution Markers Are Filesystem-Detected Only

**Given** a directory containing a Perl module with adjacent CPAN markers:
```
My-Module/
├── lib/
│   └── My/
│       └── Module.pm
├── META.json
├── SIGNATURE
└── CHECKSUMS
```
**When** provenance detection runs on `lib/My/Module.pm`
**Then** all three markers (`has_meta`, `has_signature`, `has_checksums`) are `true`

**Given** a directory with no CPAN markers
**When** provenance detection runs
**Then** all three fields are `false`

**Note**: No Perl interpreter is involved in detection.

### AC5: Existing Resolution Behavior Unchanged

**Given** existing module resolution tests
**When** the provenance feature is implemented
**Then** all existing resolution tests pass without modification

**Rationale**: The feature is strictly additive; no existing behavior changes.

### AC6: SUPPLY_CHAIN_SECURITY.md Documents Scope Boundary

**Given** the `docs/reference/SUPPLY_CHAIN_SECURITY.md` reference document
**When** a reader examines the security story
**Then** they can clearly determine what is covered (Rust binary SBOM/SLSA) and what is not covered (indexed Perl modules)

**Implementation**: Add a note in `SUPPLY_CHAIN_SECURITY.md` explicitly stating that Perl module trust verification is a separate concern from release artifact attestation.

---

## Implementation Details

### New Types

```rust
/// Represents CPAN distribution markers found adjacent to a resolved module.
/// This is NOT cryptographic verification — presence of files is detected,
/// not their validity.
#[derive(Debug, Clone, Default)]
pub struct Provenance {
    /// True if META.json or META.yml exists adjacent to the module
    pub has_meta: bool,
    /// True if SIGNATURE file exists (Module::Signature attestation)
    pub has_signature: bool,
    /// True if CHECKSUMS file exists (SHA/MD5 integrity hash)
    pub has_checksums: bool,
}
```

### Modified Types

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

### Files to Modify

| File | Change |
|------|--------|
| `crates/perl-module/src/resolution/uri.rs` | Add `Provenance` struct, `provenance` field to `IncRoot`, `IncRoot::detect_provenance()` method |
| `crates/perl-module/src/resolution/mod.rs` | Re-export `Provenance` if needed |
| `crates/perl-lsp/src/runtime/lifecycle/module_resolution.rs` | Wire `reportUnverifiedModules` setting (verify path exists first) |
| `docs/reference/SUPPLY_CHAIN_SECURITY.md` | Add scope boundary note |

### Note on Crate Architecture

ADR-0035 describes separate microcrates (`perl-module-resolution-uri`, `perl-module-resolution-path`) but the actual codebase has a single `perl-module` crate with submodules at `crates/perl-module/src/resolution/`. Implementation must use actual paths:
- `crates/perl-module/src/resolution/uri.rs` (not `perl-module-resolution-uri`)
- `crates/perl-module/src/resolution/mod.rs` (not `perl-module-resolution`)

---

## Dependencies

1. **Rust standard library only**: No external crates needed for filesystem presence detection.
2. **No Perl runtime**: Detection is purely filesystem-based.
3. **No changes to perl-workspace-index**: It's decoupled from module resolution.

---

## Future Enhancements (Out of Scope)

1. **Cryptographic verification**: Verify CPAN `SIGNATURE` files using `Module::Signature` (requires Perl interpreter).
2. **Trust-based resolution gating**: Reject or flag modules with `Unknown` provenance.
3. **Core Perl module exclusion**: Don't flag `strict.pm`, `warnings.pm` as `Unknown` since they're signed by the Perl distribution.
4. **DarkPAN support**: Allow enterprises to configure trusted internal CPAN mirrors.
