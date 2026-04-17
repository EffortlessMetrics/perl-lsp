# Red Test Builder Findings — work-d46b9b96

## Tests Written
- `crates/perl-dap/tests/safe_eval_documentation_clarification_test.rs`: 7 tests covering safe eval documentation clarification

## What the Tests Expect
The tests verify that documentation correctly states:
1. Safe eval provides **SYNTHACTIC VALIDATION ONLY** (admission control)
2. Safe eval does NOT provide interpreter sandboxing or OS-level isolation

The tests check:
- `test_documentation_files_exist`: All referenced docs exist
- `test_safe_eval_contains_syntactic_validation_clarification`: safe_eval.rs code comments contain clarification
- `test_security_spec_clarifies_safe_eval_is_not_sandbox`: DAP_SECURITY_SPECIFICATION.md contains "not a sandboxed" + "interpreter boundary"
- `test_adr_0028_mentions_safe_eval_limitation`: ADR-0028 should mention safe eval limitations
- `test_dap_user_guide_safe_eval_context`: DAP_USER_GUIDE.md should clarify safe eval is syntactic validation only
- `test_adr_0019_safe_eval_limitation_context`: ADR-0019 should clarify safe eval limitations
- `test_documentation_gap_closure_for_safe_eval`: Integration test for overall gap closure

## Current Test Results (RED STATE)
- 4 tests PASS (internal docs already have clarification)
- 3 tests FAIL (user-facing docs lack clarification)

**Failing tests identify the documentation gap:**
1. `test_adr_0028_mentions_safe_eval_limitation` - FAILED
2. `test_adr_0019_safe_eval_limitation_context` - FAILED
3. `test_dap_user_guide_safe_eval_context` - FAILED

## What Code Builder Needs to Do
Add clarifying statements to these user-facing documentation files:

### 1. docs/adr/0028-safe-eval-timeout.md
Add a note explaining that safe eval is:
- Syntactic validation (admission control)
- NOT a sandboxed interpreter boundary
- Works alongside timeout enforcement

### 2. docs/adr/0019-security-first-dap.md
In the "Safe Evaluation Defaults" section, clarify that:
- "Safe evaluation defaults" means expression policy validation
- Safe eval does NOT provide interpreter isolation
- It's syntactic validation + timeout, not sandboxing

### 3. docs/tutorials/DAP_USER_GUIDE.md
When mentioning "safe mode" or "safe eval", add a clarifying note:
- Safe eval checks expression syntax and blocks known dangerous operations
- It does NOT provide OS-level isolation or a sandboxed interpreter
- Use `allowSideEffects: true` for full evaluation with explicit opt-in

## Types Inspected
- `safe_eval.rs`: Already contains proper clarification in code comments ("admission control, not a sandboxed interpreter boundary")
- `DAP_SECURITY_SPECIFICATION.md`: Already contains proper clarification ("not a sandboxed interpreter boundary")
- `DAP_USER_GUIDE.md`: Mentions "safe mode" but lacks clarification
- `ADR-0019`: Mentions "safe evaluation defaults" but lacks clarification
- `ADR-0028`: Focuses on timeout policy, lacks sandbox clarification

## Friction Encountered
- Tests initially failed due to path resolution issues (CARGO_MANIFEST_DIR points to crate, not repo root)
- Text matching across line breaks required adjusting test patterns (e.g., "not a sandboxed" vs "not a sandboxed interpreter boundary")
- Documentation structure varies across files, requiring flexible matching patterns