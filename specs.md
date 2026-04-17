# Specification: Readonly Module Pattern Support

## Feature/Behavior Description

This specification addresses two gaps in the perl-lsp LSP server's support for the `Readonly` CPAN module:

1. **Semantic Token Modifier Gap**: When a variable is declared with `Readonly my $VAR => value`, it should receive the `SemanticTokenModifier::Readonly` modifier during syntax highlighting, distinguishing it from mutable variables.

2. **Typed Variant Support Gap**: The Readonly module provides typed variants (`Readonly::Scalar`, `Readonly::Array`, `Readonly::Hash`) that should be handled identically to the base `Readonly` function.

## Prior State

- `use Readonly` import enables `readonly_enabled` flag in the analyzer
- `Readonly my $VAR => value` produces `SymbolKind::Constant` with `declaration: "Readonly"`
- `SemanticTokenModifier::Readonly` exists but is never applied to variable declarations or references

## Expected Behavior After Fix

### Phase 1: Semantic Token Modifier

When `use Readonly` is active in a file, and code like `Readonly my $CONST => 42` is declared:

1. The variable `$CONST` should have `SymbolKind::Constant` with `declaration: "Readonly"`
2. The semantic token for `$CONST` should include `SemanticTokenModifier::Readonly`
3. Any references to `$CONST` should be flagged with `SemanticTokenType::VariableReadonly` and `SemanticTokenModifier::Readonly`

### Phase 2: Typed Variants (Conditional)

When `use Readonly` is active, these patterns should be recognized:

- `Readonly::Scalar my $x => 1`
- `Readonly::Array my @arr => (1, 2, 3)`
- `Readonly::Hash my %hash => (key => "value")`

All typed variants should produce the same symbol extraction as the base `Readonly` function.

## Acceptance Criteria

### Phase 1 (Semantic Token Modifier)

1. **AC1**: Given `use Readonly` is active, when `Readonly my $X => 1` is declared, the semantic token for `$X` must include `SemanticTokenModifier::Readonly` and have type `VariableReadonly`.

2. **AC2**: Given `use Readonly` is active, when `Readonly my ($Y, $Z) => (1, 2)` is declared (multi-variable), both `$Y` and `$Z` must receive the `Readonly` modifier on their semantic tokens.

3. **AC3**: Given `use Readonly` is active, when a reference to a Readonly variable is used, its semantic token must use `SemanticTokenType::VariableReadonly`.

### Phase 2 (Typed Variants) — Conditional on Parser Verification

4. **AC4**: Given `use Readonly` is active, when `Readonly::Scalar my $x => 1` is declared, `$x` must be extracted as `SymbolKind::Constant` with `declaration: "Readonly"`.

5. **AC5**: Given `use Readonly` is active, when `Readonly::Array my @arr => (1, 2)` is declared, `@arr` must be extracted as `SymbolKind::Constant` with `declaration: "Readonly"`.

6. **AC6**: Given `use Readonly` is active, when `Readonly::Hash my %h => (a => 1)` is declared, `%h` must be extracted as `SymbolKind::Constant` with `declaration: "Readonly"`.

## Non-Goals

- This specification does NOT address `Const::Fast` module support (parallel gaps exist and should be tracked separately)
- This specification does NOT change the existing symbol extraction logic — it only wires up the semantic token modifier
- This specification does NOT add new AST nodes — it extends existing pattern matching
- This specification does NOT add `Readonly::Object` support (out of scope for this issue)

## Dependencies

1. **perl-ast**: AST node definitions (no changes needed)
2. **perl-symbol**: Shared `SymbolKind`, `VarKind` enums (no changes needed)
3. **perl-semantic-analyzer**: Uses surface module output and generates semantic tokens
4. **LSP protocol**: Must map to correct `SemanticTokenModifier::Readonly` and `SemanticTokenType::VariableReadonly`

## Implementation Notes

### Phase 1 Implementation Points

1. `crates/perl-semantic-analyzer/src/analysis/semantic/node_analysis.rs` around line 42 (VariableDeclaration case)
2. `crates/perl-semantic-analyzer/src/analysis/semantic/node_analysis.rs` around line 84 (Variable reference case)
3. `crates/perl-semantic-analyzer/src/analysis/semantic/node_analysis.rs` around line 32-70 (VariableListDeclaration case)

### Phase 2 Implementation Points (Conditional)

1. `crates/perl-symbol/src/surface/decl.rs` around line 263 (FunctionCall handling)
2. `crates/perl-semantic-analyzer/src/analysis/symbol.rs` around line 717 (FunctionCall handling)

### Verification Requirement

Phase 2 requires a parser test to verify that `Readonly::Scalar($x)` produces `FunctionCall { name: "Readonly::Scalar" }`. If the AST shape differs, the implementation must be adjusted accordingly.
