# Initial Plan — work-d46b9b96

## Approach

The recommended fix is **Option A** (documentation + tests, ~2 hours) because:

1. The code is already self-aware about the limitation (internal comments exist)
2. The gap is in public-facing documentation/tests, not a hidden vulnerability
3. For a debugger context, sending expressions to the debugger is expected behavior
4. The design tradeoff is conscious and documented internally

### What Option A Covers

1. **Documentation Clarification** (in `docs/tutorials/DAP_USER_GUIDE.md`)
   - Add a section explaining the `evaluate` request security model
   - Clarify that "safe eval" means syntactic validation (admission control), not interpreter sandboxing
   - Explain that `allowSideEffects=true` bypasses validation entirely
   - Note that expressions execute in the debugger process context

2. **Test Documentation** (in `crates/perl-dap/tests/security_evaluate_tests.rs`)
   - Add tests that explicitly document the validation boundary
   - Add tests showing what happens when `allowSideEffects=true` (no validation occurs)

3. **Code Comments** (optional, if needed)
   - The internal comments already exist and are clear

### Specific Changes

1. **`docs/tutorials/DAP_USER_GUIDE.md`**: Add security section explaining the evaluation model
2. **`crates/perl-dap/tests/security_evaluate_tests.rs`**: Add clarifying tests

## Risks

### Risk 1: Compliance Requirements May Mandate Sandboxing
If security compliance requires actual interpreter sandboxing (Safe.pm compartment), Option A won't suffice and we'd need to escalate to Option B (Safe.pm, 4-6h) or Option C (subprocess isolation, 8-12h).

**Mitigation**: The issue description notes this is for a debugger context where the tradeoff is acceptable. Plan review should confirm no compliance mandates require sandboxing.

### Risk 2: Users May Expect Full Isolation
Users reading "safe eval" may expect complete isolation, which isn't provided.

**Mitigation**: Clear documentation of what "safe eval" actually means (syntactic validation only) prevents misunderstandings.

### Risk 3: Related Issue #3619 (SafeExecutor restrictions)
Issue #3619 about SafeExecutor restrictions should be reviewed alongside this decision.

**Mitigation**: Flag as a dependency; coordinate with that issue's resolution.

## Task Breakdown

### Phase 1: Documentation (Primary Fix)
1. Update `docs/tutorials/DAP_USER_GUIDE.md` to add a "Debug Console Security" section
2. Explain the syntactic validation vs sandbox distinction
3. Document the `allowSideEffects` behavior clearly

### Phase 2: Tests (Clarification)
1. Add test to `security_evaluate_tests.rs` that explicitly documents the gap
2. Add comments explaining what the test is verifying (the absence of validation when `allowSideEffects=true`)

### Phase 3: Verification (post-fix)
1. Run `cargo test -p perl-dap` to verify no regressions
2. Verify documentation renders correctly

## Out of Scope

- Option B (Safe.pm compartment implementation) — different work item if compliance requires
- Option C (subprocess isolation) — different work item if compliance requires
- Changes to the actual validation logic — the behavior is intentional
- Issue #3619 (SafeExecutor restrictions) — separate issue to address independently
