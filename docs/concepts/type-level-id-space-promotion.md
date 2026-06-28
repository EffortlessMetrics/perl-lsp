# Type-Level ID-Space Promotion

## The pattern

When the same primitive type (e.g. `i32`) encodes values from multiple semantic domains
and those domains can collide, the standard fix is to promote from the primitive to
distinct newtypes or a sum type (enum). This eliminates the collision at the Rust type
level: you can no longer pass a value from domain A where domain B is expected.

**But type-level promotion is necessary, not sufficient.**

The wire or serialized form of the promoted type must satisfy an additional property:
provably-disjoint bands decoded purely by range, with a fallible encoder that rejects any
input that would cross a band boundary.

---

## The rule

> Promote the shared primitive to distinct newtypes or an enum.
> Use provably-disjoint wire ranges decoded purely by range comparison.
> Make the encoder fallible: `encode() -> Option<WireType>` returns `None` for any input
> that would land outside its declared band.
> An allocation convention ("we never allocate those values") is NOT enforcement.

---

## Why allocation conventions fail

A convention says: "EvalResult lives in [1_000_000, 1_999_999_999]; Child lives in
[2_000_000_000, ...)." The encoder doesn't check. The decoder uses residue arithmetic
(`raw % 10 in {1,2,3}`) to distinguish variants rather than range boundaries.

Three failure modes follow directly:

1. **Overlapping residues**: `EvalResult{counter:1}` encodes to 1_000_001; residue 1
   is also the Scope residue. Decoder returns `Scope` instead of `EvalResult`.

2. **No upper bound on counter**: a large counter produces a wire value >=
   2_000_000_000 and crosses into the Child band. Decoder returns `Child`.

3. **Negative parent ID**: `Child{parent:-1}` produces a negative wire offset, landing
   inside the EvalResult band. Decoder returns `EvalResult`.

All three are silent wrong-variant returns with no error. The type system cannot catch
them because they are codec bugs, not type errors.

---

## The structural fix

```
Band layout (example -- substitute your actual domain values):
  Scope     : [1,           999_999]        (max 999_999 entries)
  EvalResult: [1_000_000,   1_999_999_999]  (max ~1 billion entries)
  Child     : [2_000_000_000, i32::MAX]     (remaining space)

Decode: pure range comparison, no residue arithmetic.
Encode: fallible -- return None if the input would cross a band boundary.
```

Adversarial test grid (required for any change that introduces a wire codec):

| Input | Expected |
|-------|----------|
| `Scope{frame: MAX_VALID}` | `Some(999_999)` |
| `Scope{frame: MAX_VALID+1}` | `None` |
| `EvalResult{counter: 0}` | `Some(1_000_000)` |
| `EvalResult{counter: MAX}` | `None` (would cross into Child) |
| `Child{parent: -1}` | `None` |
| `Child{parent: 0}` | `Some(2_000_000_000)` |

The decoder must also be tested: supply a wire value from each band and assert the correct
variant is returned. Supply a wire value of 0 and assert an error or None.

---

## Relation to other patterns

- **Hazard-class invariants** [[hazard-class-invariants]]: Class 1 (ID/Reference-Space
  Collision) covers the original collision between independent allocators sharing an
  untyped integer space. This pattern is the sequel: types are added but the codec
  re-introduces the collision. The two documents together cover the full lifecycle of
  this hazard class.

- **Shift-left ladder** [[shift-left-ladder]]: A fallible encoder and adversarial band-
  crossing tests belong on the "spec acceptance criteria + red-TDD" rung. Front-loading
  them moves deep-review from discovery to confirmation.

---

## Worked example

Issue #1351, PR #1430 (`perl-lsp-swarm`): the DAP `variablesReference` field carried
three semantic domains in a single `i32`. After the #1219 collision (Scope vs EvalResult
with base 50_000), the fix promoted to `enum VariableReference { Scope, EvalResult, Child }`.
The first codec implementation used residue disambiguation and no upper-bound checks;
green-tdd found the Scope/EvalResult overlap, deep-review found the EvalResult/Child
overflow and the negative-parent Child case. The second implementation used pure-range
bands and a fallible encoder; all three adversarial cases returned `None` cleanly.

Full incident: [docs/learnings/2026-06-tagged-range-codec-band-overflow.md](../learnings/2026-06-tagged-range-codec-band-overflow.md)
