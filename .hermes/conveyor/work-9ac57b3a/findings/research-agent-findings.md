# Research Findings — work-9ac57b3a

## Issue Summary
The parser produces `expected_colon` errors when `is_label_start()` incorrectly identifies an `Identifier Colon` pattern as a label, even when what follows the colon cannot start a valid statement. This causes `parse_labeled_statement()` to consume a colon that actually belongs to a ternary operator or other expression context.

## Relevant Codebase Areas
- **`crates/perl-parser-core/src/engine/parser/statements.rs`** — `is_label_start()` (line 1030) and `parse_labeled_statement()` (line 1050) — the root cause location
- **`crates/perl-parser-core/src/engine/parser/expressions/precedence.rs`** — ternary parsing via `parse_ternary()` and `parse_ternary_with()` where `expected_colon` error fires
- **`crates/perl-tokenizer/src/token_stream.rs`** — already has `peek_third()` method for 3-token lookahead (lines 213-225)
- **`crates/perl-parser-core/tests/fix_expected_colon.rs`** — 1214-line test file driven by this error bucket
- **`crates/perl-parser-core/tests/fix_nested_ternary_label_4169.rs`** — related fix for ternary/postfix disambiguation (issue #4169)

## Key Findings
1. **`is_label_start()` only uses 2-token lookahead**: it checks `peek() == Identifier` and `peek_second() == Colon`, but never checks what comes AFTER the colon.
2. **`peek_third()` already exists** in `TokenStream` — no need to add it.
3. **The fix is straightforward**: add a 3rd-token lookahead check in `is_label_start()`. If the token after the colon is `?`, `:`, `;`, `,`, `=>`, `)`, `]`, `}`, or EOF — it's NOT a label start.
4. **4 unique files** in the `expected_colon` bucket: `IO/Socket/SSL/Intercept.pm`, `Mail/Address.pm`, `Parse/RecDescent.pm`, `Regexp/Common/SEN.pm`.
5. **Related issue #4169** was partially about the same problem but focused on postfix call handling, not label detection.

## Proposed Approach
Modify `is_label_start()` in `statements.rs` to add a 3-token lookahead check. After confirming `peek() == Identifier` and `peek_second() == Colon`, check `peek_third()`. If the third token is one of the "invalid statement starters" (Question, Colon, Semicolon, Comma, FatArrow, RightParen, RightBracket, RightBrace, or EOF), return `false`. Otherwise, return `true` as before.

This approach is chosen because it is surgical — it only changes the disambiguation heuristic without modifying the parsing logic itself — and uses existing infrastructure (`peek_third()` already exists).

## Top Risks
1. **Breaking valid labels**: the list of "invalid 3rd tokens" must be exactly right. For example, `Label: if $cond { }` IS valid (statement modifier), so we must NOT put `If` in the invalid list.
2. **Token stream side effects**: `peek_third()` caches the third token — we need to ensure this doesn't interfere with other parsing that uses `peek_second()`.
3. **Other error sources**: there may be other places producing `expected_colon` errors that this fix won't address.

## Scope
- **Fixes**: `is_label_start()` disambiguation in `statements.rs`
- **Tests**: add specific patterns from the CPAN files to `fix_expected_colon.rs`
- **Does NOT fix**: other error buckets, other parts of the parser
