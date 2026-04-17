# ADR: Wire SemanticTokenModifier::Readonly and Support Readonly::Scalar/Array/Hash Variants

## Status
**Proposed**

## Context

GitHub issue #3393 reports that the `Readonly` CPAN module is not fully supported. Prior investigation confirmed:

1. **Basic Readonly extraction EXISTS** — `use Readonly` + `Readonly my $VAR => value` is already implemented and produces `SymbolKind::Constant` with `declaration: "Readonly"`.

2. **Semantic token modifier NOT wired** — `SemanticTokenModifier::Readonly` exists in `tokens.rs` but is never applied. The `node_analysis.rs` code only checks for `state` and `:shared` modifiers, ignoring Readonly declarators.

3. **Typed variants NOT supported** — `Readonly::Scalar`, `Readonly::Array`, and `Readonly::Hash` function calls are not handled; code only checks `name == "Readonly"`.

4. **Const::Fast has parallel gaps** — Same missing wiring exists for `Const::Fast` module.

## Decision

Implement fixes in two phases:

### Phase 1: Wire SemanticTokenModifier::Readonly (Low Risk)

In `crates/perl-semantic-analyzer/src/analysis/semantic/node_analysis.rs`:

1. **VariableDeclaration case** (line ~42): Add `declarator == "Readonly"` check to apply `SemanticTokenModifier::Readonly`.

2. **Variable reference case** (line ~84): Add `Some("Readonly") => SemanticTokenType::VariableReadonly` match arm and apply `SemanticTokenModifier::Readonly` modifier.

3. **VariableListDeclaration** also needs Readonly check (multi-variable `Readonly my ($x, $y)` pattern).

### Phase 2: Support Readonly::Scalar/Array/Hash (Conditional)

In both `crates/perl-symbol/src/surface/decl.rs` and `crates/perl-semantic-analyzer/src/analysis/symbol.rs`:

Extend the FunctionCall pattern match to include:
- `name == "Readonly::Scalar"`
- `name == "Readonly::Array"`
- `name == "Readonly::Hash"`

**Condition**: Phase 2 implementation is contingent on verifying that `Readonly::Scalar($x)` parses as `FunctionCall { name: "Readonly::Scalar" }` rather than `MethodCall`. The AST definition supports qualified names in `FunctionCall.name`, but this was never tested for `::` separators. A parser test must be written and passed before Phase 2 is merged.

## Alternatives Considered

### Alternative 1: Full Defer
Defer both phases indefinitely. **Rejected** because the semantic token modifier exists but is unused — completing this wiring is low-effort, high-value.

### Alternative 2: Single Phase for All
Implement semantic tokens and typed variants in one phase. **Rejected** because Phase 2 has unverified AST shape risk. Separating phases prevents blocking semantic token fixes on parser verification.

### Alternative 3: New AST Node
Create a dedicated `ReadonlyDeclaration` AST variant. **Rejected** because `constant_wrapper_decl_from_node()` already handles this pattern correctly — the issue is missing wiring, not missing architecture.

## Consequences

### Benefits
- Readonly variables get proper syntax highlighting (different from mutable variables)
- LSP clients can identify read-only variables via `SemanticTokenModifier::Readonly`
- Typed Readonly variants work identically to base `Readonly`
- Uses existing infrastructure — no new architecture needed

### Risks
- **AST Shape Risk (Phase 2 only)**: If `Readonly::Scalar` parses as `MethodCall` instead of `FunctionCall`, Phase 2 code changes will not match. Mitigated by requiring parser test.
- **Backward Compatibility**: Adding `Readonly` modifier is purely additive — existing highlighting unchanged.
- **Expression Context Risk**: `Readonly($y)` in non-declarative context could be misidentified. Mitigated by only matching when first arg is `VariableDeclaration`.

### Out of Scope (Tracked Separately)
- `Const::Fast` parallel gaps — same pattern, same missing wiring
- `Const::Fast::Scalar/Array/Hash` typed variants

## Notes

- `SymbolKind::Constant` documentation explicitly mentions "use constant or Readonly" — this fix completes that intention.
- Current active roadmap is "Quality Cleanup" and semantics hardening — this fix is aligned.
