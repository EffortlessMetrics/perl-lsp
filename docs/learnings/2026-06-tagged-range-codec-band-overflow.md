---
tags: [id-collision, bounds, dap, debugger, codec, band-overflow, tagged-range]
repos: [perl-lsp-swarm]
related: ["#1219", "#1351", "#1430"]
portable: false
article_asset: true
search_terms: [variablesReference, var_ref, tagged-range, band-overflow, encode_option, ScopeKind, EvalResult, VariableReference, Scope, Child, 1_000_000, 2_000_000_000, residue-disambiguation, disjoint-bands, #1219, #1351, #1430]
---

# Type-enum promotion re-introduced the ID-collision class through the wire codec

**Date**: 2026-06
**Hazard class**: id-collision, bounds
**Portable lesson**: [docs/concepts/type-level-id-space-promotion.md](../concepts/type-level-id-space-promotion.md)

## What happened

Issue #1351 (PR #1430) retired the #1219 ID-collision class by promoting the raw `i32`
variablesReference wire value to a typed `enum VariableReference { Scope, EvalResult, Child }`.
This was the structurally correct move -- but the FIRST implementation re-introduced the
collision class THREE separate ways inside the wire codec, all sharing the same root cause:
an encode arm that assumed its output stayed inside its declared wire band without checking
the boundary against adjacent bands.

1. **green-tdd caught**: `EvalResult { counter: 1 }` encoded to wire value 1_000_001.
   The decoder used `raw % 10 ∈ {1,2,3}` residue-based disambiguation across the full
   integer range. Wire 1_000_001 decoded as `Scope` (residue 1) instead of `EvalResult`.
   Overlapping bands plus residue tricks gave the wrong variant for any counter with the
   right low digit.

2. **deep-review caught**: `EvalResult` counter had no upper-bound check. A large counter
   produced a wire value >= 2_000_000_000, which crossed into the `Child` band and decoded
   as `Child` (silent wrong variant, no error).

3. **deep-review caught**: `Child { parent: -1 }` produced a negative wire offset, landing
   in the EvalResult band (wire ~= 999_999) and decoding as `EvalResult` (wrong variant,
   no error).

The result: three distinct inputs silently decoded to the wrong variant -- the same
behavioral class as the original #1219 collision, just inside the codec rather than
between two independent allocators.

## Why

Type promotion to an enum eliminates the collision at the Rust type level: you can no
longer pass a `Scope` reference where a `Child` reference is expected. But the enum's
wire serialization still used an encoding convention ("EvalResult lives in the million
band, Child lives in the two-billion band") without enforcement. Specifically:

- The decode path used residue arithmetic (`raw % 10`) instead of pure range comparison,
  meaning any value in any band decoded to the variant whose residue matched -- there was
  no band-exclusive decode.
- The encode paths did not check whether their output remained inside their declared band;
  out-of-range inputs were silently truncated or wrapped.
- A negative parent ID in `Child` was never ruled out as invalid input from the DAP
  client.

The collision re-entered through the codec precisely because the codec relied on an
allocation convention ("we never produce those values") rather than a mechanical enforcement
("the encoder rejects inputs that would cross the boundary").

## Fix

Replace the convention-based codec with a pure-range disjoint-band design:

- `Scope` sub-set [1, 999_999]
- `EvalResult` sub-set [1_000_000, 1_999_999_999]
- `Child` sub-set [2_000_000_000, ...)

Decode by range only -- no residue tricks. Encode via `encode() -> Option<i32>` that
returns `None` and lets the caller propagate an error for any input that would cross a
band boundary (frame_id > 99_999, counter overflow into Child range, negative parent).
Adversarial tests: `EvalResult{counter:0}`, `EvalResult{counter:MAX}`, `Child{parent:-1}`,
`Scope{frame:99_999}`, `Scope{frame:100_000}` -- assert correct variant or `None`.

## Spec impact

Motivated the portable rule in
[docs/concepts/type-level-id-space-promotion.md](../concepts/type-level-id-space-promotion.md).
Extends Class 1 (ID/Reference-Space Collision) in
[docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) with
the observation that type-level promotion is necessary but not sufficient -- the codec must
also enforce disjoint ranges mechanically.

Cross-reference: the original #1219 collision (convention-based integer ranges without
types) is documented in
[2026-06-dap-ref-space-collision.md](2026-06-dap-ref-space-collision.md). This incident
is the sequel: types were added but the codec re-introduced the same class.

## Portable lesson

Type promotion to an enum is necessary but not sufficient to eliminate ID-collision.
If the wire codec uses overlapping bands or residue-based disambiguation, the collision
class re-enters through the codec. Make the encoder fallible and the decoder pure-range.

- **Pattern**: [docs/concepts/type-level-id-space-promotion.md](../concepts/type-level-id-space-promotion.md)
- **Class**: Class 1 -- ID/Reference-Space Collision (codec variant)
- **Generalization**: An allocation convention ("we never put those values there") is not
  enforcement; a fallible encoder and a pure-range decoder are.

## Shift-left evidence

green-tdd (haiku, cheap) caught bug 1. deep-review (sonnet, expensive) caught bugs 2 and
3 after green-tdd. This confirms two things: the shift-left ladder works (adversarial
green-tdd tests are worth writing for novel codec infrastructure), and deep-review remains
mandatory for novel infrastructure even after shift-left -- the shift-left pass reduced the
deep-review finding count but did not eliminate all gaps.

See [2026-06-shift-left-validated.md](2026-06-shift-left-validated.md) for the broader
shift-left validation.

## Related PRs

- [#1351](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1351) -- issue: variablesReference capstone -- promote to typed enum codec
- [#1430](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1430) -- PR: implement VariableReference enum with disjoint-band codec
- [#1219](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1219) -- original collision: EvaluateResult base 50_000 vs scope-ref formula
