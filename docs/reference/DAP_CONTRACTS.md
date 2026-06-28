# DAP Contract Index

**Purpose.** One place an agent or PR author can ask:
*"What contract does this DAP change touch? What tests must change? What must not be bypassed?"*

Every contract below names: the invariant that must hold, the owner module, the
consumers, the proof tests and oracle, known exceptions, and non-goals / future work.

This document is kept factual and citable. Claims without a primary artifact
(file path, test name, merged PR) are not made.

**Related:** For parser-level behavioral contracts, see
[docs/reference/PARSER_CONTRACTS.md](PARSER_CONTRACTS.md).
For subsystem-level hazard rows that spec-planner seeds into `acceptance.md`, see
[docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md](SUBSYSTEM_HAZARD_DEFAULTS.md).

---

## 1. variablesReference Wire-Band Codec

### Contract

The DAP `variablesReference` field is a single `i32` that encodes references to three
logically distinct spaces. This contract governs the codec that maps typed Rust values
to and from that wire integer.

**Wire bands (pairwise disjoint):**

| Variant | Wire Range | Encoding formula |
|---|---|---|
| `Scope` | [1, 999_999] | `frame_id * 10 + kind` (frame_id ∈ [0, 99_999], kind ∈ {1=Locals, 2=Package, 3=Globals}) |
| `EvalResult` | [1_000_000, 1_999_999_999] | `1_000_000 + counter` |
| `Child` | [2_000_000_000, i32::MAX] | `2_000_000_000 + (parent << 16 \| (index & 0xFFFF))` |

Wire value `0` is reserved by the DAP spec ("no children") and is **never** a valid encode output.
Negative values are invalid.

**Decode is PURE-RANGE.** The decoder tests which band `raw` falls into using only
range comparisons — no residue arithmetic (`% 10`), no discriminant extraction across bands.
Because the bands are pairwise disjoint, no value can match more than one case.

**Encode is FALLIBLE.** `VariableReference::encode() -> Option<i32>` returns `None` and
MUST reject any input that would produce a wire value outside its declared band:

- `Scope`: `None` when `frame_id ∉ [0, 99_999]` (frame_id=100_000 yields wire 1_000_001,
  which falls in the EvalResult band).
- `EvalResult`: `None` when `counter < 0` or when `1_000_000 + counter > 1_999_999_999`
  (counter ≥ 1_999_000_000 would push into the Child band).
- `Child`: `None` when `parent < 0` (negative parent yields a wire value in the EvalResult band).

**INVARIANT:** Every site that produces or consumes a `variablesReference` wire value MUST
route through `VariableReference::encode` / `VariableReference::decode`. No call site may
perform raw arithmetic on a `variablesReference` integer outside of this codec module.
This includes `fallback_scope_variables` and any other ad-hoc reference allocators.

**Issue #1445** tracks the one known surviving unmigrated site (`fallback_scope_variables`
collision risk). Until that issue is closed, treat any ad-hoc reference arithmetic in
`perl-dap` as a hazard-class violation and route to #1445.

A mechanical LINT / test to enforce this invariant for all future ref-producing sites
is tracked in #1445's spec; it is not yet implemented.

### Owner module

`crates/perl-dap/src/debug_adapter/var_ref.rs`

Key types:
- `VariableReference` (enum with `Scope`, `EvalResult`, `Child` variants)
- `ScopeKind` (enum: `Locals=1`, `Package=2`, `Globals=3`)
- `VariableReferenceError`

Key constants (pairwise-disjoint band boundaries):

```
SCOPE_MIN = 1
SCOPE_MAX = 999_999
SCOPE_FRAME_ID_MAX = 99_999
EVAL_BASE = 1_000_000
EVAL_MAX = 1_999_999_999
CHILD_BASE = 2_000_000_000
```

### Consumers

Any DAP handler that allocates or looks up a `variablesReference`:
- `variables` request handler (scope variable lookup)
- `evaluate` request handler (eval result variable trees)
- `scopes` response builder (allocates Scope refs per frame)
- Any child-variable expansion path

### Proof

