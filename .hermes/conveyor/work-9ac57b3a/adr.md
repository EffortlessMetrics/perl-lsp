# ADR-0017: Add 3-Token Lookahead to `is_label_start()` for Disambiguation

## Status
Proposed

## Context

The parser produces `expected_colon` errors when it incorrectly identifies a label (`Identifier Colon` sequence) in contexts where the colon actually belongs to a ternary operator or other expression, not a statement label.

The `is_label_start()` function in `statements.rs` (lines 1030-1047) uses only 2-token lookahead:
- Token 1 (peek): Is it an `Identifier`?
- Token 2 (peek_second): Is it a `Colon`?

If both are true, `is_label_start()` returns `true`, causing `parse_labeled_statement()` to consume the colon. This is incorrect when the colon belongs to:
- A ternary operator: `foo: ? bar : baz`
- A hash constructor: `word: => 'value'`
- A comma-separated list expression

The baseline shows 10 unique CPAN files affected by this error bucket, including `IO/Socket/SSL/Intercept.pm` and `Regexp/Common/SEN.pm`.

## Decision

Modify `is_label_start()` to perform 3-token lookahead using the existing `peek_third()` method (already implemented in `perl-tokenizer/src/token_stream.rs` line 213). If the token after the colon cannot start a statement, return `false` instead of incorrectly treating the sequence as a label start.

**Invalid 3rd tokens (cannot start a statement):**
- `TokenKind::Question` — ternary `?` operator
- `TokenKind::Colon` — another colon (double label or ternary else-part)
- `TokenKind::Semicolon` — immediate statement end
- `TokenKind::Comma` — expression continuation
- `TokenKind::FatArrow` — hash key-value context
- `TokenKind::RightParen` — closing paren
- `TokenKind::RightBracket` — closing bracket
- `TokenKind::RightBrace` — closing brace (orphan closing brace, not a statement)
- `TokenKind::Eof` — end of input, no statement follows

**Valid 3rd tokens (can start a statement):**
- `TokenKind::LeftBrace` — block: `LABEL: { ... }`
- `TokenKind::LeftParen` — paren expr: `LABEL: (expr)`
- Keywords: `if`, `while`, `for`, `my`, `print`, etc.
- Identifiers, variables, operators, etc.

## Implementation

```rust
fn is_label_start(&mut self) -> bool {
    if self.peek_kind() != Some(TokenKind::Identifier) {
        return false;
    }
    if let Ok(second_token) = self.tokens.peek_second() {
        if second_token.kind == TokenKind::Colon {
            // Check 3rd token — if it can't start a statement, not a label
            if let Ok(third_token) = self.tokens.peek_third() {
                match third_token.kind {
                    TokenKind::Question
                    | TokenKind::Colon
                    | TokenKind::Semicolon
                    | TokenKind::Comma
                    | TokenKind::FatArrow
                    | TokenKind::RightParen
                    | TokenKind::RightBracket
                    | TokenKind::RightBrace
                    | TokenKind::Eof => return false,
                    _ => {}
                }
            }
            return true;
        }
    }
    false
}
```

## Consequences

### Tradeoffs

**Benefits:**
- Reduces spurious `expected_colon` errors from misidentified label colons
- Uses existing, proven infrastructure (`peek_third()` is already used in 6+ places)
- Surgical change — only modifies the disambiguation heuristic, not the label parsing logic
- All 158 existing tests in `fix_expected_colon.rs` continue to pass

**Risks:**
- The heuristic may have edge cases not covered by current tests
- Only addresses label-colon disambiguation; other `expected_colon` error sources remain
- Tests in `fix_expected_colon.rs` all pass (158/158), suggesting they were written for issue #4169 (ternary + postfix call) rather than label-colon disambiguation specifically
- Baseline scope discrepancy: issue title says "8 files" but there are actually 10 unique files

### Alternatives Considered

1. **4+ token lookahead with full statement parse** — Parse what follows the colon as a statement to definitively determine if it's a label. Rejected as too complex and potentially disruptive to error recovery.

2. **Lexer-level colon context tracking** — Add metadata to tokens indicating colon context. Rejected as more invasive, requiring changes to the lexer/tokenizer interface.

3. **No fix (accept errors)** — Accept `expected_colon` errors in affected CPAN files. Rejected because the root cause is well-understood and the fix is straightforward.

## Notes

- `peek_third()` is cached just like `peek_second()` — same pattern used throughout the codebase
- `TokenKind::Eof` is the correct token kind (not `TokenKind::None` as mentioned in some notes)
- `RightBrace` is correctly included in the invalid list: `LABEL: }` is invalid Perl (orphan closing brace)
- Statement modifiers ARE valid after a label: `LABEL: print "hi" if $debug;` — here the token after colon is `print` (Identifier), which is not in the invalid list