# Plan Review Findings — work-9ac57b3a

## Overall Assessment
**feasible with modifications** — The core fix (adding 3-token lookahead to `is_label_start()`) is technically sound and correctly targets the root cause. However, there are significant scope discrepancies, incomplete token analysis, and a missing verification step before implementation.

## Scope Assessment

**Issue title:** "parser: expected_colon errors suggest ternary/label disambiguation needs improvement (8 files)"

**Actual scope:** 10 unique files (20 entries total), not 8. The research agent correctly identified 2 of the affected files (IO/Socket/SSL/Intercept.pm, Regexp/Common/SEN.pm), incorrectly listed 2 files not in the baseline (Mail/Address.pm, Parse/RecDescent.pm), and missed 6 others (Mojo/Log.pm, Mojolicious/Plugin/DefaultHelpers.pm, Mojolicious/Routes.pm, POE/Test/Loops/z_leolo_wheel_run.pm, Sort/BySpec.pm, Test/Needs.pm, AnyEvent/Handle.pm, AnyEvent/Log.pm).

## What Works

1. **Root cause is correctly identified** — `is_label_start()` in `statements.rs` (lines 1030-1047) uses only 2-token lookahead (`Identifier Colon`) without verifying that what follows the colon can start a statement. This is confirmed.

2. **`peek_third()` exists and is usable** — Verified at `perl-tokenizer/src/token_stream.rs` line 214. It's cached just like `peek_second()`, so no new infrastructure needed.

3. **The 3-token lookahead approach is correct** — Adding a check that the token after the colon cannot be `?`, `:`, `;`, `,`, `=>`, `)`, `]`, `}`, or EOF will correctly disambiguate labels from ternary operators.

4. **The invalid token list is mostly correct** — `Question`, `Colon`, `Semicolon`, `Comma`, `FatArrow`, `RightParen`, `RightBracket`, `RightBrace`, and `Eof` cannot start a statement in Perl.

5. **Tests exist and provide coverage** — 158 tests in `fix_expected_colon.rs` pass, ensuring the fix won't break existing patterns.

## What Doesn't Work

1. **`RightBrace` concern in verification comment is incorrect** — The verification comment claims `RightBrace` shouldn't be in the invalid list because "a label CAN be followed by a closing brace." This is wrong:
   - `LABEL: { say "hello" }` has `LeftBrace` after the colon (not `RightBrace`)
   - `LABEL: }` is invalid Perl (orphan closing brace) and should be rejected
   - The concern conflates the token AFTER the colon with the token AFTER the block

2. **TokenKind::Eof is used, not TokenKind::None** — The plan mentions "Eof / None" but `None` doesn't exist in the codebase. Only `TokenKind::Eof` exists (line 365 of `perl-token/src/lib.rs`).

3. **Missing statement modifier check in invalid tokens** — The plan doesn't mention statement modifiers. Consider: `LABEL: print "hi" if $debug;` — here the token after the colon is `print` (Identifier), which CAN start a statement (it's valid). So this is actually fine as-is. But what about `LABEL: if $x { }`? The token after colon is `if` which CAN start a statement. So the current approach is correct.

4. **Baseline may be stale** — All 158 tests pass, but the baseline still shows 20 `expected_colon` entries. This suggests either the tests don't cover the actual failing patterns, or the baseline needs regeneration.

5. **Incomplete analysis of other error sources** — The plan acknowledges "there may be other places producing `expected_colon` errors beyond label disambiguation" but doesn't investigate. This could mean the fix reduces the bucket count without eliminating it.

## Top Risks

1. **Risk:** The fix only addresses label-colon disambiguation, but other code paths may produce `expected_colon` errors.
   - **Likelihood:** medium — The baseline shows 20 entries across 10 files; it's unclear what percentage is label-related.
   - **Impact:** Partial reduction in error count, leading to incomplete fix.
   - **Mitigation:** Before implementing, extract and analyze actual failing patterns from the CPAN files to determine what percentage is label-related. If most errors are from other causes, the approach needs adjustment.

2. **Risk:** The list of "invalid tokens after colon" may be incomplete.
   - **Likelihood:** medium — The plan doesn't consider all edge cases (e.g., `Label: then` where `then` is a keyword used as a label in some legacy Perl, though unlikely).
   - **Impact:** False positives (rejecting valid labels) or false negatives (accepting invalid label-like constructs).
   - **Mitigation:** Add comprehensive test cases for valid label patterns to ensure the invalid token list doesn't break them. The test file should include `LABEL: { block }`, `LABEL: while (1) { }`, `LABEL: if $x { }`, etc.

