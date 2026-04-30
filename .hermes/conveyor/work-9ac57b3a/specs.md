# Spec — work-9ac57b3a: Parser Label/Ternary Disambiguation

## Feature Description

Improve the Perl parser's disambiguation of label colons (`Identifier Colon`) from ternary/hash constructor colons by adding 3-token lookahead to `is_label_start()`.

**The problem:** `is_label_start()` currently only checks for `Identifier Colon` (2-token lookahead). When it returns `true`, the parser consumes the colon as a label colon. This is wrong when the colon actually belongs to a ternary operator (`foo: ? bar : baz`) or hash constructor (`word: => 'value'`).

**The fix:** Check the token AFTER the colon (3rd token). If that token cannot start a statement, `is_label_start()` returns `false`, preventing incorrect label parsing.

## Acceptance Criteria

1. **`is_label_start()` returns `false` for invalid patterns:**
   - `foo: ? bar : baz` — token after colon is `?` (ternary question)
   - `word: => 'value'` — token after colon is `=>` (fat arrow)
   - `foo: : bar` — token after colon is another `:` (colon)
   - `foo: ;` — token after colon is `;` (semicolon)
   - `foo: )` — token after colon is `)` (right paren)
   - `foo: ]` — token after colon is `]` (right bracket)
   - `foo: }` — token after colon is `}` (right brace)
   - `foo:` at end of file — token after colon is `Eof`

2. **`is_label_start()` returns `true` for valid label patterns:**
   - `FOO: my $x = 1;` — token after colon is identifier (valid statement start)
   - `LABEL: { block }` — token after colon is `{` (block start)
   - `LABEL: (expr)` — token after colon is `(` (paren expr)
   - `LABEL: while (1) { }` — token after colon is keyword (valid statement start)
   - `LABEL: if $x { }` — token after colon is keyword (valid statement start)
   - `LABEL: print "hi" if $debug;` — token after colon is identifier with statement modifier (valid)

3. **Existing tests pass:** All 158 tests in `fix_expected_colon.rs` continue to pass after the fix.

4. **Error reduction:** The `expected_colon` error bucket count decreases when the CPAN corpus baseline is re-generated after the fix.

## Non-Goals

- This fix does NOT address all sources of `expected_colon` errors — only those caused by label-colon disambiguation
- This fix does NOT change how valid labels are parsed — it only affects whether the parser attempts to parse a label in ambiguous cases
- This fix does NOT add new tests for the actual failing CPAN patterns (deferred to implementation phase)
- This fix does NOT update the issue title to reflect the actual scope (10 files, not 8)

## Dependencies

- `peek_third()` method exists in `perl-tokenizer/src/token_stream.rs` (line 213) and is already used in 6+ places in the codebase
- `TokenKind::Eof` exists (not `TokenKind::None` as sometimes referenced)
- `fix_expected_colon.rs` test file exists with 158 tests
- Branch `feat/work-9ac57b3a/parser:-expected_colon-errors-suggest-te` is created

## Implementation Notes

The fix is contained entirely within `is_label_start()` in `crates/perl-parser-core/src/engine/parser/statements.rs`. No changes to:
- `parse_labeled_statement()` — the actual label parsing logic
- `perl-tokenizer` or `perl-lexer` — only parser heuristics
- The lexer/token interface — uses existing `peek_third()` infrastructure

## Verification

1. Run `cargo test -p perl-parser-core -- fix_expected_colon` — all 158 tests pass
2. Run `cargo test -p perl-parser-core -- fix_nested_ternary_label_4169` — all tests pass
3. Run `cargo test -p perl-parser-core` — full test suite passes
4. Re-generate CPAN corpus baseline to verify error reduction