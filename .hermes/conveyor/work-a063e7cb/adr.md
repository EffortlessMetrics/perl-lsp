# ADR-2026-04-17: Fix Incomplete Variable Reference Parsing in Double-Quoted Strings

## Status
**Proposed**

## Context

In `crates/perl-lexer/src/lib.rs`, the `parse_double_quoted_string` function handles string interpolation for `$var{...}` (hash subscript) and `$var[...]` (array slice) expressions. When `consume_balanced_segment('{', '}')` or `consume_balanced_segment('[', ']')` fails to find a closing bracket, the lexer still pushes a `StringPart` using the current (advanced) position — producing a malformed token.

The `${expr}` branch (lines 2777–2782) already guards this correctly with `if let Some(end)`, but the `$var{...}` and `$var[...]` branches use `let _ = ...` which discards the failure indicator.

This causes the lexer to produce malformed tokens for valid Perl input like `"$foo{"` (incomplete hash subscript), leading to downstream parsing errors.

## Decision

Add `if let Some(end)` guards to the `$var[...]` (array slice) and `$var{...}` (hash subscript) branches in `parse_double_quoted_string`, matching the pattern already used by the `${expr}` branch.

When `consume_balanced_segment` returns `None`, the `{` or `[` is consumed (lost) and the string continues lexing from the next character. This is a lexer-level recovery; the malformed string input may still produce an error token or be treated as literal text.

## Specific Changes

1. **`$var[...]` branch (lines 2868–2873)**: Wrap `consume_balanced_segment` result in `if let Some(end)` before pushing `StringPart::ArraySlice`.

2. **`$var{...}` branch (lines 2874–2879)**: Wrap `consume_balanced_segment` result in `if let Some(end)` before pushing `StringPart::Expression`.

## Consequences

### Benefits
- Consistent error handling across all three interpolation branches
- Lexer no longer produces malformed `StringPart` tokens for incomplete variable references
- Parser confidence improves (fewer cascading errors from malformed tokens)
- Aligns with the codebase's active Quality Cleanup phase

### Tradeoffs / Risks
- **Behavioral change**: Currently, `"$foo{"` produces a partial `StringPart::Expression` with the orphan `{`. After the fix, the `{` is silently lost (consumed). This is arguably more correct since Perl would not successfully interpolate `$foo{` without a closing `}`, but it changes the error recovery path.
- **Same-pattern bugs exist for method calls** (`$var->[...]`, `$var->{...}`, `$var->(...)`) at lines 2818–2834. These are out of scope for this issue but should be tracked separately.

## Alternatives Considered

### Alternative 1: Explicit error emission
When `consume_balanced_segment` returns `None`, emit an explicit error token for the malformed interpolation. This would provide better diagnostics but requires more changes to the lexer's token emission logic and is out of scope for this fix.

### Alternative 2: Backtrack to `tail_start` and push literal
After `consume_balanced_segment` returns `None`, backtrack the position to `tail_start` and push the `{` or `[` as a literal character. This preserves the opening delimiter but requires position management that could introduce new bugs. Additionally, Perl itself would treat `$foo{` as invalid syntax, so treating it as literal text is already a best-effort recovery.

## Notes
- Tests should be placed in `crates/perl-lexer/tests/` (confirmed by plan-reviewer) alongside existing `interpolated_string_preserves_complex_tails` test
- Method-call variants (lines 2818–2834) have the same bug pattern but are out of scope
- The fix is lexer-only; no changes to `perl-parser` or `perl-tokenizer` are needed