**Three band-overflow bugs caught during capstone (#1430 / #1444, merged):**

1. **green-tdd caught (bug 1):** `EvalResult { counter: 1 }` encoded to wire 1_000_001.
   The original decoder used `raw % 10 ∈ {1,2,3}` residue-based disambiguation and decoded
   1_000_001 as `Scope` (residue 1) instead of `EvalResult`. Overlapping residues across
   bands gave the wrong variant for any counter whose low digit matched a valid `ScopeKind`.

2. **deep-review caught (bug 2):** `EvalResult` counter had no upper-bound check. A large
   counter (≥ 1_999_000_000) produced a wire value ≥ 2_000_000_000, which crossed into the
   `Child` band and decoded as `Child` (silent wrong variant, no error).

3. **deep-review caught (bug 3):** `Child { parent: -1 }` produced a negative wire offset,
   landing in the EvalResult band (wire ≈ 999_999_xxx) and decoding as `EvalResult` (wrong
   variant, no error).

These three failures share the same root: the codec relied on a convention
("we never allocate those values there") rather than mechanical enforcement
("the encoder rejects inputs that cross the band boundary"). The fix replaced
residue-based disambiguation with pure-range decode and made encode fallible.

**Issue #1445** — surviving unmigrated `fallback_scope_variables` site that still uses
ad-hoc arithmetic; represents the ongoing hazard that this invariant must cover.

**Test suite** (inline `#[cfg(test)]` in `var_ref.rs`):
- `scope_kind_tryfrom_valid` / `scope_kind_tryfrom_invalid`
- `scope_encode_decode_basic`
- `evalresult_encode_decode_roundtrip` — verifies counter=1 and counter=3 decode as EvalResult (not Scope)
- `scope_frame_id_max_boundary` — frame_id=99_999 is valid
- `scope_frame_id_over_max_returns_none` — frame_id=100_000 rejected
- `child_encode_decode_base`
- `decode_zero_none` / `decode_negative_none`
- `decode_invalid_scope_kind_none`
- `bands_are_disjoint_no_scope_wire_in_eval_range` — static assertion that max Scope wire < EVAL_BASE
- `evalresult_negative_counter_returns_none` — guard for bug 2 fix
- `evalresult_overflow_into_child_band_returns_none` — guard for bug 2 fix
- `child_negative_parent_returns_none` — guard for bug 3 fix

### Known exceptions / specializations

The `ScopeKind` residue (`raw % 10`) is still used *within* the Scope band to extract
the kind discriminant after the pure-range check has already confirmed `raw ∈ [1, 999_999]`.
This is not a cross-band disambiguation — it is a within-band field extraction. The contract
prohibits cross-band residue tricks, not within-band field packing.

### Non-goals / future migrations

- A future lint / static analysis tool should enforce the "no raw arithmetic on variablesReference"
  invariant mechanically. Issue #1445 tracks this. Until that lint exists, code review is the
  sole enforcement gate for new ref-producing sites.
- The `Child` band uses a packed `(parent << 16 | index)` encoding that limits `index` to 16 bits
  and `parent` to the top portion of the band. A future change that needs more child capacity
  should propose a new band layout and update this contract.

### Cross-links

- Incident write-up: [docs/learnings/2026-06-tagged-range-codec-band-overflow.md](../learnings/2026-06-tagged-range-codec-band-overflow.md)
- Portable pattern: [docs/concepts/type-level-id-space-promotion.md](../concepts/type-level-id-space-promotion.md)
- Original collision (pre-typed-enum): [docs/learnings/2026-06-dap-ref-space-collision.md](../learnings/2026-06-dap-ref-space-collision.md)
- Hazard class: DAP-1 in [docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md](SUBSYSTEM_HAZARD_DEFAULTS.md)
- Open work: issue #1445 (unmigrated `fallback_scope_variables` site + lint)

---

## Cross-Reference

| Contract | Owner crate | Key test file | Governing PR |
|---|---|---|---|
| variablesReference wire-band codec | `perl-dap` | `crates/perl-dap/src/debug_adapter/var_ref.rs` (inline tests) | #1430, #1444; open #1445 |
