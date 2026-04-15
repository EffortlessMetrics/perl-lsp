# Initial Plan — work-d53699dc

## Approach

This is a documentation-only change (Option A from the issue) because CPAN signatures have low adoption in the Perl ecosystem and implementing signature verification would be over-engineered for the actual user need. The goal is to explicitly document that Perl module signature verification is outside the scope of the supply chain security work already documented in ADR-0015, making the trust boundary explicit for users and future developers.

The approach is straightforward because the issue already provides a clear builder spec with specific file locations and the changes are limited to documentation and tests only — no algorithmic changes are needed.
1. Add a "Non-Goals" section to ADR-0015
2. Update two doc comments to clarify the trust boundary
3. Create a test file that documents the current behavior (path-based checks only)

## Task Breakdown

### Phase 1: Documentation Updates

1. **`docs/adr/0015-supply-chain-security.md`**
   - Add new section "## Non-Goals: Perl Module Signature Verification" after the "Consequences" section (after line 119)
   - Explain: CPAN signatures have low adoption; module resolution trusts paths configured in workspace; this is intentionally out of scope; users needing verification should use external tools

2. **`crates/perl-module-resolution-uri/src/lib.rs`**
   - Update `IncRoot` struct doc comment (line 33) to state explicitly that it carries path-based resolution metadata only, with no signature verification, trust levels, or provenance information

3. **`crates/perl-path-security/src/lib.rs`**
   - Update the module-level doc comment (line 1) to clarify that path validation (traversal prevention, workspace bounds) is distinct from distribution trust verification

### Phase 2: Test File Creation

4. **`crates/perl-module-resolution-uri/tests/module_signature_nongol.rs`**
   - Create new test file following the existing BDD test pattern
   - Three tests documenting that module resolution:
     - Resolves modules by path existence only
     - Does not verify CPAN signatures (no SIGNATURE file check)
     - Does not verify distribution integrity
   - Tests should pass with current implementation (documenting behavior, not changing it)

### Phase 3: Verification

5. Run `cargo test -p perl-module-resolution-uri` to verify new tests pass
6. Run `cargo clippy -p perl-module-resolution-uri -p perl-path-security` to check for warnings
7. Run `cargo build` to ensure no breaking changes

## Risks

1. **Documentation drift**: The ADR might get out of sync with implementation. Mitigated by keeping the non-goals section concise and tied to verifiable facts (what the code does not do).

2. **Misunderstanding of scope**: Future contributors might assume signature support exists if the docs don't clearly state otherwise. The new non-goals section addresses this directly.

3. **Test file location**: The issue specifies `module_signature_nongol.rs` but doesn't specify exact test content. The tests should document the actual behavior (path-based resolution only) and not introduce any new behavior. Tests should be self-explanatory given the BDD naming convention already used in the crate.

## Files Summary

| File | Action |
|------|--------|
| `docs/adr/0015-supply-chain-security.md` | Add "Non-Goals: Perl Module Signature Verification" section |
| `crates/perl-module-resolution-uri/src/lib.rs` | Update `IncRoot` struct docs |
| `crates/perl-path-security/src/lib.rs` | Update module-level docs |
| `crates/perl-module-resolution-uri/tests/module_signature_nongol.rs` | Create new test file |

## Effort Estimate

**EASY (1-2 hours)** — All changes are documentation and tests. No algorithmic changes. The issue's builder spec is clear and detailed.
