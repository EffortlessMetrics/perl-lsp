# Mutation Testing Improvements for Issue #155

## Summary

Added 19 comprehensive mutation hardening tests targeting the `remove_overlapping_tokens` function in the `perl-lsp-semantic-tokens` crate to improve mutation testing score from 78% toward the 85% target.

## Changes Made

### File: `crates/perl-lsp-semantic-tokens/src/semantic_tokens.rs`

Added 19 new mutation hardening tests to the existing test module, specifically targeting:

1. **FnValue Mutations (71% of survivors)** - Tests that ensure all return paths and push operations are exercised:
   - Empty input handling (`mutation_hardening_empty_input`)
   - Single token preservation (`mutation_hardening_single_token`)
   - Cascading overlaps with multiple push operations (`mutation_hardening_three_tokens_cascading`)
   - Systematic removal of shorter tokens (`mutation_hardening_systematic_removal`)
   - Interleaved non-overlapping tokens (`mutation_hardening_interleaved_no_overlap`)

2. **BinaryOperator Mutations (25% of survivors)** - Tests that verify all comparison operations:
   - Boundary conditions (`mutation_hardening_exact_boundary`, `mutation_hardening_boundary_minus_one`)
   - Overlap detection (`mutation_hardening_single_char_overlap`, `mutation_hardening_partial_overlap_length_determines_winner`)
   - Length comparisons (`mutation_hardening_equal_length_keeps_first`)
   - Line equality checks (`mutation_hardening_different_lines_no_overlap`)
   - Sort order validation (`mutation_hardening_sort_order`, `mutation_hardening_sort_order_same_line`, `mutation_hardening_mixed_line_position_sort`)

3. **Edge Cases** - Tests for corner cases that might harbor mutations:
   - Zero-length tokens (`mutation_hardening_zero_length_token`, `mutation_hardening_multiple_zero_length`)
   - Large position values (`mutation_hardening_large_positions`)
   - Metadata preservation (`mutation_hardening_preserves_metadata`)
   - Adjacent non-overlapping tokens (`mutation_hardening_adjacent_non_overlapping`)

## Test Results

All 27 tests (8 existing + 19 new) pass successfully:

```
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### New Tests Added

1. `mutation_hardening_empty_input` - Verifies empty input handling
2. `mutation_hardening_single_token` - Ensures single token preservation
3. `mutation_hardening_adjacent_non_overlapping` - Tests non-overlapping adjacent tokens
4. `mutation_hardening_exact_boundary` - Tests exact boundary conditions
5. `mutation_hardening_single_char_overlap` - Single character overlap handling
6. `mutation_hardening_partial_overlap_length_determines_winner` - Length-based winner selection
7. `mutation_hardening_equal_length_keeps_first` - Equal length tie-breaking
8. `mutation_hardening_different_lines_no_overlap` - Cross-line non-overlap
9. `mutation_hardening_three_tokens_cascading` - Cascading overlap scenarios
10. `mutation_hardening_zero_length_token` - Zero-length token handling
11. `mutation_hardening_multiple_zero_length` - Multiple zero-length tokens
12. `mutation_hardening_large_positions` - Overflow prevention
13. `mutation_hardening_sort_order` - Multi-line sort validation
14. `mutation_hardening_sort_order_same_line` - Same-line sort validation
15. `mutation_hardening_systematic_removal` - Progressive replacement
16. `mutation_hardening_interleaved_no_overlap` - Interleaved token preservation
17. `mutation_hardening_boundary_minus_one` - Off-by-one boundary testing
18. `mutation_hardening_preserves_metadata` - Token metadata preservation
19. `mutation_hardening_mixed_line_position_sort` - Complex sort scenarios

## Mutation Coverage Analysis

### Targeted Mutations

The tests specifically target mutations in these code locations:

**Line 310** - `sort_by_key` comparison:
- Tests: `mutation_hardening_sort_order`, `mutation_hardening_sort_order_same_line`, `mutation_hardening_mixed_line_position_sort`

**Line 322** - `line == last_line` condition:
- Tests: `mutation_hardening_different_lines_no_overlap`

**Line 322** - `start_char < last_start + last_length` comparison:
- Tests: `mutation_hardening_adjacent_non_overlapping`, `mutation_hardening_exact_boundary`, `mutation_hardening_single_char_overlap`, `mutation_hardening_boundary_minus_one`

**Line 324** - `length > last_length` comparison:
- Tests: `mutation_hardening_partial_overlap_length_determines_winner`, `mutation_hardening_equal_length_keeps_first`

**Line 325** - `result.pop()` operation:
- Tests: `mutation_hardening_partial_overlap_length_determines_winner`, `mutation_hardening_systematic_removal`

**Line 326** - `result.push(token)` in replacement:
- Tests: All tests that trigger replacement logic

**Line 330** - `result.push(token)` in else branch:
- Tests: `mutation_hardening_interleaved_no_overlap`, `mutation_hardening_adjacent_non_overlapping`

**Line 333** - `result.push(token)` in initial branch:
- Tests: `mutation_hardening_single_token`, `mutation_hardening_empty_input`

**Line 337** - `result` return:
- Tests: All tests verify the return value

## Expected Impact

### Mutation Score Improvement

Based on the issue analysis:
- **Current Score**: 78%
- **Target Score**: 82-85%
- **Survivors Targeted**: 41/59 (69%) concentrated in `remove_overlapping_tokens`

The 19 new tests comprehensively cover:
- All comparison operators (BinaryOperator mutations)
- All function return paths (FnValue mutations)
- Edge cases and boundary conditions
- Complex interaction scenarios

### Quality Improvements

1. **Better Edge Case Coverage**: Tests now cover zero-length tokens, boundary conditions, and overflow scenarios
2. **Explicit Mutation Killing**: Each test documents which specific mutation it targets
3. **Regression Prevention**: Comprehensive test suite prevents future mutations from surviving
4. **Code Behavior Documentation**: Tests serve as documentation for the overlap removal algorithm

## Integration with Existing Tests

The new tests complement the existing 8 tests:
- Existing tests cover basic functionality and common cases
- New tests focus on mutation-specific scenarios and edge cases
- No redundancy - each test targets specific code paths
- All tests use the same helper function (`tok()`) for consistency

## Running the Tests

```bash
# Run all semantic token tests
cargo test -p perl-lsp-semantic-tokens

# Run only mutation hardening tests
cargo test -p perl-lsp-semantic-tokens mutation_hardening

# Run mutation testing (if cargo-mutants is installed)
cargo mutants -p perl-lsp-semantic-tokens --file 'src/semantic_tokens.rs'
```

## Verification

To verify the mutation score improvement:

```bash
cargo install cargo-mutants
cargo mutants -p perl-lsp-semantic-tokens --file 'src/semantic_tokens.rs'
```

Expected outcome: Mutation score should increase from 78% to 82-85% range, with significantly fewer survivors in the `remove_overlapping_tokens` function.

## Future Work

If mutation score doesn't reach 85% target:
1. Analyze remaining survivors using `cargo mutants --list`
2. Add property-based tests using `proptest` for comprehensive coverage
3. Consider mutation-specific assertions for complex state transitions
4. Review and strengthen tests for encoding and walking functions

## Related

- Issue: #155 - Mutation Testing: Opportunity to improve score from 78% toward 85% target
- Focus: 41/59 survivors (69%) in `remove_overlapping_tokens` function
- Mutation Types: FnValue (71%), BinaryOperator (25%)
