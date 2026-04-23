# Context: #5498

## Decision log

**Decision:** Extend existing dap_feature_flag_coverage_tests.rs inline (Option 3)
- **Rationale:** DAP coverage tests are organized in a single test file for cohesion. Adding function breakpoint tests to this existing file keeps related tests together and follows project pattern.
- **Alternative rejected:** Create new test file — would fragment DAP test infrastructure and duplicate existing harness (setup_dap_server, has_feature, etc.).

**Decision:** Test protocol-layer acceptance (serde round-trip), not runtime evaluation
- **Rationale:** Issue scope is about testing that the protocol accepts condition strings without panicking. Runtime evaluation of conditions is a separate layer (daemon, Perl interpreter). Protocol tests validate that the protocol layer is spec-compliant (accepts FunctionBreakpoint with condition field).
- **Scope boundary:** Protocol tests verify "accepts condition"; runtime tests (if needed) would verify "evaluates condition correctly." This spec covers protocol layer only.

**Decision:** Use simple, realistic condition expressions (scalar variable, compound expression)
- **Rationale:** Tests document expected behavior for users. Simple conditions are common; compound expressions (defined + logical AND) represent real use cases developers would write.

## Objections addressed

**Concern (from oppositional review):** Tests are just serialization checks, not behavior checks
- **Resolution:** Correct. Protocol-layer tests verify that the protocol accepts these inputs. Runtime behavior (whether conditions actually guard breakpoints) is tested elsewhere. This spec documents protocol compliance, not debugger behavior.

**Concern (from oppositional review):** Tests pass trivially and document nothing
- **Resolution:** Tests include comments documenting what behavior is Perl-specific (scalar truthiness, expression evaluation). Comments guide developers on what is handled by protocol vs. runtime.

**Concern (from oppositional review):** Wrong test layer
- **Resolution:** Acknowledged. These are protocol tests, not adapter/runtime tests. That's intentional — fill the gap in protocol-layer test coverage for function breakpoints (which currently have none, while source breakpoints have extensive coverage).

**Maintainer resolution:** Despite diaboli DEFER verdict, maintainer marked as ALIGNED with "builder-ready" label
- **Interpretation:** Maintainer approved building despite diaboli concern. Proceed with implementation.

## Research findings

**Confirmed facts:**
- File `crates/perl-dap/tests/dap_feature_flag_coverage_tests.rs` exists (1061 lines)
- Hit_condition tests for SOURCE breakpoints cover lines 254-358
- Logpoints section starts at line 360
- Feature `dap.breakpoints.function` is registered in features.toml (advertised, ga maturity)
- No existing tests for SetFunctionBreakpoints in the file
- Existing test pattern uses `setup_dap_server()`, `has_feature()`, `server.request()`

**DAP protocol structure:**
- SetFunctionBreakpointsArguments contains Vec<FunctionBreakpoint>
- FunctionBreakpoint struct has fields: name, condition: Option<String>, hit_condition: Option<String>
- Protocol accepts any string in condition field (validation is deferred to runtime/daemon)

## Related issues

- #5496 — Parser error recovery tests (parallel coverage work)
- #5499 — Completion scope ranking tests (parallel coverage work)

## Test pattern consistency

Tests follow established pattern in dap_feature_flag_coverage_tests.rs:
1. Feature gate check with early return
2. Setup test fixture (setup_dap_server)
3. Create protocol arguments with test data
4. Call server.request() and parse response
5. Assert response structure matches expectations
6. Return Ok(()) for success

This consistency ensures tests are maintainable and aligned with DAP test conventions.

## Diaboli verdict handling

The advocatus diaboli review recommended DEFER due to:
1. No user-reported issues
2. Protocol is spec-correct as-is
3. Tests just verify acceptance, not behavior

Maintainer overrode DEFER with ALIGNED verdict, indicating:
1. Coverage gaps are valuable for regression prevention
2. Even spec-correct features need test coverage
3. Building strengthens confidence before public alpha

Proceeding per maintainer decision.
