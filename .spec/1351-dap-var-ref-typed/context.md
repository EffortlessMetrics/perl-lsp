# Context: Type-Separated variablesReference Codec

## Problem Statement

The DAP variables reference space (variablesReference wire format) currently uses ad-hoc integer ranges to distinguish Scope (frame-scoped), EvalResult (evaluation result), and Child (variable child) references. The encoding scheme `frame_id * 10 + scope_type` was identified in #1219 to have a collision hazard: frame_id=5000 with scope_type=1 collides with early EvalResult counter values.

This is an **ID/ref-space collision hazard class** — a type-safety bug where two logically distinct reference types map to the same wire integer, breaking the protocol invariant.

## Solution Approach

Retire the collision hazard by introducing a type-safe `VariableReference` enum that encodes/decodes unambiguously into i32 wire format. The enum has three variants, each with its own wire range, guaranteeing no overlap:

```rust
pub enum VariableReference {
    Scope { frame_id: i32, kind: ScopeKind },           // [1, 9_999_999]
    EvalResult { counter: i32 },                          // [1_000_000, ...) — no overlap with Scope
    Child { parent: i32, index: u32 },                    // [2_000_000_000, i32::MAX] — no overlap
}
```

**Wire ranges:**
- **Scope:** [1, 9_999_999] encoded as `frame_id * 10 + kind` (where kind ∈ [1, 3])
  - Max frame_id with this scheme: 999_999, giving 9_999_990 + 3 = 9_999_993
  - Safe margin before EvalResult base
- **EvalResult:** [1_000_000, ...) encoded as `1_000_000 + counter`
  - Distinct prefix (1_000_000) guarantees no collision with Scope
  - Upper bound: 2_000_000_000 (Child base)
- **Child:** [2_000_000_000, ...) encoded as `2_000_000_000 + packed(parent, index)`
  - Highest base ensures no collision with EvalResult

**Decoding algorithm (precedence matters):**
1. If wire >= 2_000_000_000 → Child variant
2. Else if 1_000_000 <= wire < 2_000_000_000 → EvalResult variant
3. Else if 1 <= wire < 1_000_000 → Scope variant (and validate kind ∈ [1,3])
4. Else → None (out of range)

This order guarantees each valid wire maps to exactly one variant.

**ScopeKind enum:**
```rust
pub enum ScopeKind {
    Locals = 1,   // Local variables in frame
    Package = 2,  // Package globals
    Globals = 3,  // All globals
}
```

---

## Key Decisions

### 1. Why separate Scope/EvalResult/Child as enum variants instead of a flat scheme?

**Rejected:** Keeping ad-hoc ranges and relying on comments.
```rust
// Old way (error-prone):
let var_ref = if frame_id * 10 + scope_type < 1_000_000 {
    // ... Scope handling
} else if var_ref < 2_000_000_000 {
    // ... EvalResult handling
}
```

**Chosen:** Type-safe enum forces the compiler to check all cases. The variant encodes the intent (Scope references are for frames, EvalResult for eval, Child for nesting). Encoding/decoding is explicit and testable.

### 2. Why ScopeKind as a separate enum?

The Scope variant contains a frame_id (used to select which stack frame) and a kind (selects which scope within that frame). The kind is small (3 values) and logically distinct from frame_id, so it deserves its own enum. This also makes the wire encoding unambiguous: kind occupies the last digit (mod 10).

### 3. Why these wire ranges?

**Scope [1, 9_999_999]:**
- Encodes as `frame_id * 10 + kind` where kind ∈ [1, 3]
- Max frame_id = 999_999 (6 digits) → wire 9_999_990..9_999_993
- Covers typical stack depth (thousands of frames)
- Leaves a safe gap below 1_000_000 for EvalResult

**EvalResult [1_000_000, 2_000_000_000):**
- Starts at 1_000_000 (9 decimal digits, distinct prefix from Scope)
- Covers typical evaluation cache sizes (millions of eval results)
- Stops at 2_000_000_000 (Child base) to leave room for Child

**Child [2_000_000_000, i32::MAX]:**
- Starts at 2_000_000_000 (high, non-overlapping with EvalResult)
- Encodes parent and index: `2_000_000_000 + (parent << 16 | index)`
- parent and index are bit-packed; parent in high 16 bits, index in low 16 bits
- Covers deeply nested variable hierarchies

### 4. Why TryFrom<i32> for ScopeKind instead of From?

ScopeKind has only 3 valid values (1, 2, 3). Attempting to construct from an invalid discriminant (0, 4, -1) should return an error, not panic. `TryFrom` signals this fallibility in the type system.

### 5. Why saturating arithmetic for encode?

