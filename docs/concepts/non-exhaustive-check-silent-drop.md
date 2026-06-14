# Non-Exhaustive Check / Silent Drop

*Portable concept. Grounded in perl-lsp. See also: [shift-left-ladder](shift-left-ladder.md), [hazard-class-invariants](hazard-class-invariants.md), [verify-the-instrument](verify-the-instrument.md).*

---

## The pattern

A check that does not enumerate all cases — and proceeds silently as if it did — is a non-exhaustive check with silent drop. New cases are not caught; they are skipped without signal.

This pattern appears at three altitudes with identical structure:

**Code level**: A `match` on an enum that uses `_ => {}` wildcards, or an `if let` that only handles one variant, silently ignores all other variants. When a new variant is added to the enum, the `match` compiles cleanly and the new variant is silently dropped. No test fails; the behavior is simply absent.

Example from PR #1457/#1459: Three consumers of `NodeKind` used `if let` or wildcard arms. When new `NodeKind` variants were added, all three consumers silently skipped them. Semantic tokens, hover, and go-to-definition were missing for the new node types — not because of a logic error, but because the check never looked.

**CI level**: A `cargo clippy --lib` or `cargo test --lib` invocation silently omits binary targets, integration tests, and example targets. The gate appears to cover the PR's surface. It does not. The untested surface is exactly the gap between what the gate covers and what "the build is correct" requires.

Example from PR #1282/#1458: Coverage was measured with `--lib` only. Integration test lines were silently excluded from profdata. The coverage percentage was healthy; the covered surface was narrower than the repo's correctness model required. When a duplicate function was introduced, the gate with narrow scope passed while master broke.

**Process level**: A spec or review checklist that says "handle all edge cases" without enumerating them is a non-exhaustive check. When a builder reads it, they handle the cases they can think of and silently skip the ones they cannot. The checklist passes; the cases are dropped.

Example: A spec that lists "handle empty input" but does not enumerate the three distinct empty-input paths through a parser function. Builders handle the obvious path and skip the non-obvious ones. The spec passes; the edge cases are absent.

---

## The isomorphism

All three instances share the same structure:

1. A check is defined over a set of cases.
2. The set is not enumerated — it is described by a wildcard, a flag, or a phrase.
3. A new case appears (new enum variant, new target type, new edge case class).
4. The check runs. The new case falls into the wildcard / out of scope / outside the phrase.
5. The check passes. The new case is silently absent.
6. The absence is discovered later — by a user, by a downstream agent, by a master break.

The discovery cost is proportional to how far downstream the absence propagates before detection. A compiler-enforced exhaustive `match` catches it at compile time. A CI gate with the wrong scope catches it at CI time (or not at all). A process checklist with imprecise language catches it at review time — or not until a user files a bug.

---

## Counter-move ladder (cheap first)

**Rung 1: Exhaustive Rust `match` (compile-time, zero marginal cost)**
Replace `_ => {}` wildcards and `if let` patterns with exhaustive `match`. When a new variant is added, the compiler requires all match arms to be updated. Cost: one-time refactor. Benefit: all future additions are forced to handle every existing consumer before the code compiles.

```rust
// Non-exhaustive: silently drops new variants
if let NodeKind::Scalar(n) = kind { ... }

// Exhaustive: new variants cause compile error until handled
match kind {
    NodeKind::Scalar(n) => { ... }
    NodeKind::Array(n) => { ... }
    // Adding NodeKind::Hash forces a new arm here
}
```

**Rung 2: Full CI target scope (one-time config fix)**
Replace `--lib` with `--all-targets` for clippy and test invocations. Replace coverage profdata collection that excludes integration tests with invocations that include them. Cost: one CI config change. Benefit: all future PRs are checked against the full target surface.

```bash
# Before: silently excludes binary targets, integration tests
cargo clippy --lib
# After: all targets, all test kinds
cargo clippy --all-targets
```

