# ADR-0017: map/grep ↔ foreach Loop Conversion Code Actions

## Status
Proposed

## Context

GitHub issue #3511 requests implementing bidirectional code actions to convert between functional style (`map`/`grep`) and imperative `foreach` loops in Perl. This is a `refactor.rewrite` LSP action.

The issue describes four transformations:
1. **map → foreach**: Convert `my @results = map { $_ * 2 } @items;` to explicit loop
2. **foreach → map**: Convert loop that builds array via `push` back to `map`
3. **grep → foreach**: Convert `my @filtered = grep { $_->{active} } @users;` to explicit loop
4. **foreach → grep**: Convert loop with conditional `push` back to `grep`

### AST Representation

In the perl-lsp codebase:
- `map` and `grep` are **not** their own `NodeKind` variants — they are parsed as `NodeKind::FunctionCall { name: "map"|"grep", args: Vec<Node> }`
- The first argument (the block) is parsed via `parse_builtin_block()` and becomes a `NodeKind::Block { statements: Vec<Node> }`
- The second argument is the list expression

**Critical correction from verification**: `NodeKind::Push` and `NodeKind::Pop` do **not** exist. `push` is represented as `FunctionCall { name: "push", args: [...] }`.

### Existing Patterns

The existing `loop_conversion.rs` handles:
- C-style `for (init; cond; update)` → `foreach my $item (@array)`
- `foreach my $_ (@list)` using implicit `$_` → explicit variable

Integration point is `collect_actions_for_range()` in `mod.rs` at line ~194-196.

## Decision

Implement **Phase 1 only**: map/grep → foreach conversion as a **new module** `crates/perl-lsp-code-actions/src/enhanced/map_grep_conversion.rs`.

### Why a New Module?

1. `loop_conversion.rs` handles fundamentally different constructs (C-style loops, `$_` aliasing)
2. Separating concerns makes the codebase easier to maintain
3. Follows established pattern in the codebase

### Why Phase 1 Only?

1. **map/grep → foreach is low-risk**: The map/grep expression is already well-formed; conversion is straightforward text transformation
2. **foreach → map/grep is high-risk**: Requires complex pattern detection:
   - Detecting a single `push` statement (not `NodeKind::Push`, but `FunctionCall { name: "push", ... }`)
   - Verifying no `next`/`last`/`redo` control flow
   - Side-effect analysis
3. **Incremental delivery**: Ship value quickly, enhance later

### Conversion Examples

**map → foreach:**
```perl
# Input
my @results = map { $_ * 2 } @items;

# Output
my @results;
for my $item (@items) {
    push @results, $item * 2;
}
```

**grep → foreach:**
```perl
# Input
my @filtered = grep { $_->is_active } @users;

# Output
my @filtered;
for my $user (@users) {
    push @filtered, $user if $user->is_active;
}
```

### Skip Conditions (Phase 1)

Offer conversion **only** when:
1. First argument is a `NodeKind::Block` with exactly **one** statement (single-expression body)
2. Second argument exists (the list expression)

Skip (do not offer action) when:
- Block has multiple statements
- Block uses regex instead of code block (e.g., `grep /pattern/ @list`)

## Consequences

### Benefits
- Clean separation of concerns between C-style and functional-style loop conversions
- Low-risk, high-value: straightforward text transformation
- Follows established codebase patterns
- Incremental delivery enables faster shipping

### Tradeoffs
- Variable naming collision possible (mitigated: existing `loop_conversion.rs` uses same `$item` pattern)
- Multi-statement blocks cannot be converted (acceptable: skip and document)
- Tests must go in `behavior_spec_tests.rs` (not `modernize_tests.rs`) because these are `RefactorRewrite` kind

### Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Variable name collision with `$item` | Medium | Medium | Existing codebase pattern; use consistent naming |
| Multi-statement blocks offered | High | Medium | Check `statements.len() == 1` before offering |
| `$_` aliasing differences | Medium | Low | Phase 1 converts FROM functional style; minimal risk |

## Alternatives Considered

### Alternative 1: Extend existing `loop_conversion.rs`
Rejected — mixes different concerns (C-style vs. functional-style loops).

### Alternative 2: Implement bidirectional conversion in Phase 1
Rejected — reverse conversion requires complex pattern detection and safety analysis.

### Alternative 3: Inline in `mod.rs`
Rejected — dedicated module provides better testability and follows established patterns.

## Phase 2 (Future Work)

Deferred to a future work item. Will require:
- Detecting `for my $item (@list) { push @results, expr($item); }` patterns
- Pattern-matching on `FunctionCall { name: "push", ... }` (not `NodeKind::Push`)
- Skip conditions: multiple statements, control flow (`next`/`last`/`redo`), side effects