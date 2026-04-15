# ADR-0020: Module Resolution Trust Boundary — Signature Verification Out of Scope

**Status**: Proposed
**Date**: 2026-04-15
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ADR-0015](./0015-supply-chain-security.md)

## Context

The GitHub issue [effortlessmetrics/perl-lsp#4361](https://github.com/effortlessmetrics/perl-lsp/issues/4361) identifies a documentation gap: the trust boundary for module resolution is not clearly documented. Users and contributors may assume that module resolution provides security guarantees (such as signature verification) that it does not actually provide.

### Problem Statement

1. **Undocumented Trust Boundary**: Module resolution trusts workspace paths, configured include paths, and optionally system `@INC` without any signature or provenance verification for resolved Perl modules.

2. **Potential Misunderstanding**: Without explicit documentation, users may assume:
   - Module resolution verifies CPAN distribution SIGNATURE files (per Module::Signature)
   - The `IncRoot` struct carries trust or provenance metadata
   - `perl-path-security` provides distribution trust verification

3. **Architectural Clarity**: The existing ADR-0015 covers SBOM and SLSA provenance for **release artifacts** but is silent on runtime module resolution trust boundaries.

### Evidence

The following code artifacts confirm that no signature verification exists:

- **`IncRoot` struct** (`crates/perl-module-resolution-uri/src/lib.rs`): Contains only `kind`, `path`, `precedence`, and `source` fields — no signature status, trust level, or distribution integrity fields.

- **`ModuleUriResolution::Resolved(String)`**: Returns only a URI string with no provenance metadata.

- **Resolution logic** (`crates/perl-module-resolution-uri/src/lib.rs`): Performs `full_path.is_file()` existence checks only — no signature file reading or verification.

- **`perl-path-security` crate**: Explicitly scoped to "Workspace-bound path validation and traversal prevention" — it cannot and does not reason about distribution trust.

- **grep results**: Zero references to `signature`, `Signature`, `SIGNATURE`, `CPAN::Signature`, or `Module::Signature` in `crates/perl-module-resolution-uri/src/`.

## Decision

**We clarify the module resolution trust boundary through documentation only (Option A).** Perl module signature verification is explicitly out of scope for the following reasons:

1. **Low CPAN Ecosystem Adoption**: CPAN distribution SIGNATURE file verification (per Module::Signature) has low adoption in the Perl ecosystem. Implementing signature verification would be over-engineered for the actual user need.

2. **Architectural Separation**: Path-based workspace security (`perl-path-security`) and distribution trust verification are separate concerns. Mixing them would blur architectural boundaries.

3. **ADR-0015 Scope**: The existing ADR-0015 covers SBOM and SLSA provenance for **release artifacts** (the perl-lsp tool itself), not runtime module resolution. This is a different trust domain.

### Documentation Changes

The following changes document the trust boundary:

1. **ADR-0015**: Add "Non-Goals: Perl Module Signature Verification" section explaining that:
   - Module::Signature SIGNATURE files are not verified
   - Distribution integrity is not checked
   - Users needing verification should use external tools (e.g., `CPAN::Shell->verify`)
   - This is a deliberate design choice, not an oversight

2. **`IncRoot` struct docs** (`crates/perl-module-resolution-uri/src/lib.rs`): Update to explicitly state it carries path-based resolution metadata only — no signature status, trust levels, or provenance information.

3. **`perl-path-security` module docs** (`crates/perl-path-security/src/lib.rs`): Clarify that path validation (traversal prevention, workspace bounds) is architecturally distinct from distribution trust verification.

4. **Test file** (`crates/perl-module-resolution-uri/tests/module_signature_nongol.rs`): New integration tests documenting that module resolution performs path-based checks only, without signature verification.

## Alternatives Considered

### Option B: Implement Signature Verification

Reject. CPAN SIGNATURE file adoption is very low in the Perl ecosystem. Implementing signature verification would require:
- Adding `Module::Signature` or similar dependency
- Changing `IncRoot` to carry signature status
- Modifying `ModuleUriResolution` enum to include provenance metadata
- Significant complexity for a feature most users would not use

This would be over-engineered for the actual need and would blur the architectural separation between path security and distribution trust.

### Option C: Reject the Issue (Won't Fix)

Reject. While signature verification is out of scope, the documentation gap is real. Users and contributors need clear documentation of what module resolution does and does not guarantee. Documentation-only Option A addresses this at minimal cost.

## Consequences

### Positive

1. **Clear Trust Boundary**: Users understand exactly what module resolution guarantees and does not guarantee
2. **Reduced Support Burden**: Users will not ask "why doesn't perl-lsp verify CPAN signatures?" when the docs explicitly state it's out of scope
3. **Architectural Clarity**: The separation between path-based workspace security and distribution trust is now explicit
4. **Easier Security Reviews**: Security auditors can quickly understand what is and is not within scope
5. **No Scope Creep**: Future contributors will not assume signature verification exists and try to "complete" an incomplete feature

### Negative

1. **Documentation Maintenance**: The non-goals section must be kept in sync with implementation changes
2. **No New Protections**: Users who need signature verification must use external tools

### Mitigations

- Frame non-goals around verifiable facts (what the code does NOT do) rather than policy statements about ecosystem adoption
- Keep the non-goals section tied to the `IncRoot` struct fields and resolution logic — facts that are stable
- The test file serves as executable documentation that will fail if behavior changes unexpectedly

## Non-Goals (Explicit Out of Scope)

The following are explicitly out of scope for this decision:

- Runtime Perl module signature verification (CPAN::Signature, Module::Signature SIGNATURE files)
- Distribution integrity checking
- Provenance metadata in `IncRoot` or `ModuleUriResolution`
- Changes to resolution algorithm behavior
- Changes to `ModuleUriResolution` enum variants or `IncRoot` struct fields

## References

- [ADR-0015: Supply Chain Security (SBOM + SLSA Provenance)](./0015-supply-chain-security.md)
- [effortlessmetrics/perl-lsp#4361](https://github.com/effortlessmetrics/perl-lsp/issues/4361)
- [Module::Signature CPAN distribution](https://metacpan.org/pod/Module::Signature)
- [perl-path-security crate](../crates/perl-path-security/src/lib.rs)
- [perl-module-resolution-uri crate](../crates/perl-module-resolution-uri/src/lib.rs)
