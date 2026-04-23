# ADR-0017: SQL::Abstract Method Completion, Hover, and Signature Help

## Status
**Proposed**

## Date
2026-04-23

## Context
GitHub Issue #3561 requests SQL::Abstract syntax highlighting and completion support for perl-lsp. SQL::Abstract is a foundational Perl module (used by DBIx::Class and 1000+ other distributions) that generates SQL from Perl data structures.

The issue requests:
1. Hover documentation for SQL::Abstract methods showing their parameters
2. Signature help for methods like `select($table, $fields, $where, $order)`
3. Recognition of SQL::Abstract import patterns and common aliases
4. Completion for SQL::Abstract's complex where-clause operators (`-and`, `-or`, `-nest`, `-in`, `-between`, etc.)
5. Syntax highlighting for SQL operators within Perl hash constructors used as query conditions

The perl-lsp codebase already implements similar support for DBI (Database Interface), which provides a proven template to follow.

## Decision

We will implement SQL::Abstract method completion, hover documentation, and signature help following the established DBI pattern:

1. **Guard Pattern**: Use `use SQL::Abstract` substring check (similar to DBI's `use DBI` / `use DBIx` guard) to avoid false positives for common method names like `select`, `delete`, `update`.

2. **Type Inference**: Infer SQL::Abstract type from:
   - Variables assigned from `SQL::Abstract->new(...)` (strong signal)
   - Variable names `$sql`, `$sqla`, `$sql_abs` combined with `use SQL::Abstract` in file
   - **Excluded**: `$s` is too common in Perl (loop variable, subroutine arg, generic scalar) and will NOT be used for inference

3. **Methods Supported**: Core SQL::Abstract methods:
   - `select($table, $fields?, $where?, $order?)`
   - `insert($table, $values_or_fields, $values?)`
   - `update($table, $set, $where?)`
   - `delete($table, $where?)`
   - `where($where)` - generate WHERE clause
   - `generate($stmt, @bind)` - generate arbitrary SQL
   - `values($values)` - generate VALUES clause
   - `order_by($order)` - generate ORDER BY clause

4. **Files to Modify**:
   - `crates/perl-lsp-completion/src/completion/methods.rs` — Add SQL::Abstract constants, signatures, type inference, and `get_sql_abstract_method_documentation()` function
   - `crates/perl-lsp-completion/src/lib.rs` — Export the new documentation function
   - `crates/perl-lsp/src/runtime/language/hover.rs` — Add hover and signature help integration with guard pattern

5. **No Semantic Token Changes**: SQL::Abstract SQL is expressed as hash constructors (e.g., `{ -and => [ -like => 'foo%' ] }`), not string literals. The existing `sql_string` semantic token (for DBI string arguments) cannot be reused. Syntax highlighting for SQL::Abstract operators is explicitly out of scope for Phase 1.

6. **Where-Clause Operators Deferred**: Detecting when a hash is a SQL::Abstract where clause vs. a regular hash requires semantic context that doesn't exist today. Completion for `-and`, `-or`, `-in`, `-between`, etc. is deferred to future work.

## Alternatives Considered

### 1. No Guard Pattern
Offer SQL::Abstract methods without checking for `use SQL::Abstract`. Rejected because common method names (`select`, `delete`, `update`) would cause false positives in non-SQL::Abstract code. The DBI implementation proved guard patterns are essential.

### 2. Include `$s` in Variable Inference
Allow `$s` variable name to trigger SQL::Abstract inference. Rejected because `$s` is extremely common in Perl:
- `my $s = shift;` (subroutine argument)
- `for my $s (@items) { }` (loop variable)
- `$s` as shorthand for "string" or "scalar"

This would cause false positives in any file using SQL::Abstract that also uses `$s` for other purposes.

### 3. Implement Syntax Highlighting in Phase 1
Extend semantic tokens to highlight SQL operators in hash constructors. Rejected because:
- SQL::Abstract SQL is in hash constructors, not strings
- Would require new semantic token type (e.g., `sql_abstract_operator`)
- Would require AST-level hash context detection that doesn't exist
- This is a significant feature that deserves its own issue/implementation

### 4. Implement Where-Clause Operator Completion in Phase 1
Detect SQL::Abstract where clauses and offer `-and`, `-or`, `-in`, `-between` completions. Rejected because:
- Requires detecting when a hash is a SQL::Abstract where clause vs. regular hash
- The trigger character `-` is also used for negative numbers and other hash keys
- Disambiguation requires semantic context that doesn't exist today

## Consequences

### Benefits
- **Follows proven pattern**: DBI implementation is already in production and provides an exact template
- **Guard pattern prevents false positives**: Essential for common method names
- **Low risk**: Implementation is additive only, no breaking changes
- **Immediate value**: SQL::Abstract is foundational; DBIx::Class and 1000+ distributions will benefit

### Tradeoffs
- **Syntax highlighting explicitly deferred**: Issue author requested it in title; may cause confusion
- **Where-clause operators deferred**: Explicitly requested in issue; may disappoint users expecting full feature
- **Limited to method completion**: Function completion for imported functions (e.g., `use SQL::Abstract qw(select)`) is out of scope

### Risks
1. **DBIx::Class interaction**: DBIx::Class uses SQL::Abstract internally. Users with `use DBIx::Class` get DBI hover. Files with `use SQL::Abstract` (even via DBIx::Class) would also get SQL::Abstract hover. This is acceptable — both are useful.

2. **Variable name conflicts**: `$sql` is common for any SQL-related code, not just SQL::Abstract. Mitigation: Require `SQL::Abstract->new` assignment OR combine `$sql` with `use SQL::Abstract` guard.

3. **Perl builtin conflicts**: `select` is a Perl builtin. Mitigation: Guard pattern ensures SQL::Abstract `select` only appears when `use SQL::Abstract` is present.