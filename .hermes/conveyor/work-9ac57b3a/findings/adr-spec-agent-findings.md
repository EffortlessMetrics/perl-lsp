# ADR/Spec Findings — work-9ac57b3a

## What This ADR Decides
Add 3-token lookahead to `is_label_start()` to disambiguate Perl labels from ternary operators and hash constructors by checking if the token after the colon can start a statement.

## Key Decision
Modify `is_label_start()` in `statements.rs` to use `peek_third()` (already implemented) to inspect the token after the colon. If that token is `Question`, `Colon`, `Semicolon`, `Comma`, `FatArrow`, `RightParen`, `RightBracket`, `RightBrace`, or `Eof`, the function returns `false` (not a label start) because these tokens cannot start a valid Perl statement.

## Alternatives Considered

1. **Do nothing** — Reject the fix and accept `expected_colon` errors in CPAN files. This was rejected because the error bucket affects 10 unique CPAN files and the root cause is well-understood.

2. **Add 4+ token lookahead with full statement parse** — Attempt to parse what follows the colon as a statement to definitively determine if it's a label. This was rejected because it would be complex, expensive, and could have cascading effects on error recovery. The 3-token heuristic is sufficient for all known cases.

3. **Track colon context separately in the lexer** — Add metadata to the token stream indicating whether a colon was seen in expression vs. statement context. This was rejected because it would require changes to the lexer/ tokenizer interface, which is more invasive than the proposed parser-only fix.

## Consequences

**Benefits:**
- Reduces `expected_colon` errors from misidentified label colons in ternary/constructor contexts
- Uses existing `peek_third()` infrastructure — surgical, proven pattern
- Does not change label parsing logic, only the heuristic for when to attempt parsing
- All 158 existing tests in `fix_expected_colon.rs` continue to pass

**Tradeoffs:**
- The 3-token heuristic is a heuristic, not a proof — there may be edge cases where valid labels are rejected or invalid labels are accepted
- The fix only addresses label-colon disambiguation; other sources of `expected_colon` errors remain
- Tests in `fix_expected_colon.rs` all pass (158/158), suggesting they were written for issue #4169 (ternary + postfix call) rather than label-colon disambiguation — the fix may not be verified against actual failing patterns until the baseline is re-run

## Acceptance Criteria

1. `is_label_start()` returns `false` when the 3rd token after an `Identifier Colon` sequence is `Question`, `Colon`, `Semicolon`, `Comma`, `FatArrow`, `RightParen`, `RightBracket`, `RightBrace`, or `Eof`
2. `is_label_start()` continues to return `true` for valid label patterns like `LABEL: { block }`, `LABEL: while (1) { }`, `LABEL: if $x { }`, `LABEL: print "hi" if $debug;`
3. All 158 existing tests in `fix_expected_colon.rs` pass after the fix
4. The `expected_colon` error bucket count decreases when the CPAN corpus baseline is re-generated