# ADR/Spec Findings — work-a063e7cb

## What This ADR Decides
Fix a lexer bug in `parse_double_quoted_string` where the `$var{...}` and `$var[...]` branches don't check if `consume_balanced_segment` fails, causing malformed `StringPart` tokens to be produced for incomplete variable references.

## Key Decision
Add `if let Some(end)` guards to the two buggy branches, matching the pattern already used by the correctly-implemented `${expr}` branch. When `consume_balanced_segment` returns `None`, don't push a malformed token.

## Alternatives Considered
1. **Explicit error emission** — emit an error token when `consume_balanced_segment` returns `None`. Rejected as out of scope; more invasive.
2. **Backtrack and push literal** — backtrack position and push `{`/`[` as literal text. Rejected; requires complex position management and changes error semantics.

## Consequences
- **Positive**: Consistent error handling, no malformed tokens, improved parser confidence
- **Negative**: The orphan `{` or `[` is silently lost (consumed) when the balanced segment is not found — slight behavior change in error recovery
- **Known gap**: Same-pattern bugs exist for method calls (`$var->[...]`, etc.) at lines 2818–2834 but are out of scope

## Acceptance Criteria
1. `"$foo[ "` (missing `]`) does not push a malformed `StringPart::ArraySlice`
2. `"$foo{"` (missing `}`) does not push a malformed `StringPart::Expression`
3. Valid cases `"$foo[0]"` and `"$foo{bar}"` still work correctly
4. All existing tests in `perl-lexer` and `perl-parser` pass