**Rung 3: Explicit enumeration in specs and review checklists (5% overhead, high confidence)**
Replace "handle all edge cases" with an enumerated list: "handle (a) empty input, (b) single-token input, (c) input with only whitespace." Each item in the list is a case the builder must explicitly address. Cases not in the list can be added by the builder if discovered, but the listed cases cannot be dropped.

**Rung 4: Runtime guards for genuinely optional cases (expensive, last resort)**
For cases that cannot be eliminated at compile time (dynamic dispatch, external inputs), add explicit runtime checks that log or return errors for unrecognized cases rather than silently skipping them. This is the most expensive option — it adds per-call overhead and requires maintenance — and should be used only when rungs 1-3 are not applicable.

---

## Why it works

The counter-move ladder works because it shifts the silence into signal. At each rung:

- Rung 1 makes the compiler produce an error (not silence) when a new variant is unhandled.
- Rung 2 makes CI fail (not pass) when the full target surface has an error.
- Rung 3 makes the review checklist flag (not pass) when an enumerated case is not addressed.
- Rung 4 makes the runtime log or fail (not continue) when an unrecognized case is encountered.

In each case, the new case produces a signal. The signal is caught by the appropriate layer. The absence of signal means the case was handled — not that it was silently dropped.

---

## When to apply

Apply rung 1 (exhaustive match) immediately when a `_ => {}` or `if let` pattern is discovered on a type that is expected to grow. This is a one-time cost with compounding benefit.

Apply rung 2 (full target scope) when a CI incident is traced to a scope gap. The fix is a one-line change to the CI invocation; the benefit is permanent.

Apply rung 3 (explicit enumeration) when writing a spec or review checklist for any area with known edge case classes. The cost is ~5% more writing time; the benefit is that builders cannot skip listed cases.

Apply rung 4 (runtime guards) only when the case set is genuinely dynamic — external plugin systems, user-extensible configurations, dynamic dispatch over trait objects. Not for internal enum variants, which rung 1 handles more cheaply.

---

## Worked example: #1457/#1459 NodeKind variant addition

A new `NodeKind::HashSubscript` variant was added in PR #1457. Three consumers in the semantic token and LSP providers used non-exhaustive patterns:

- Consumer A: `if let NodeKind::Scalar(n) | NodeKind::Array(n) = kind { ... }` — no `HashSubscript` arm
- Consumer B: `match kind { NodeKind::Scalar(_) => ..., _ => {} }` — wildcard drop
- Consumer C: `if let NodeKind::Interpolated(n) = kind { ... }` — other variants invisible

All three compiled cleanly. All three silently dropped `HashSubscript`. Semantic tokens, hover, and go-to-definition were absent for hash subscript expressions.

Detection: green-tdd agent added a test that asserted semantic tokens for a hash subscript expression. The test failed. The absence was surfaced.

Fix (PR #1459): all three consumers converted to exhaustive `match`. Future `NodeKind` additions now require all three consumers to be updated before the code compiles.

Rung promoted: the `NodeKind` variant-addition hazard class was added to `hazard-class-invariants.md` with the exhaustive-match invariant, so future specs involving `NodeKind` additions include this check by default.

---

## Relation to other patterns

- **Shift-left ladder** (`shift-left-ladder.md`) — the counter-move ladder is the shift-left ladder applied to the non-exhaustive-check class specifically. Rung 1 (compile-time) is the highest and cheapest rung; rung 4 (runtime) is the lowest and most expensive.
- **Hazard-class invariants** (`hazard-class-invariants.md`) — the NodeKind variant hazard class and the coverage scope hazard class are both instances of this pattern captured as durable invariants. New PRs in those areas inherit the invariant automatically.
- **Verify the instrument** (`verify-the-instrument.md`) — a CI gate with the wrong scope is an instrument that cannot verify what it appears to verify. The rung-2 fix (full target scope) is an instrument repair, not a code fix.