3. **Risk:** The test file `fix_expected_colon.rs` may not cover the actual failing patterns.
   - **Likelihood:** high — All 158 tests pass, yet the baseline still shows errors. This strongly suggests the tests were written for issue #4169 (ternary + postfix call disambiguation), not for the label-colon disambiguation issue.
   - **Impact:** The fix may be "tested" but not against the actual failure modes.
   - **Mitigation:** Before implementing, extract specific Perl code snippets from the 10 failing files that produce `expected_colon` errors. Add these as test cases to verify the fix actually works.

4. **Risk:** `peek_third()` is cached but `on_stmt_boundary()` clears lookahead.
   - **Likelihood:** low — The same caching pattern is used for `peek_second()` throughout the codebase.
   - **Impact:** Token stream inconsistency (unlikely given existing usage patterns).
   - **Mitigation:** Ensure the fix is placed where statement boundaries are properly managed.

## Edge Cases

1. **`LABEL: { }` (empty block after label)** — The token after colon is `LeftBrace`. This should be accepted as a valid label (even though the block is empty). The fix handles this correctly since `LeftBrace` is not in the invalid list.

2. **`LABEL: }` (orphan closing brace)** — The token after colon is `RightBrace`. This should be rejected. The fix handles this correctly since `RightBrace` IS in the invalid list.

3. **`LABEL: if $x { }` (label followed by if statement)** — The token after colon is `If`. This should be accepted as a valid label. The fix handles this correctly since `If` is not in the invalid list.

4. **`LABEL: unless $x { }` (label followed by unless statement)** — Same as above.

5. **`LABEL: while (1) { }` (label followed by while loop)** — Same as above.

6. **`foo: ? bar : baz` (identifier-colon before ternary)** — The token after colon is `Question`. This should be rejected. The fix handles this correctly since `Question` IS in the invalid list.

7. **`word: => 'value'` (fat arrow after identifier-colon)** — The token after colon is `FatArrow`. This should be rejected. The fix handles this correctly since `FatArrow` IS in the invalid list.

8. **Label at end of file without semicolon** — The token after colon is `Eof`. This should be rejected since there's no statement after the label. The fix handles this correctly since `Eof` IS in the invalid list.

9. **Double label `FOO: BAR: statement`** — `FOO:` has `Identifier Colon`, and the token after colon is `BAR:` (Identifier). Since `Identifier` is not in the invalid list, this would be accepted as a label. Is `FOO: BAR: statement` valid Perl? Actually, this is parsed as `FOO:` (label) followed by `BAR: statement` (another label + statement). So the second colon would be checked by `is_label_start()` recursively.

## Recommendations

1. **Clarify scope before proceeding** — The issue title says "8 files" but there are 10. Update the issue title or document the actual scope.

2. **Extract and analyze actual failing patterns** — Before implementing the fix, run the parser against the 10 failing CPAN files and extract the specific code patterns that cause `expected_colon` errors. This will confirm whether they are all label-colon disambiguation issues or a mix of causes.

3. **Add test cases for actual failing patterns** — Once the failing patterns are extracted, add them to `fix_expected_colon.rs` or create a new test file. These tests should FAIL before the fix and PASS after.

4. **Verify `RightBrace` handling** — The concern in the verification comment about `RightBrace` is incorrect, but confirm that `LABEL: { }` (empty block) parses correctly after the fix.

5. **Add comprehensive valid-label tests** — Ensure the fix doesn't break: `LABEL: { }`, `LABEL: while (1) { }`, `LABEL: if $x { }`, `LABEL: for my $i (@arr) { }`, `LABEL: do { }`, `LABEL: sub foo { }`, etc.

6. **Verify statement modifiers work after labels** — `LABEL: print "hi" if $debug;` is valid Perl. Ensure the fix handles this (the token after colon is `print`/Identifier, which is not in the invalid list).

## Confidence to Proceed

**medium** — The technical approach is sound and correctly identifies the root cause. `peek_third()` exists and the 3-token lookahead pattern is proven in the codebase. However, there is a significant mismatch between the tests (158 passing) and the baseline (20 errors), suggesting the tests don't cover the actual failing patterns. Additionally, the scope discrepancy (8 vs 10 files) and the incorrect `RightBrace` concern in the verification comment raise questions about the thoroughness of prior analysis.

**To raise confidence:**
1. Extract actual failing patterns from the 10 CPAN files
2. Add those patterns as test cases that fail before the fix
3. Implement the fix and verify those new tests pass
4. Verify the existing 158 tests still pass
