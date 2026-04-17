# ADR: Const::Fast Read-Only Variable Semantic Token Tracking

## Title
ADR-001: Fix Symbol Lookup for Const::Fast Read-Only Variable Semantic Token Generation

## Status
**Proposed**

## Context

The issue (GitHub #3392) reports that `Const::Fast` read-only variables are not properly tracked by the perl-lsp analyzer. Specifically:
- `const my $PI => 3.14159;` should emit `SymbolKind::Constant` and `VariableReadonly` semantic tokens
- `const my @ARRAY => (1, 2, 3);` should emit `SymbolKind::Constant` and `VariableReadonly` semantic tokens

The infrastructure for this feature exists:
- `SemanticTokenType::VariableReadonly` is defined in `tokens.rs` line 14 but **unused**
- `SemanticTokenModifier::Readonly` is defined in `tokens.rs` line 80 but **unused**
- Symbol extraction correctly stores Const::Fast symbols with `kind: SymbolKind::Constant` and `declaration: Some("const")`

**However**, the proposed implementation plan has a critical flaw: `find_symbol` requires exact `kind` match (`symbol.kind == kind`), but Const::Fast symbols are stored with `kind: SymbolKind::Constant` while variable lookups use `kind: SymbolKind::scalar()`, `SymbolKind::array()`, or `SymbolKind::hash()` derived from the sigil.

For example, when analyzing `$PI`:
1. Symbol lookup derives `kind = SymbolKind::scalar()` from sigil `$`
2. `find_symbol("PI", scope_id, SymbolKind::scalar())` is called
3. The symbol was stored with `kind: SymbolKind::Constant`
4. `Constant != scalar()` → lookup returns empty
5. The `symbol.declaration` check never executes
6. Plain `Variable` token is emitted instead of `VariableReadonly`

**This affects both declaration sites AND reference sites.**

## Decision

### Primary Decision: Fix Symbol Lookup to Support Kind-Ignored Lookups

We will modify `find_symbol` to accept an optional `kind` filter, allowing lookups that find symbols by name and scope regardless of their `SymbolKind` variant.

**Change to `crates/perl-semantic-analyzer/src/analysis/symbol.rs`:**

```rust
/// Find symbol definitions visible from a given scope for Navigate/Analyze workflows.
///
/// When `kind_filter` is `Some(kind)`, only symbols matching that exact kind are returned.
/// When `kind_filter` is `None`, all symbols matching name and scope are returned (ignoring kind).
///
/// This is essential for Const::Fast/Readonly variables which are stored with
/// `SymbolKind::Constant` but need to be found during variable reference analysis
/// where the lookup kind is derived from the sigil (scalar/array/hash).
pub fn find_symbol(
    &self,
    name: &str,
    from_scope: ScopeId,
    kind_filter: Option<SymbolKind>,
) -> Vec<&Symbol> {
    // ... existing scope-walking logic ...
    // Change: `symbol.kind == kind` becomes `kind_filter.map_or(true, |k| symbol.kind == k)`
}
```

**Rationale:**
- `SymbolKind::Constant` semantically represents "a constant value" which can have a sigil-based container (scalar/array/hash)
- The `declaration` field already distinguishes between `use constant`, `const`, and `Readonly` wrappers
- Making `kind` optional in `find_symbol` allows both regular variable lookups and constant lookups to work correctly
- This is a minimal, backward-compatible change (existing callers passing `Some(kind)` behave identically)

### Secondary Decision: Use `symbol.declaration` for Readonly Detection

After successful symbol lookup, the semantic token generation in `node_analysis.rs` will check:
- `symbol.declaration == Some("const")` → emit `VariableReadonly` with `Readonly` modifier
- `symbol.declaration == Some("Readonly")` → emit `VariableReadonly` with `Readonly` modifier
- Otherwise → use existing logic

**This avoids needing to change `SymbolKind::Constant` semantics**, which would affect other parts of the codebase (e.g., `use constant` declarations).

## Alternatives Considered

### Alternative 1: Add `find_symbol_by_name_only(name, scope_id)` Method

Add a completely separate method that ignores kind entirely.

**Pros:**
- Clean separation of concerns
- No change to existing `find_symbol` API

**Cons:**
- Duplicates scope-walking logic
- Two methods to maintain
- Callers must choose which method to use (API confusion)

**Rejected because:** The optional `kind_filter` approach achieves the same goal with a single method and no logic duplication.

### Alternative 2: Change Const::Fast Storage to Use `SymbolKind::Variable`

Store Const::Fast symbols with `kind: SymbolKind::Variable(VarKind::Scalar)` instead of `SymbolKind::Constant`, and add a separate `read_only: bool` field.

**Pros:**
- Aligns with the idea that a Const::Fast variable is "still a variable, but read-only"

**Cons:**
- Large architectural change to symbol extraction (`symbol.rs` lines 2878-2916)
- Changes meaning of `SymbolKind::Constant` (currently includes Const::Fast)
- Risk of breaking other parts of the codebase that depend on `SymbolKind::Constant` including Const::Fast
- `use constant NAME => value` (compile-time constants) and `const my $x =>` (Const::Fast) would become semantically different kinds

**Rejected because:** The issue is specifically about semantic token generation, not symbol taxonomy. Using `declaration` to distinguish wrappers is sufficient and less invasive.

### Alternative 3: Track `const_fast_enabled` in `SemanticAnalyzer`

Add state tracking to `SemanticAnalyzer` similar to `SymbolExtractor`, enabling Const::Fast/Readonly context awareness during analysis.

**Pros:**
- Doesn't require symbol table changes

**Cons:**
- Significant refactoring of `SemanticAnalyzer`
- Duplicates state that already exists in symbol table
- Complex edge cases (nested scopes, closures, dynamic contexts)

**Rejected because:** The symbol table already has all necessary information. Looking up the symbol is simpler and less error-prone than tracking enablement state.

### Alternative 4: Change Variable Reference Lookup to Try Multiple Kinds

When a variable reference lookup fails, retry with `SymbolKind::Constant`.

**Pros:**
- Minimal code change
- Works for the specific case

**Cons:**
- Inefficient (two lookups instead of one)
- Doesn't scale if more "exception kinds" are added
- Only fixes reference sites, not declaration sites
- Creates an implicit assumption that failed lookup means "might be a constant"

**Rejected because:** The optional `kind_filter` approach is cleaner and more general.

## Consequences

### Positive Consequences

1. **Const::Fast variables will emit `VariableReadonly` tokens** — The original issue will be fixed
2. **Readonly module variables will emit `VariableReadonly` tokens** — Same fix covers both modules
3. **Backward compatible** — Existing callers passing `Some(kind)` behave identically
4. **Minimal code change** — Only `find_symbol` signature and the single condition inside it change
5. **Future-proof** — The optional `kind_filter` can be used for other purposes if needed

### Negative Consequences

1. **API change** — All existing callers of `find_symbol` must be updated to pass `Some(kind)` instead of `kind` directly
   - **Mitigation:** The compiler will catch all call sites; there are few callers (verified: only `node_analysis.rs` uses it for variable lookups)

2. **Potential confusion** — Passing `None` for `kind_filter` means "find any kind"
   - **Mitigation:** The semantic is clear: `None` means "don't filter by kind"

### Neutral Consequences

1. **`VariableListDeclaration` handling** — Same fix applies; no additional changes needed beyond what `node_analysis.rs` requires

## Dependencies

- **Symbol table API change:** `crates/perl-semantic-analyzer/src/analysis/symbol.rs`
  - Change `find_symbol` signature to accept `Option<SymbolKind>`
  - Update condition from `symbol.kind == kind` to `kind_filter.map_or(true, |k| symbol.kind == k)`

- **Semantic token generation:** `crates/perl-semantic-analyzer/src/analysis/semantic/node_analysis.rs`
  - Update `find_symbol` calls to pass `Some(kind)` (existing behavior)
  - Add new `find_symbol` call for declaration sites with `None` (to find constant regardless of sigil-derived kind)
  - Add check for `symbol.declaration == Some("const") | Some("Readonly")` to emit `VariableReadonly`

- **Tests:** `crates/perl-semantic-analyzer/tests/comprehensive_unit_tests.rs`
  - Add test: `semantic_token_const_fast_scalar_emits_variable_readonly`
  - Add test: `semantic_token_const_fast_array_emits_variable_readonly`
  - Add test: `semantic_token_readonly_scalar_emits_variable_readonly`
  - Add test: `semantic_token_const_fast_reference_emits_variable_readonly`

## Files NOT Requiring Changes

- `crates/perl-symbol-surface/src/decl.rs` — Symbol extraction already correct
- `crates/perl-semantic-analyzer/src/analysis/symbol.rs` — Symbol extraction already correct (only `find_symbol` lookup needs fix)
- `crates/perl-semantic-analyzer/src/analysis/semantic/tokens.rs` — Token types already defined correctly

## Questions for Future Consideration (Out of Scope)

1. Should hover info indicate "read-only" explicitly for constants?
2. Should rename operations warn when renaming constants?
3. Should diagnostics flag assignment to read-only variables?
