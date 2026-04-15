# Specification: Module Resolution Trust Boundary Documentation

## Feature Description

Documentation-only changes to clarify the module resolution trust boundary in perl-lsp. This work adds an explicit "Non-Goals" section to ADR-0015 and updates doc comments in `perl-module-resolution-uri` and `perl-path-security` to state that Perl module signature verification is intentionally out of scope.

After this work is implemented:
- ADR-0015 will have a clear "Non-Goals: Perl Module Signature Verification" section
- The `IncRoot` struct documentation will explicitly state it carries path-based resolution metadata only
- The `perl-path-security` module documentation will clarify that path validation is distinct from distribution trust
- A new test file will document the actual behavior (path-based checks only, no signature verification)

## Acceptance Criteria

### AC1: ADR-0015 Non-Goals Section Added

**Given** the existing ADR-0015 at `docs/adr/0015-supply-chain-security.md`
**When** a reader reaches the "Security Guarantees" section
**Then** there is a "Non-Goals: Perl Module Signature Verification" subsection that explicitly states:
- Module::Signature SIGNATURE files are not verified
- Distribution integrity is not checked
- Module resolution trusts configured paths without provenance verification
- Users needing verification should use external tools (e.g., CPAN::Shell->verify)

### AC2: IncRoot Struct Documentation Updated

**Given** the `IncRoot` struct in `crates/perl-module-resolution-uri/src/lib.rs`
**When** a developer reads the struct documentation
**Then** the docs explicitly state that:
- The struct carries path-based resolution metadata only
- No fields exist for signature status, trust level, or distribution integrity
- Resolution returns URI strings with no provenance information

### AC3: perl-path-security Module Documentation Clarified

**Given** the `perl-path-security` crate at `crates/perl-path-security/src/lib.rs`
**When** a developer reads the module documentation
**Then** the docs explicitly state that:
- Path validation (traversal prevention, workspace bounds) is architecturally distinct from distribution trust verification
- The crate focuses on workspace path boundaries, not module provenance

### AC4: New Test File Documents Current Behavior

**Given** the existing `perl-module-resolution-uri` crate
**When** the test suite runs
**Then** there exists a test file `crates/perl-module-resolution-uri/tests/module_signature_nongol.rs` with tests that:
- Verify module resolution returns a URI when the module file exists (path-based check only)
- Verify that a module with an adjacent SIGNATURE file is resolved without reading/verifying the signature
- Verify that `IncRoot` struct has no signature-related fields (documenting the API contract)

### AC5: All Tests Pass

**Given** the changes are implemented
**When** `cargo test -p perl-module-resolution-uri` runs
**Then** all tests pass including the new `module_signature_nongol.rs` tests

### AC6: No Clippy Warnings

**Given** the changes are implemented
**When** `cargo clippy -p perl-module-resolution-uri -p perl-path-security` runs
**Then** no warnings are produced

## Non-Goals (Explicitly Out of Scope)

The following are NOT included in this specification:

1. **No signature verification implementation**: No runtime or compile-time signature verification for Perl modules
2. **No API changes**: No modifications to `ModuleUriResolution` enum variants or `IncRoot` struct fields
3. **No resolution algorithm changes**: The module resolution logic remains unchanged
4. **No new dependencies**: No new crates or external dependencies
5. **No SBOM/SLSA changes to perl-lsp**: Release artifact attestation (ADR-0015) is unchanged — this work only adds documentation

## Dependencies

- `tempfile`: Used by existing test infrastructure for creating temporary directories
- `url`: Used by existing test infrastructure for URI manipulation
- `perl_module_resolution_uri`: The crate under test (no API changes)
- `perl_path_security`: The crate under documentation clarification (no API changes)

## File Inventory

| File | Action | Description |
|------|--------|-------------|
| `docs/adr/0015-supply-chain-security.md` | Modify | Add "Non-Goals: Perl Module Signature Verification" section |
| `crates/perl-module-resolution-uri/src/lib.rs` | Modify | Update `IncRoot` struct docs |
| `crates/perl-path-security/src/lib.rs` | Modify | Update module-level docs |
| `crates/perl-module-resolution-uri/tests/module_signature_nongol.rs` | Create | New BDD-style test file |
