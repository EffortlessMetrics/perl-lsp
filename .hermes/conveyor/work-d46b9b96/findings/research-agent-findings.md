# Research Findings — work-d46b9b96

## Issue Summary
The DAP `evaluate` request's "safe eval" feature performs syntactic validation only (admission control), not interpreter sandboxing. When `allowSideEffects=true`, validation is completely skipped and expressions are sent directly to the Perl debugger process. Internal code comments acknowledge this, but public-facing documentation does not make it clear.

## Relevant Codebase Areas
- `crates/perl-dap/src/debug_adapter/evaluation.rs:59-102` — handle_evaluate() with the validation skip
- `crates/perl-dap/src/debug_adapter/safe_eval.rs` — syntactic validation (admission control)
- `docs/tutorials/DAP_USER_GUIDE.md:477` — mentions "safe eval" without details
- `crates/perl-dap/tests/security_evaluate_tests.rs` — existing security tests (gap: no boundary documentation)

## Key Findings
1. **Validation is bypassed when `allowSideEffects=true`** (evaluation.rs:59-87) — the entire `validate_safe_expression()` check is skipped
2. **Internal comments exist** acknowledging this is "admission control, not a sandboxed interpreter boundary" (safe_eval.rs:5, evaluation.rs:58)
3. **Public docs are silent** on the limitation — DAP_USER_GUIDE.md just says "safe eval" without explaining what it means
4. **Design is intentional for debugger context** — the debugger IS the runtime, so sending expressions there is expected behavior

## Proposed Approach
Option A (documentation + tests, ~2h) is recommended because the code is already self-aware about the limitation, the gap is in public-facing docs/tests, and the tradeoff is valid for a debugger context. This involves clarifying the user-facing documentation and adding tests that explicitly document the validation boundary.

## Top Risks
1. **Compliance may require sandboxing** — if security compliance mandates Safe.pm compartment or subprocess isolation, Option A won't suffice (would need Option B at 4-6h or Option C at 8-12h)
2. **User expectations** — "safe eval" implies more isolation than what's actually provided; clear docs prevent misunderstandings
3. **Related issue #3619** — SafeExecutor restrictions should be reviewed alongside this decision

## Scope
**Covers**: Documentation clarification in user guide, clarifying tests in security_evaluate_tests.rs  
**Does NOT cover**: Safe.pm compartment implementation (Option B), subprocess isolation (Option C), changes to validation logic, issue #3619 (SafeExecutor)
