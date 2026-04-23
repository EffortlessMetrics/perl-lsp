# Specification: SQL::Abstract Method Completion, Hover, and Signature Help

## Feature Summary

Add SQL::Abstract support to perl-lsp providing method completion, hover documentation, and signature help following the existing DBI pattern.

## Feature/Behavior Description

### Method Completion
When the user types `$sql->` (or similar SQL::Abstract variable) in a file that has `use SQL::Abstract`, perl-lsp will offer completions for SQL::Abstract methods: `select`, `insert`, `update`, `delete`, `where`, `generate`, `values`, `order_by`.

### Hover Documentation
When the user hovers over a SQL::Abstract method call in a file with `use SQL::Abstract`, perl-lsp will display:
- Method name
- Full signature with parameters
- Description of what the method does

### Signature Help
When the user types the arguments to a SQL::Abstract method, perl-lsp will display parameter hints showing expected argument types.

### Type Inference
perl-lsp will infer SQL::Abstract type from:
- Variable assigned from `SQL::Abstract->new(...)`
- Variable names `$sql`, `$sqla`, `$sql_abs` in a file with `use SQL::Abstract`
- Variable name `$s` is explicitly **excluded** from inference (too common in Perl)

### Guard Pattern
SQL::Abstract completions and hover are only offered when the file contains `use SQL::Abstract`. This prevents false positives for common method names in non-SQL::Abstract code.

## Acceptance Criteria

### AC1: Method Completion
**Given** a Perl file with `use SQL::Abstract` and `$sql = SQL::Abstract->new();`  
**When** the user types `$sql->`  
**Then** perl-lsp offers completions including: `select`, `insert`, `update`, `delete`, `where`, `generate`, `values`, `order_by`

### AC2: Hover Documentation
**Given** a Perl file with `use SQL::Abstract` and `$sql->select(...)` on a line  
**When** the user hovers over `select`  
**Then** perl-lsp displays hover documentation showing the method signature and description

### AC3: Signature Help
**Given** a Perl file with `use SQL::Abstract` and `$sql->select(` with cursor inside the parentheses  
**When** the user is typing arguments  
**Then** perl-lsp shows signature help with parameter hints for `$table, $fields?, $where?, $order?`

### AC4: Guard Prevents False Positives
**Given** a Perl file **without** `use SQL::Abstract` that uses a variable named `$sql`  
**When** the user types `$sql->select`  
**Then** perl-lsp does **not** offer SQL::Abstract method completions

### AC5: Variable Name Inference Without Constructor
**Given** a Perl file with `use SQL::Abstract` and `my $sqla;` (no assignment from `SQL::Abstract->new`)  
**When** the user types `$sqla->`  
**Then** perl-lsp offers SQL::Abstract method completions

### AC6: Tests Exist
**Given** the test suite for perl-lsp-completion  
**When** tests are run  
**Then** tests for SQL::Abstract method completion and hover documentation pass

## Non-Goals

### Syntax Highlighting (Out of Scope)
SQL::Abstract SQL is expressed as hash constructors (e.g., `{ -and => [ -like => 'foo%' ] }`), not string literals like DBI. Implementing syntax highlighting for these operators would require:
- A new semantic token type (e.g., `sql_abstract_operator`)
- AST-level context detection to identify when a hash is a SQL::Abstract where clause
- This is a separate feature to be addressed in a future issue

### Where-Clause Operator Completion (Out of Scope)
Completion for SQL::Abstract where-clause operators (`-and`, `-or`, `-in`, `-between`, `-like`, etc.) inside hash constructors is out of scope because:
- Requires detecting when a hash is a SQL::Abstract where clause vs. a regular hash
- The trigger character `-` is also used for negative numbers and other purposes
- Semantic context needed for disambiguation doesn't exist today

### Function Completion for Imported Functions (Out of Scope)
`use SQL::Abstract qw(select insert)` imports functions directly into the namespace. Method completion on `$sql->` is in scope; function completion for imported functions is not.

## Dependencies

- **perl-lsp-completion crate**: Must expose `get_sql_abstract_method_documentation()` function
- **perl-lsp crate**: Must integrate SQL::Abstract hover and signature help following DBI pattern
- **Existing DBI pattern**: The DBI implementation in `methods.rs` and `hover.rs` provides the template to follow

## Technical Notes

### Files to Modify
1. `crates/perl-lsp-completion/src/completion/methods.rs`:
   - Add `SQL_ABSTRACT_METHODS` constant
   - Add `SQL_ABSTRACT_METHOD_SIGS` constant with `(name, signature, description)` tuples
   - Add `get_sql_abstract_method_documentation()` function
   - Extend `infer_receiver_type()` to detect SQL::Abstract variables
   - Update `add_method_completions()` to include SQL::Abstract methods

2. `crates/perl-lsp-completion/src/lib.rs`:
   - Export `get_sql_abstract_method_documentation`

3. `crates/perl-lsp/src/runtime/language/hover.rs`:
   - Add SQL::Abstract hover with guard pattern
   - Add SQL::Abstract signature help

### SQL::Abstract Methods and Signatures
| Method | Signature | Description |
|--------|-----------|-------------|
| select | `select($table, $fields?, $where?, $order?)` | Generate SELECT statement |
| insert | `insert($table, $values_or_fields, $values?)` | Generate INSERT statement |
| update | `update($table, $set, $where?)` | Generate UPDATE statement |
| delete | `delete($table, $where?)` | Generate DELETE statement |
| where | `where($where)` | Generate WHERE clause |
| generate | `generate($stmt, @bind)` | Generate arbitrary SQL |
| values | `values($values)` | Generate VALUES clause |
| order_by | `order_by($order)` | Generate ORDER BY clause |