# Maintainer Vision Findings — work-9ac57b3a

## Alignment Assessment
**aligned** — The proposed fix (adding 3-token lookahead to `is_label_start()`) aligns with the codebase's trajectory of iterative parser improvement using existing infrastructure. The approach is surgical, uses proven patterns (`peek_third()` is already used in 6+ places), and correctly targets the root cause.

## Reasoning

### 1. The fix uses existing, proven infrastructure
The `peek_third()` method is already implemented in `perl-tokenizer/src/token_stream.rs` (line 213) and is cached just like `peek_second()`. It is already used extensively in the codebase:
- `expressions/calls.rs` (lines 28, 109, 125, 126, 156, 281) — for disambiguating indirect function calls
- `expressions/unary.rs` (line 38) — for detecting `::` vs `:`
- `variables.rs` (lines 842, 885)

The plan-reviewer correctly notes that the 3-token lookahead approach is "proven in the codebase."

### 2. The fix is surgical and correct
The proposed change only modifies `is_label_start()` to add a disambiguation heuristic — it does NOT change how labels are parsed, just whether the parser attempts to parse a label in ambiguous cases. This is exactly how `is_label_start()` is meant to work: it's a heuristic gate, not a parser.

The root cause is well-identified: `is_label_start()` currently returns `true` for any `Identifier Colon` sequence, but Perl labels MUST be followed by a statement. When the token after `:` is `?`, `:`, `;`, `,`, `=>`, `)`, `]`, `}`, or EOF, it's impossible for a valid statement to follow, so it's not a label.

### 3. The fix aligns with established disambiguation patterns
The codebase already has similar disambiguation patterns:
- `helpers.rs` defines `is_stmt_modifier_kind()` to distinguish statement modifiers
- `unary.rs` uses `peek_third()` to distinguish `::` from `:`
- `calls.rs` uses 3-token lookahead to disambiguate indirect function calls from regular calls

The proposed fix follows this same pattern: use lookahead to disambiguate based on what's syntactically possible.

### 4. The fix does not break valid label parsing
The invalid token list correctly excludes tokens that CAN start statements:
- `LeftBrace` (`{`) — valid for `LABEL: { block }`
- `LeftParen` (`(`) — valid for `LABEL: (expr)`
- Keywords like `If`, `While`, `For`, etc. — valid for `LABEL: if $x { }`
- Identifiers — valid for `LABEL: print "hi"`
- Variables (`$`, `@`, `%`, etc.) — valid

### 5. The crate architecture is respected
The fix is contained entirely within `perl-parser-core`, which is appropriate:
- `perl-parser-core` is the layer for parser heuristics and disambiguation
- The change doesn't touch `perl-tokenizer` or `perl-lexer`
- Tests go in `tests/fix_expected_colon.rs` per the established pattern

## Impact on Codebase Trajectory

### Positive impacts (6 months out):
1. **Fewer spurious `expected_colon` errors** — Real CPAN files like `IO/Socket/SSL/Intercept.pm` and `Regexp/Common/SEN.pm` will parse more cleanly
2. **Better error signals** — When real label-colon issues arise, they'll be caught by the proper error path rather than misidentified as label-colon consumption
3. **Precedent for 3-token lookahead disambiguation** — Future disambiguation needs can follow this pattern
4. **Continued iterative parser hardening** — This continues the trajectory of incremental fixes visible in `fix_nested_ternary_label_4169.rs`, `fix_expected_colon.rs`, etc.

### Risks (6 months out):
1. **Incomplete coverage** — The fix only addresses label-colon disambiguation. Other code paths may produce `expected_colon` errors. The plan-reviewer correctly notes that "there may be other places producing `expected_colon` errors beyond label disambiguation." If this is true, the baseline won't go to zero.
2. **Tests don't cover the actual failure modes** — This is a significant concern. All 158 tests in `fix_expected_colon.rs` pass, yet the baseline shows 20 `expected_colon` entries. This strongly suggests the tests were written for issue #4169 (ternary + postfix call) rather than for the label-colon disambiguation issue. The fix may work but isn't verified against actual failing patterns.

## Recommendations

### 1. Before implementation: Extract actual failing patterns
The most important thing is to verify that the fix actually addresses the baseline errors. The plan-reviewer recommends:
- Run the parser against the 10 failing CPAN files
- Extract specific code patterns that cause `expected_colon` errors
- Add those patterns as test cases that FAIL before the fix

Without this, we won't know if the fix actually reduces the baseline count.

### 2. Add test cases for valid labels
Ensure the fix doesn't break:
- `LABEL: { }` (empty block)
- `LABEL: while (1) { }`
- `LABEL: if $x { }`
- `LABEL: for my $i (@arr) { }`
- `LABEL: print "hi" if $debug;` (statement modifier)

### 3. Verify RightBrace handling
The plan-reviewer correctly identified that the verification agent's concern about `RightBrace` was incorrect. `LABEL: }` is invalid Perl (orphan closing brace) and should be rejected. The fix correctly treats `RightBrace` as an invalid 3rd token.

### 4. Scope correction
Update the issue title: there are 10 unique files (20 entries), not 8. The research agent correctly identified 2 of them but missed 6 others and incorrectly listed 2 that aren't in the baseline.

## Long-Term Impact

### Does it improve or degrade the architecture?
**Improves.** The fix adds a disambiguation heuristic that makes the parser more accurate without changing its fundamental structure. The codebase already has 6+ uses of `peek_third()` for similar purposes — this is consistent with the architecture.

### Does it introduce or pay down technical debt?
**Pays down debt.** The current `is_label_start()` is technically incorrect — its comment says single-colon after identifier is "unambiguously a label" but that's false for ternary/hash constructor contexts. The fix corrects this flawed assumption.

### Does it open or close doors for future work?
**Opens doors.** Better label disambiguation enables future work on:
- More complex label contexts (e.g., `LABEL: expr if cond`)
- Better error recovery when labels are misused
- Potential for label-specific features (goto targets, loop control)

## Questions the Pipeline Should Answer

1. **What percentage of the 20 baseline `expected_colon` entries are actually label-colon disambiguation issues?** If most are from other causes, the fix reduces but doesn't eliminate the bucket.

2. **Why do all 158 tests pass if the issue is active?** The tests in `fix_expected_colon.rs` appear to cover issue #4169 (ternary + postfix call) rather than label-colon disambiguation. This needs investigation before the fix is accepted.

3. **Should the issue title be updated to reflect 10 files instead of 8?** The scope discrepancy is a factual error that should be corrected.

4. **Is the baseline stale or current?** If the baseline hasn't been regenerated since issue #4169 was fixed, it may contain stale errors that were already fixed.

---

## Verdict

**ALIGNED** — The technical approach is sound, uses existing infrastructure, follows established patterns, and correctly targets the root cause. The main concern is that the existing tests don't appear to cover the actual failure modes that the fix addresses. This is a verification gap, not a design flaw. With proper test coverage for the actual failing patterns, this fix should proceed.
