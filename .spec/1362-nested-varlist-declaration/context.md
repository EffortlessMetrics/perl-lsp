# Context: #1362 — Nested variable list declarations

## Problem

The Perl parser fails when parsing nested variable list declarations that contain multiple items inside the nested parentheses. For example, `my ($a, ($b, $c)) = (1, 2, 3)` throws "expected ')', found ','" at the position after `$b`, causing a cascade of downstream parse errors.

This blocks the parsing of destructuring assignments and nested lexical declarations that are valid Perl syntax. Users cannot parse code using this pattern, causing LSP to lose semantic capabilities (completion, hover, rename) in those blocks.

The bug is real and verified: nested lists with a single item (e.g., `my ($a, ($b))`) work fine; the parser only fails when there are multiple items in the nested parentheses.

## Why this approach

The root cause is in `parse_variable_list_item()` (line 163-167 in `variables.rs`). When it encounters a `LeftParen`, it:
1. Consumes the `(`
2. Recursively calls `parse_variable_list_item()` exactly once (parsing a single item)
3. Immediately expects `)` — no loop to handle comma-separated items

The fix is to replace the single-item recursion with a loop similar to the one already in `parse_variable_declaration()` (lines 15-50). This loop:
1. Parses items in a while loop until `RightParen` or EOF
2. Consumes commas between items
3. Expects `)` only after the loop

If the nested list has exactly 1 item, return that item directly (backward compatible). If it has multiple items, wrap them in a new `NodeKind::NestedVariableList { items: Vec<Node> }` node to preserve the structure in the AST.

This approach:
- Mirrors the existing pattern in `parse_variable_declaration()` (proven, tested structure)
- Requires minimal code changes (mostly copy-paste + wrapping)
- Preserves backward compatibility (single items return unwrapped)
- Maintains location tracking for proper source spans
- Prepares the AST for semantic analysis (all nested items are captured and named)

## Alternatives rejected

1. **Modify the caller (`parse_variable_declaration()`) to handle nested lists differently**: Rejected because `parse_variable_list_item()` is also called from `expressions/calls.rs:557`, and changes would need to be duplicated. Centralizing the fix in the function itself is cleaner.

2. **Return a flat list from nested parens (no wrapper node)**: Rejected because it loses the nesting structure. Downstream semantic analysis (variable scoping, destructuring patterns) needs to know which items are nested so it can apply the correct semantics. Preserving structure in the AST is essential.

3. **Use a recursive descent approach with deeper recursion**: Rejected because the existing `parse_variable_declaration()` already uses a while loop, and a while loop is more predictable and easier to test for delimiter edge cases (unbalanced parens, etc.).

4. **Add error recovery for missing comma**: Rejected as out-of-scope for this fix. The parser's existing error recovery is reasonable — if a comma is missing, the parser correctly reports "expected comma or )" and continues. This fix doesn't change error handling, only enables the happy path.

## Prior art / duplicates

No existing implementation of nested variable lists in this codebase. The pattern `my ($a, ($b, $c))` has never been parsed successfully. Perl's own parser handles this as a core language feature (destructuring / list flattening). This is not a duplicate of any existing feature.

Related work:
- `parse_variable_declaration()` in the same file already has the loop structure we're adopting for `parse_variable_list_item()`.
- Subroutine signature parsing (`parse_signature()`, lines 846+) uses a similar while loop for parameters.

## Links

- Issue: #1362
- Specification branch: `impl/1362-nested-varlist-declaration`
- PARSER_CONTRACTS.md: Check "variable list" or "destructuring" sections (if present) — may need review post-implementation to see if contracts need documenting
- Related concepts: 
  - Destructuring patterns (Perl lists are multi-level structures)
  - Variable scope and nesting (semantic analyzer will need to handle nested declarations)
- Related issues: None identified; this is a standalone parser bug fix
