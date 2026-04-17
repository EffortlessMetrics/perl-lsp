# Spec: strict-subs Package-Qualified Function Call Validation

## Feature/Behavior Description

Validate package-qualified function calls (e.g., `Foo::bar()`) under `strict 'subs'` when the identifier part (the final component after `::`) is not a known builtin and not in hash key context.

### Behavior

When `use strict 'subs'` is active:

| Code | Validated? | Reason |
|------|-----------|--------|
| `FOO()` | ✅ Flagged | Unqualified bareword, not a known function |
| `Foo::bar()` where `bar` is not a builtin | ✅ Flagged | Qualified bareword with unknown identifier |
| `Foo::bar()` where `bar` is a builtin (e.g., `print`) | ❌ Not flagged | Consistent with `print()` behavior |
| `$h{Foo::bar}` (hash key context) | ❌ Not flagged | Hash key context is always excluded |
| `$obj->method()` (method call) | ❌ Not flagged | Method calls are a different node type |
| `Foo::bar()` where `bar` is uppercase (e.g., `DBI`) | ✅ Flagged | `is_known_function` fast-paths on uppercase |

### Implementation Location

**File**: `crates/perl-semantic-analyzer/src/analysis/scope_analyzer.rs`

**Location**: `NodeKind::FunctionCall` handler (line 843)

**Logic**: After the existing `extract_name_like_variable` block, add a branch for qualified names (containing `::`) when `strict_subs_mode` is active. Extract the identifier part using `rsplit("::").next()` and apply the same checks as the `Identifier` handler.

## Acceptance Criteria

### AC1: Foo::bar() is flagged under strict_subs when bar is not a known builtin
```
use strict 'subs';
Foo::bar();
```
- Must produce an `IssueKind::UnquotedBareword` with `variable_name = "Foo::bar"`
- The issue must be reported on the line where `Foo::bar()` appears

### AC2: Foo::print() is NOT flagged under strict_subs (consistent with print())
```
use strict 'subs';
Foo::print();
```
- Must NOT produce any `IssueKind::UnquotedBareword` for `Foo::print`
- This is consistent with `print()` not being flagged (print is a known builtin)

### AC3: Existing unqualified bareword behavior is unchanged
```
use strict 'subs';
FOO();
Bar::print();
```
- `FOO()` must still be flagged (existing behavior)
- `Bar::print()` must NOT be flagged (print is a known builtin, consistent with AC2)

### AC4: Hash key context is excluded
```
use strict 'subs';
my %h = (Foo::bar => 1);
```
- Must NOT produce an `IssueKind::UnquotedBareword` for `Foo::bar` in hash key context

### AC5: Method calls are not affected
```
use strict 'subs';
$obj->method();
```
- Must NOT produce an `IssueKind::UnquotedBareword` (method calls are a different node type)

### AC6: Package-qualified variables ($Foo::bar) are not affected
```
use strict 'subs';
print $Foo::bar;
```
- Must NOT produce any issue (package-qualified variables are handled separately)

## Non-Goals

1. **Full qualified-name resolution**: This fix does NOT verify that `Foo::bar` actually resolves to an existing package/subroutine. It only checks the identifier part (`bar`) against the known-builtin list, consistent with how unqualified barewords are handled.

2. **Cross-file module existence checking**: The fix operates purely on local name pattern matching, not on symbol resolution.

3. **Method call validation**: `->method()` calls are not in scope (different node type).

4. **strict 'vars' behavior**: Package-qualified variables (`$Foo::bar`) are not validated under `strict 'vars'` and this fix does not change that.

## Dependencies

- `perl-semantic-analyzer`: The fix is contained entirely within this crate
- `perl-parser-core`: AST node definitions (`FunctionCall`, `Identifier`, `MethodCall`)
- Existing infrastructure:
  - `is_known_function()` — checks if name is a known builtin
  - `has_builtin_import()` — checks if name is imported via `use subs`
  - `is_in_hash_key_context()` — checks ancestor chain for hash subscript context
  - `pragma_state.strict_subs` — the strict subs mode flag

## Test Plan

### New Tests (in `scope_and_symbol_tests.rs`)

1. **Positive test**: `Foo::bar()` is flagged as bareword under `strict_subs`
2. **Negative test**: `Foo::print()` is NOT flagged (print is builtin)
3. **Negative test**: `Foo::bar()` in hash key context is NOT flagged
4. **Consistency test**: Verify `FOO()` is still flagged, `print()` is still not flagged

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| False negatives: `Foo::print()` where `Foo` doesn't exist | Acceptable — consistent with existing `print()` behavior |
| Uppercase identifiers always flagged: `Foo::Bar()` | Acceptable — consistent with existing `Bar()` behavior |
| Hash key context check may not apply to FunctionCall nodes | Verified — `is_in_hash_key_context` traverses ancestor chain, not node type |