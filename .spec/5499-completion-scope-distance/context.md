# Context: #5499

## Decision log

**Decision:** Add integration test (Option 2) rather than unit tests or extending existing test
- **Rationale:** Scope-distance ranking is only meaningful with full semantic context (symbol table, scope nesting). Integration test proves end-to-end behavior. Effort (3-4h) is reasonable for a user-visible feature.
- **Alternatives rejected:**
  - Option 1 (unit tests): Would require mocking symbol table and scope nesting; doesn't test actual LSP harness behavior.
  - Option 3 (add both): Redundant effort; integration test covers unit test cases.

**Decision:** Create new test function instead of extending test_completion_ranking()
- **Rationale:** test_completion_ranking() tests special variables ($_, $$, $@) with simple ranking. Adding nested scope test case to same function would combine two different scenarios (special vars + user vars) in one test. Separate function is clearer.
- **Tradeoff:** Slightly longer test file, but better isolation and documentation.

**Decision:** Assert sort_text ordering rather than completion item list position
- **Rationale:** sort_text is the stable ordering mechanism used by LSP clients. Directly asserting sort_text order proves the ranking computation is working. Assertion is specific and debuggable.
- **Why not list position:** List positions depend on filtering behavior; sort_text is the direct output of completion provider.

## Objections addressed

**Concern (from oppositional review):** sort_text format is an implementation detail
- **Resolution:** True, but it's the only observable mechanism for ranking. If sort_text format changes in future refactoring, test needs update — that's correct (test documents the implementation). The intent (closer scope ranks higher) remains.

**Concern (from oppositional review):** Integration test is expensive and fragile
- **Resolution:** True, integration tests are costly. But user-visible behavior (completion ranking) should be tested at integration level. Unit tests of `compute_scope_distance()` would be insufficient because scope distance only matters in context of full symbol table + LSP response.

**Concern (from oppositional review):** LSP harness adds noise to a simple ranking question
- **Resolution:** Acknowledged. But the real behavior users see is LSP completion responses. Testing at LSP layer proves what matters.

**Maintainer override:** Despite oppositional challenge, maintainer marked ALIGNED with builder-ready
- **Interpretation:** Maintainer approved building despite concerns about test cost. Proceed per maintainer decision.

**Diaboli verdict:** BUILD (overruling both test economy and degree-of-separation concerns)
- **Rationale:** Completion ranking is user-facing; regressions are costly; integration test validates end-to-end behavior.

## Research findings

**Confirmed facts:**
- File `crates/perl-lsp-rs/tests/lsp_completion_tests.rs` exists (1534 lines)
- test_completion_ranking() is at line 1008, ends at line 1055
- test_incremental_completion() starts at line 1057
- Insertion point after 1055 is clear
- Completion test harness uses: start_lsp_server, initialize_lsp, send_notification, send_request, completion_items, drain_until_quiet
- Sort_text format documented in comments: "0_" for special vars, "1<distance>_name" for regular vars
- No existing tests for variable scope-distance ranking

**Completion provider architecture:**
- compute_scope_distance() in variables.rs returns distance metric
- Ranking uses format!(\"1{}_{}\", distance.sort_key(), name)
- Lower sort_text = higher ranking (lexicographic string comparison)

## Related issues

- #5496 — Parser error recovery tests (parallel quality work)
- #5498 — DAP function breakpoint tests (parallel quality work)

## Test pattern consistency

Test follows established pattern in lsp_completion_tests.rs:
1. Create LSP server and initialize
2. Send didOpen notification with test Perl code
3. Drain async messages (wait for processing)
4. Send completion request at specific position
5. Extract completion items via helper
6. Assert on response structure/ordering
7. Return Result for error propagation

This consistency ensures the test is maintainable and aligned with LSP test conventions.

## Threading requirements

Per perl-lsp-rs CLAUDE.md:
- Must use `RUST_TEST_THREADS=2 cargo test ... -- --test-threads=2`
- LSP tests are sensitive to parallelism
- Builder must follow this constraint

## Acceptance criteria alignment

Test directly validates acceptance criteria from the issue:
1. "Variables from immediate scope should rank higher than parent scope" — asserted via sort_text comparison
2. "Completion respects scope nesting" — tested with nested blocks
3. "No test for nested blocks where same variable name appears at multiple scope depths" — addressed with $config shadowing

The test proves the implementation works end-to-end.
