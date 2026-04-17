# ADR-23431b76: Validate Package-Qualified Function Calls Under strict-subs

## Status
Proposed

## Context

When `use strict 'subs'` is active in Perl, bareword subroutine calls like `FOO()` are correctly flagged as errors. However, package-qualified function calls like `Foo::bar()` are not validated — they pass silently even when neither `Foo` nor `bar` exist.

In Perl, `strict 'subs'` forbids barewords (unquoted strings used as subroutine names). A package-qualified call like `Foo::bar()` is still a bareword — the entire `Foo::bar` string is an unquoted identifier used as a subroutine reference. If `Foo` or `bar` does not exist, Perl throws a runtime error.

### Root Cause

The `NodeKind::FunctionCall` handler in `scope_analyzer.rs` (line 843) processes function calls but performs no bareword validation for qualified names. The helper function `extract_name_like_variable()` (line 1377) explicitly returns `None` for names containing `::`:

```rust
fn extract_name_like_variable<'a>(&self, name: &'a str) -> Option<(&'a str, &'a str)> {
    let (sigil, var_name) = split_variable_name(name);
    if sigil.is_empty()
        || var_name.is_empty()
        || var_name.contains("::")  // ← Qualified names return None
        || !self.looks_like_variable_name(var_name)
    {
        return None;
    }
    Some((sigil, var_name))
}
```

This means `Foo::bar()` bypasses all bareword checks because it never reaches the variable-use recording logic, and there is no separate bareword validation path for qualified function calls.

### Existing Infrastructure

The `Identifier` handler (line 974) already implements bareword validation for unqualified names:
```rust
if strict_subs_mode
    && !self.is_in_hash_key_context(node, ancestors, 1)
    && !is_known_function(name)
    && !pragma_state.has_builtin_import(name)
    && !self.is_in_hash_key_context(node, ancestors, 10)
{
    issues.push(ScopeIssue { kind: IssueKind::UnquotedBareword, ... });
}
```

This same check pattern should apply to the identifier part of qualified function calls.

## Decision

Add bareword validation for package-qualified function calls in the `NodeKind::FunctionCall` handler, following the same logic as the `Identifier` handler:

1. When `strict_subs_mode` is active AND `name.contains("::")`, extract the identifier part (final component after the last `::`)
2. Apply the same exclusion checks as unqualified barewords:
   - Not in hash key context
   - Not a known builtin function
   - Not an imported builtin
3. If all exclusions pass, report `IssueKind::UnquotedBareword` with the qualified name

### Semantic Scope

The check validates the **identifier part** (e.g., `bar` from `Foo::bar`), not the full qualified name. This is consistent with the existing unqualified bareword check, which also does not verify that a bareword like `print` actually resolves to a builtin — it only checks the name pattern.

**Implication**: `Foo::print()` will NOT be flagged (same as `print()`), because `print` is a known builtin. `Foo::nonexistent()` WILL be flagged, because `nonexistent` is not a known function.

**Why not full resolution**: Verifying that `Foo::bar` actually resolves to an existing package/subroutine would require cross-file module existence checking, which is beyond the scope of this change and would have significant complexity and performance implications.

## Alternatives Considered

### 1. Full qualified-name resolution
Check if the entire `Foo::bar` path resolves to an existing symbol in the workspace index.

**Rejected because**: Would require cross-file module existence checking, which is a larger feature with significant complexity. The existing unqualified bareword check also does not perform full resolution (it only checks name patterns), so this would be inconsistent with existing behavior.

### 2. Separate `QualifiedBareword` issue kind
Create a distinct issue kind for qualified barewords to differentiate them from unqualified barewords.

**Rejected because**: The semantic problem is the same — an unquoted string used as a subroutine name. Differentiating would add complexity without providing additional diagnostic value. The fix should be consistent with existing bareword handling.

### 3. Check only if identifier starts with uppercase
Only flag qualified calls where the identifier part starts with uppercase (e.g., `Foo::Bar`), assuming lowercase identifiers are more likely to be valid function references.

**Rejected because**: This is already the behavior of `is_known_function()` — it fast-paths on uppercase-starting names. Applying a redundant uppercase check would be redundant with the existing infrastructure.

## Consequences

### Benefits
- **Closes the validation gap**: `Foo::bar()` where `Foo` or `bar` doesn't exist will now be flagged under `strict_subs`
- **Consistent with existing behavior**: Reuses the same check pattern as unqualified barewords
- **Minimal code change**: Only adds logic to the existing `FunctionCall` handler
- **No breaking changes**: Package-qualified variables (`$Foo::bar`) continue to work as before; the fix is specifically for function calls

### Risks
- **False negatives for known builtins**: `Foo::print()` will not be flagged, even if `Foo` doesn't exist. This is consistent with existing behavior for `print()`, but means the check doesn't catch all invalid qualified calls.
- **Hash key context**: Need to verify `is_in_hash_key_context` works correctly for `FunctionCall` nodes (it traverses ancestor context, so should work regardless of node type)
- **Uppercase-first identifiers**: `Foo::Bar()` will always be flagged (same as `Bar()`) because `is_known_function` fast-paths on uppercase

### Tradeoffs
- The fix is intentionally limited to checking the identifier part, not full resolution. This is a limitation but is consistent with how unqualified barewords are handled.
- A more complete fix (full qualified-name resolution) would require significant additional work and is tracked as a potential future improvement.