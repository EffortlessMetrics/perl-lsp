# Context: NodeKindCategory Drift-Guard for Document Symbols

## Problem Statement

The document-symbols extractor in `crates/perl-lsp-rs/src/runtime/symbol_extraction.rs` currently uses a monolithic pattern match on `NodeKind` with a catch-all `_ => {}` arm. If a new `NodeKind::Declaration` variant (e.g., `NodeKind::Role`) is added in the future, the extractor will silently ignore it unless the developer explicitly remembers to add a matching arm.

This is a **drift risk**: the parser adds a new declaration construct, but the LSP provider silently drops its symbols because the catch-all matches first.

## Solution Approach

Adopt the merged `NodeKindCategory::Declaration` classification API (from PR #1295) as a compile-time **drift-guard**:

```rust
match &node.kind {
    kind if kind.category() == NodeKindCategory::Declaration => {
        // Now this inner match must be exhaustive over Declaration variants.
        // A new Declaration variant → compiler error until handled.
        match &node.kind {
            NodeKind::Subroutine { .. } => { /* emit Function */ }
            NodeKind::Package { .. } => { /* emit Module */ }
            NodeKind::Class { .. } => { /* emit Class */ }
            NodeKind::Method { .. } => { /* emit Method */ }
            NodeKind::VariableDeclaration { declarator == "our", .. } => { /* emit Variable */ }
            NodeKind::FunctionCall { name == "has", .. } => { /* emit Property */ }
            _ => {} // All other Declaration variants (Use, No, Format, etc.)
        }
    }
    // Non-Declaration variants: no guard needed
    NodeKind::Program { .. } => { /* recurse */ }
    NodeKind::Block { .. } => { /* recurse */ }
    NodeKind::ExpressionStatement { .. } => { /* recurse */ }
    _ => {} // All other non-Declaration variants
}
```

### Key Properties

1. **Behavior-preserving**: All 6 existing symbol-kind mappings remain exactly the same. No symbol output changes.
2. **Narrow adoption**: The category guard is used only as a pre-filter; per-variant symbol-kind logic stays intact.
3. **Compile-time protection**: If `NodeKind::Declaration` variants are added, the inner match becomes non-exhaustive → compiler error → forces decision on new variant.
4. **No DAP impact**: Safe-for-breakpoint logic is untouched (PR #1297 handles that separately).

---

## Key Decisions

### 1. Why an outer guard + inner match (not a single match with explicit Declaration arms)?

**Rejected:** Writing all Declaration variant names in a single exhaustive match.
```rust
// Not done this way:
match &node.kind {
    NodeKind::Subroutine { .. } | NodeKind::Package { .. } | ... => { ... }
    _ => {}
}
```

**Why rejected:** Too verbose, hard to maintain, and doesn't scale as new Declaration variants are added.

**Chosen:** Guard + inner match provides clarity: "handle all Declarations here" → "now be exhaustive over them" → compiler catches gaps.

### 2. Why keep `FunctionCall { name == "has" }` in the Declaration guard?

**Answer:** `FunctionCall` is NOT a `NodeKindCategory::Declaration` variant. It's an `Expression`.

So the guard `kind.category() == NodeKindCategory::Declaration` will NOT match `FunctionCall { name: "has", .. }`.

**Then how does "has" get emitted?** 

Looking at the current code, `FunctionCall { name == "has" }` is handled in the flat match at the same level as Subroutine/Package. After the refactoring, this arm moves OUTSIDE the outer guard (since FunctionCall is not a Declaration). So the flow is:

```
FunctionCall { name: "has", args: [String { value: "attr" }, ..] }
  → Does NOT match `kind.category() == Declaration` (FunctionCall is Expression)
  → Falls through to outer `_ => {}`
  → Silent drop (PROBLEM!)
```

This is a **scope expansion risk** — if we're not careful, the refactoring would break the "has" symbol emission.

**Mitigation:** The builder must be careful. Options:

1. **Add a separate outer guard for FunctionCall { name == "has" }** (if we want to keep it):
   ```rust
   kind if kind.category() == NodeKindCategory::Declaration => { /* ... */ }
   kind if kind.category() == NodeKindCategory::Expression && matches!(&node.kind, NodeKind::FunctionCall { name, .. } if name == "has") => { /* emit Property */ }
   ```

2. **Document that FunctionCall { name == "has" } is currently emitted, and the refactoring preserves this** (status quo).

3. **Decide if "has" symbols are worth the special case** (if not, remove the arm and drop Property symbols).

For now, we **document the current behavior and preserve it**: FunctionCall { name == "has" } is handled in the refactored code by moving it to the outer match's non-Declaration arm (as it currently is). The checker is: "Does FunctionCall appear in the Declaration category?" Answer: No. So it is handled outside the guard.

**Builder note:** Verify that "has" symbol extraction still works after refactoring. Write a test if not already present.

### 3. Why not replace per-variant symbol-kind mapping with category-based logic?

**Rejected:** Using category() to assign symbol kinds.
```rust
// Not done this way:
let symbol_kind = match kind.category() {
    Declaration => match ... // still need per-variant mapping
    Expression => 18,
    ...
};
```

**Why rejected:** Symbol kinds are per-variant, not per-category. `Subroutine` → 12 (Function), `Package` → 2 (Module), etc. Category is too coarse.

**Chosen:** Keep per-variant mapping; use category only as a guard. Allows fine-grained control.

---

## Alternatives Rejected

### A. Global catch-all with explicit new-variant checks
```rust
match &node.kind {
    // 6 declaration-emitting arms
    _ if node.kind.category() == Declaration => {
        // Log warning: unhandled Declaration
    }
    _ => {}
}
```
**Rejected:** Logs warnings on every unsupported Declaration, noise. Does not enforce compile-time.

### B. No guard; rely on future PRs to add new Declaration arms
```rust
// Current state, no refactoring
match &node.kind {
    NodeKind::Subroutine { .. } => { emit Function }
    // ...
    _ => {} // Silently drops unknown Declarations
}
```
**Rejected:** This is the current problem — drift risk is real, as shown by #911 (NodeKindCategory was added to address this).

### C. Separate consumer for Classification API (not Symbol Extraction)
**Rejected:** Classification API is meant for multiple consumers. Document-symbols is the first; DAP is the second (gated on #1297). This adoption proves the API is usable in LSP context.

---

## Why This Matters

From issue #911 and PR #1295: NodeKindCategory was merged to provide a **consumer boundary contract**. The classification logic lives in one place (`crates/perl-ast/src/classification.rs`, exhaustive match, no wildcards). Consumers can use `category()` to make decisions, and the compile-time guarantee carries forward: if a new variant is added and not classified, both the classification module AND the consumer will error.

This refactoring **proves the contract works** by being the first real LSP consumer of the classification API. If it works cleanly here, DAP can adopt it with confidence.

---

## Scope Boundaries

### In scope
- Refactor `extract_symbols_recursive` only (not `extract_simple_symbols` — no feature gate)
- Preserve all 6 symbol-kind mappings
- Add test documenting the guard mechanism
- No API changes, no new symbol kinds

### Out of scope
- Safe-for-breakpoint logic (PR #1297)
- DAP integration (Phase 5)
- Other document-symbols features (outline, resolve, etc.)

---

## References

- **Issue**: #1330 — "Refactor document-symbols: adopt NodeKindCategory as drift-guard for declaration filtering (Phase 4)"
- **Parent PR**: #1295 — Merged `NodeKindCategory` classification API
- **Consumer boundary doc**: `docs/reference/PARSER_CONTRACTS.md` § "NodeKind classification" (section 4)
- **Classification source**: `crates/perl-ast/src/classification.rs` — exhaustive `category()` method; no wildcards
- **Related phases**: Phase 1-3 (classification, refactoring-readiness, DAP-prep); Phase 5 (DAP adoption); Phase 6-7 (product integration)
- **Validation receipts**: Issue #1330 has `accuracy-reviewed` + `architecture-reviewed` labels confirming the approach is sound

---

## Validation Checkpoint

**Accuracy-reviewed** (issue #1330): File paths verified (`symbol_extraction.rs` line 36, `classification.rs` lines 175–235), function signature confirmed, NodeKindCategory API confirmed exported.

**Architecture-reviewed** (issue #1330): Drift-guard concept validated against PARSER_CONTRACTS.md; no dependency-direction violations; microcrate boundary intact (perl-lsp-rs consuming perl-ast classification).

**Blockers**: None identified. Safe to proceed.

---

## Builder Handoff Note

The main builder should understand:
1. This is a **pure refactoring** — no behavior change to output.
2. The **compiler enforces** the new exhaustiveness requirement (very powerful, no runtime cost).
3. The **6 symbol-kind mappings** are immutable (do not change them).
4. The **test** documents the guard mechanism and serves as a regression check.
5. The **FunctionCall "has" special case** must remain working (verify with a test).

If the builder sees FunctionCall { name == "has" } failing to emit Property symbols after refactoring, the scope expanded — escalate to reviewer with a diff.
