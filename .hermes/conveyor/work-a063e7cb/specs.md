# Specs — work-a063e7cb

## Feature / Behavior Description

Fix the lexer bug in `parse_double_quoted_string` (`crates/perl-lexer/src/lib.rs`) where the `$var{...}` and `$var[...]` branches do not check the return value of `consume_balanced_segment` before pushing a `StringPart`.

When the lexer encounters an incomplete variable reference inside a double-quoted string (e.g., `"$foo{"` or `"$bar[ "`), it must not push a malformed `StringPart::Expression` or `StringPart::ArraySlice` token. Instead, it should silently skip the unbalanced delimiter and continue lexing, relying on the existing error reporting mechanisms for malformed input.

## Acceptance Criteria

1. **Array slice guard**: When lexing `"$foo[ "` (missing closing `]`), the `$var[...]` branch does not push a `StringPart::ArraySlice`. The lexer continues without producing a malformed token.

2. **Hash subscript guard**: When lexing `"$foo{"` (missing closing `}`), the `$var{...}` branch does not push a `StringPart::Expression`. The lexer continues without producing a malformed token.

3. **Valid cases unchanged**: Lexing `"$foo[0]"` still correctly produces `StringPart::ArraySlice("[0]")`. Lexing `"$foo{bar}"` still correctly produces `StringPart::Expression("$foo{bar}")`.

4. **Existing tests pass**: `cargo test -p perl-lexer` and `cargo test -p perl-parser` continue to pass without modification.

## Non-Goals

- This fix does NOT address the same-pattern bugs in method call variants (`$var->[...]`, `$var->{...}`, `$var->(...)`) at lines 2818–2834 — those are tracked separately.
- This fix does NOT add explicit error tokens for malformed interpolation — it only prevents malformed tokens from being produced.
- This fix does NOT change the behavior of the parser or token stream; it is strictly a lexer-level change.

## Dependencies

- `perl-lexer` crate (standalone, no parser dependencies)
- The fix must not break existing `StringPart` consumers in the token stream or parser