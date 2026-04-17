# Specification: map/grep → foreach Loop Conversion

## Feature/Behavior Description

Implement `RefactorRewrite` code actions that convert `map` and `grep` functional-style expressions to equivalent `foreach` loops in Perl.

### Transformations

#### map → foreach
- **Input**: `my @results = map { expr } @items;`
- **Output**:
  ```perl
  my @results;
  for my $item (@items) {
      push @results, expr;
  }
  ```
- The loop variable is `$item` (consistent with existing `loop_conversion.rs`)

#### grep → foreach
- **Input**: `my @filtered = grep { predicate } @items;`
- **Output**:
  ```perl
  my @filtered;
  for my $item (@items) {
      push @filtered, $item if predicate;
  }
  ```
- The predicate becomes a postfix `if` condition

### AST Detection

- map and grep are `NodeKind::FunctionCall { name: "map"|"grep", args: Vec<Node> }`
- First argument is a `NodeKind::Block { statements: Vec<Node> }` (parsed via `parse_builtin_block()`)
- Second argument is the list expression

### Skip Conditions

The code action is **not** offered when:
1. The block has more than one statement (multi-statement blocks cannot cleanly convert)
2. The first argument is a regex (e.g., `grep /pattern/ @list`) instead of a block
3. The map/grep has fewer or more than 2 arguments (only 2-arg variant handled)

### Code Action Properties

- **Kind**: `CodeActionKind::RefactorRewrite`
- **Title**: "Convert to foreach loop"
- **Edit**: Replaces the entire `FunctionCall` node range with the generated foreach loop

## Acceptance Criteria

### AC1: map → foreach conversion
When the cursor is on a `map { expr } @list` expression with a single-expression block:
- A code action "Convert to foreach loop" is offered
- Accepting it replaces the expression with a semantically equivalent foreach loop

### AC2: grep → foreach conversion
When the cursor is on a `grep { predicate } @list` expression with a single-expression block:
- A code action "Convert to foreach loop" is offered
- Accepting it replaces the expression with a foreach loop containing a postfix `if` condition

### AC3: Multi-statement blocks skipped
When the block has multiple statements:
- No code action is offered
- The conversion is silently skipped (no error shown to user)

### AC4: Tests in correct location
Tests are added to `behavior_spec_tests.rs` (not `modernize_tests.rs`), using the existing `enhanced_actions_for()` helper pattern.

### AC5: Production code constraints
- No `unwrap()`, `expect()`, `panic!()`, `todo!()`, or `unimplemented!()` in production code
- Follows existing patterns in `loop_conversion.rs`

## Non-Goals

1. **Reverse conversion (foreach → map/grep)** is out of scope for this work item
2. **Variable name conflict detection** is not implemented (uses `$item` consistently)
3. **Side-effect analysis** is not implemented (not needed for Phase 1: map/grep → foreach is safe)
4. **C-style `for` loop handling** is handled by existing `loop_conversion.rs`, not this module

## Dependencies

- `perl_parser_core::ast::{Node, NodeKind}` — AST types
- `perl_lsp_rename::TextEdit` — edit payload
- `crate::types::{CodeAction, CodeActionEdit, CodeActionKind::RefactorRewrite}` — result types
- `perl_tdd_support::must` / `must_some` — test helpers per AGENTS.md

## Files to Create/Modify

| File | Change |
|------|--------|
| `crates/perl-lsp-code-actions/src/enhanced/map_grep_conversion.rs` | **New** — conversion functions |
| `crates/perl-lsp-code-actions/src/enhanced/mod.rs` | Add module declaration + wire into `collect_actions_for_range()` |
| `crates/perl-lsp-code-actions/tests/behavior_spec_tests.rs` | Add tests for map/grep conversions |

## Test Cases

1. **Simple map conversion**: `map { $_ * 2 } @items` → foreach loop
2. **Simple grep conversion**: `grep { $_ > 5 } @nums` → foreach loop with postfix if
3. **map with method call**: `map { $_->name } @users` → uses `$user` variable
4. **Multi-statement block**: No action offered (skipped)
5. **grep with regex first arg**: No action offered (skipped)
6. **Void context map**: `map { print $_ } @items` → valid conversion (no assignment)