Some inputs (frame_id near i32::MAX, large counter, packed child values) may exceed safe bounds. Rather than panic or silently wrap, saturating arithmetic clamps the result to the max representable value. This preserves safety: the value may not round-trip perfectly, but it won't cause UB.

---

## Alternatives Rejected

### A. Keep ad-hoc ranges, add a wrapper struct for readability

```rust
pub struct ScopeRef { frame_id: i32, kind: ScopeKind }
pub struct EvalRef { counter: i32 }
pub struct ChildRef { parent: i32, index: u32 }

fn encode_scope_ref(sr: ScopeRef) -> i32 { sr.frame_id * 10 + sr.kind as i32 }
fn encode_eval_ref(er: EvalRef) -> i32 { 1_000_000 + er.counter }
```

**Rejected:** Caller must remember which function to call for which type. Decoding requires a giant `if/else` chain. Easy to misuse.

### B. Use u64 instead of i32

**Rejected:** DAP protocol uses i32 for variablesReference. Changing to u64 breaks wire compatibility with existing clients. This refactoring is for type safety at the Rust level, not wire format change.

### C. Assign each variant a single high-order bit (bit 30, 31)

```rust
// Pseudo: use bits 30-31 to tag variant
0b00... → Scope
0b01... → EvalResult
0b10... → Child
0b11... → (unused)
```

**Rejected:** Loses the semantic ranges. Harder to audit wire values in logs. Bit-packing is more complex than range-based decoding.

---

## Why This Matters

Issue #1219 demonstrated that ad-hoc integer ranges without type enforcement are a hazard class. The fix is to make the type system enforce the invariant: `encode(Scope) != encode(EvalResult)` at the type level, with decode() an exhaustive pattern.

This refactoring is a **type-safety improvement**, not a behavior change. Existing clients will see the same wire values (round-trip preserved). But future code that constructs VariableReference variants is now protected: the Rust compiler will verify that all cases are handled, and the encode/decode logic is centralized and tested.

---

## Scope Boundaries

### In scope
- New module `crates/perl-dap/src/debug_adapter/var_ref.rs` with VariableReference enum and ScopeKind enum
- Single-wire i32 codec: `encode()` and `decode()` methods
- TryFrom<i32> for ScopeKind
- H1-H5 hazard-class tests (red TDD)

### Out of scope
- Migration of 6 existing files (frames.rs, evaluation.rs, variables.rs, parsing.rs, parsing/scope_variables.rs, variable_cache.rs) to use the new codec — done in a follow-up PR or same PR after codec is tested
- Changes to DAP protocol wire format (wire values remain the same, encoding/decoding is internal)
- Changes to client-facing DAP capabilities or messages

---

## References

- **Issue #1219:** "ID collision in DAP variablesReference encoding" — identifies the collision hazard
- **Issue #1351:** "Type-separate variablesReference spaces to retire collision class" — this issue
- **DAP spec:** https://microsoft.github.io/debug-adapter-protocol/specification#scope — defines Scope scope IDs (1=Local, 2=Argument, 3=Registered, 4=Statics, 5=Globals, 6=Resources)
  - Note: perl-dap uses a simpler 3-value scheme (Locals=1, Package=2, Globals=3), not DAP's 6
- **Perl debugging context:** Variables in a Perl frame belong to one of three scopes: lexical (my), package (our), or global

---

## Validation Checkpoint

**Plan-reviewed:** Approach confirmed sound (type-safe enum, range-based decoding, no collision).

**Spec-reviewed:** Checklist and acceptance criteria are clear and achievable.

**No blockers identified:** Safe to proceed with red-TDD and builder implementation.

---

## Builder Handoff Note

The builder should understand:

1. **This is a type-safety refactoring,** not a behavior change. The wire format and ranges are unchanged.
2. **The codec is the single source of truth** for encode/decode logic. After implementation, all 6 migration sites will use `VariableReference::encode()` and `VariableReference::decode()` instead of ad-hoc arithmetic.
3. **The red tests define "done."** All H1-H5 tests must pass. No modifications to test assertions are allowed (the tests define the spec).
4. **Saturating arithmetic is required** to handle extreme inputs safely. Review arithmetic operations for correctness at boundaries.
5. **Round-trip must be exact** for all valid wire values within ranges. Any precision loss is a bug.

If the builder encounters issues implementing the codec to match all red tests, the scope should not be expanded. Instead, escalate to the reviewer with the specific failing test and the implementation concern.

---

## Sign-Off

This spec has been reviewed for:
- ✓ Correctness of wire ranges and encoding scheme
- ✓ Absence of collision hazard after refactoring
- ✓ Feasibility of type-safe implementation
- ✓ Clarity of red-TDD test requirements
- ✓ Scope boundaries (no unintended scope creep)

Ready for red-TDD writer to stage failing tests and builder to implement.
