# Specs: Const::Fast Read-Only Variable Semantic Token Tracking

## Feature Description

This feature ensures that read-only variables declared with `Const::Fast` and `Readonly` modules emit the correct LSP semantic tokens:
- `SemanticTokenType::VariableReadonly` token type
- `SemanticTokenModifier::Readonly` modifier

**Before (broken):**
```perl
use Const::Fast;
const my $PI => 3.14159;
const my @ARR => (1, 2, 3);

print $PI;  # Emits: Variable (plain)
print @ARR; # Emits: Variable (plain)
```

**After (fixed):**
```perl
use Const::Fast;
const my $PI => 3.14159;
const my @ARR => (1, 2, 3);

print $PI;  # Emits: VariableReadonly + Readonly modifier
print @ARR; # Emits: VariableReadonly + Readonly modifier
```

## Acceptance Criteria

### AC1: Const::Fast Scalar Variable Reference Emits Correct Token

**Given** a Perl file containing:
```perl
use Const::Fast;
const my $PI => 3.14159;
print $PI;
```

**When** the semantic analyzer processes the variable reference `$PI`

**Then** the emitted semantic token has:
- `token_type == SemanticTokenType::VariableReadonly`
- `modifiers` contains `SemanticTokenModifier::Readonly`

### AC2: Const::Fast Array Variable Reference Emits Correct Token

**Given** a Perl file containing:
```perl
use Const::Fast;
const my @NAMES => ('alice', 'bob');
print @NAMES;
```

**When** the semantic analyzer processes the variable reference `@NAMES`

**Then** the emitted semantic token has:
- `token_type == SemanticTokenType::VariableReadonly`
- `modifiers` contains `SemanticTokenModifier::Readonly`

### AC3: Readonly Module Variable Reference Emits Correct Token

**Given** a Perl file containing:
```perl
use Readonly;
Readonly my $MAX => 100;
print $MAX;
```

**When** the semantic analyzer processes the variable reference `$MAX`

**Then** the emitted semantic token has:
- `token_type == SemanticTokenType::VariableReadonly`
- `modifiers` contains `SemanticTokenModifier::Readonly`

### AC4: Regular my Variables Still Emit Regular Variable Tokens

**Given** a Perl file containing:
```perl
my $x = 10;
print $x;
```

**When** the semantic analyzer processes the variable reference `$x`

**Then** the emitted semantic token has:
- `token_type == SemanticTokenType::Variable`
- `modifiers` does NOT contain `SemanticTokenModifier::Readonly`

### AC5: Const::Fast Variable Declaration Site Emits Correct Token

**Given** a Perl file containing:
```perl
use Const::Fast;
const my $PI => 3.14159;
```

**When** the semantic analyzer processes the variable declaration `const my $PI`

**Then** the emitted semantic token has:
- `token_type == SemanticTokenType::VariableReadonly`
- `modifiers` contains `SemanticTokenModifier::Readonly`

### AC6: Nested Scoped Constants Work Correctly

**Given** a Perl file containing:
```perl
use Const::Fast;
const my $OUTER => 1;
{
    const my $INNER => 2;
    print $INNER;
}
print $OUTER;
```

**When** the semantic analyzer processes both variable references

**Then** each emits:
- `$INNER` emits `VariableReadonly` + `Readonly` modifier
- `$OUTER` emits `VariableReadonly` + `Readonly` modifier

## Non-Goals (Out of Scope)

1. **Changing symbol extraction** — Symbols are already correctly extracted with `kind: SymbolKind::Constant` and `declaration: Some("const")` or `Some("Readonly")`
2. **Changing `SymbolKind` taxonomy** — We are not changing the meaning of `SymbolKind::Constant`; we are fixing the lookup to handle it
3. **Assignment diagnostics** — We are not adding diagnostics for assignment to read-only variables (that would be a separate issue)
4. **Hover text changes** — Hover info already shows the declaration keyword; no changes needed
5. **Readonly::Rohan support** — Only `Readonly` (not `Readonly::Rohan`) module support is in scope

## Dependencies

### Symbol Table Lookup Fix

The root cause is that `find_symbol` requires exact `kind` match:
- Const::Fast symbols are stored with `kind: SymbolKind::Constant`
- Variable reference lookups use `kind: SymbolKind::scalar()` (from sigil `$`)
- `Constant != scalar()` → lookup fails

**Fix:** Modify `find_symbol` to accept `Option<SymbolKind>` as a filter, allowing lookups that ignore kind when needed.

### Files to Modify

1. `crates/perl-semantic-analyzer/src/analysis/symbol.rs`
   - Change `find_symbol` signature: `kind: SymbolKind` → `kind: Option<SymbolKind>`
   - Update condition: `symbol.kind == kind` → `kind.map_or(true, |k| symbol.kind == k)`

2. `crates/perl-semantic-analyzer/src/analysis/semantic/node_analysis.rs`
   - **Variable reference handling (lines 72-98):** After `find_symbol` call, check `symbol.declaration` for `"const"` or `"Readonly"` to emit `VariableReadonly` + modifier
   - **Variable declaration handling (lines 32-70):** Use `find_symbol` with `kind: None` to find constant symbols, emit `VariableReadonly` + modifier
   - **VariableListDeclaration handling (lines 418-463):** Same as VariableDeclaration

3. `crates/perl-semantic-analyzer/tests/comprehensive_unit_tests.rs`
   - Add tests for semantic token generation (not just symbol extraction)

## Technical Notes

### Why `kind: None` Finds Constants

When `kind` is `None`, the lookup condition becomes:
```rust
kind_filter.map_or(true, |k| symbol.kind == k)
// None.map_or(true, ...) = true
```

This means "find all symbols with this name in this scope", regardless of their `kind`. The symbol is found because `symbol.kind == Constant` passes the check when no filtering is applied.

### Why Declaration Site Handling Needs `kind: None`

For `const my $PI => 3.14159;`:
- The AST has `VariableDeclaration { declarator: "my", variable: Variable { name: "PI" } }`
- The declarator is `"my"`, not `"const"`
- We need to look up the symbol by name, but using `kind: scalar()` would fail (symbol has `kind: Constant`)
- Using `kind: None` finds the symbol, then we check `symbol.kind == Constant` to confirm it's a constant

### Why Reference Site Handling Also Needs `kind: None`

Same reason — the symbol has `kind: Constant` but the sigil-derived kind is `scalar()`. The existing lookup always fails, so the `symbol.declaration` check never executes.
