# Context: #1445 — fix(dap): fallback_scope_variables child refs collide with EvalResult wire band

## Problem

The `fallback_scope_variables()` function in `crates/perl-dap/src/debug_adapter/parsing.rs` (lines 234–282) generates placeholder variables when the Perl debugger output is unavailable. For each expandable variable (hash or array), it computes a child reference via `variables_ref.saturating_mul(100) + offset` (lines 248 and 256).

This arithmetic is problematic: for Scope refs with high frame IDs (e.g., `frame_id > ~10_000`), the computed child ref lands in the EvalResult wire band `[1_000_000, 1_999_999_999]`. A DAP client receiving such a ref and requesting its expansion would have it decoded by `VariableReference::decode()` as an **EvalResult**, not a Child — the exact ID-space collision class (#1219) that the var_ref codec (#1430) + consumer migration (#1432) were designed to eliminate.

**Severity:** Edge-case (requires call-stack depth > ~10_000 frames, unrealistic for normal `perl -d` debugging), but a genuine protocol-safety bug in an unmigrated code path. The collision corrupts the disjoint-band invariant that the codec establishes.

**Found by:** Reviewer-deep audit of PR #1444 (the consumer migration). The PR description explicitly notes that `compute_child_reference()` in `scope_variables.rs` uses `parent*1000+index` and was "deliberately NOT migrated" due to different contract (Scope-child encoding vs. VariableReference::Child). However, `fallback_scope_variables()` uses the same `*100+offset` arithmetic and should have been migrated — it was overlooked.

## Why this approach

The fix is straightforward: migrate `fallback_scope_variables()` to use `VariableReference::Child` codec, which produces refs in the disjoint band `[2_000_000_000, i32::MAX]`.

### Key decisions:

1. **Use VariableReference::Child::encode() instead of raw arithmetic.**
   - Each placeholder child (e.g., `$self`, `@_`) is assigned a stable index (0, 1, ...) based on position in the fallback vec.
   - Call `VariableReference::Child { parent: variables_ref, index }.encode().unwrap_or(0)` to produce a wire value.
   - The `encode()` function saturates and uses the formula `2_000_000_000 + (parent << 16 | (index & 0xFFFF))`, which is provably in the Child band.
   - If `encode()` returns `None` (negative parent), fallback to `0` (DAP "no children" sentinel) — safe and correct.

2. **Add a mechanical guard test to prevent future ad-hoc arithmetic.**
   - Create `crates/perl-dap/tests/dap_var_ref_arithmetic_guard_tests.rs` with a test that uses `grep` to scan `parsing.rs` for raw variablesReference arithmetic patterns (`%`, `/`, `*10`, `*100`, `1_000_000 +`, etc.).
   - If any matches are found (outside comments/tests), the test fails with the offending line numbers.
   - This enforces the contract: **only `var_ref.rs` produces/consumes variablesReference via arithmetic; all other files use VariableReference::encode/decode**.

3. **Write comprehensive collision and round-trip tests.**
   - `test_fallback_scope_variables_deep_frame_child_ref_no_collision`: Verify that a high-frame-id Scope ref's children decode as Child, never EvalResult.
   - `test_child_ref_encode_decode_roundtrip_deep_frame`: Verify that `Child::encode()` followed by `decode()` preserves identity and lands in the Child band.
   - `test_fallback_child_ref_never_in_eval_band`: Adversarial test asserting no child ref ever lands in `[1_000_000, 1_999_999_999]`.

4. **Preserve backward compatibility and existing behavior.**
   - The function signature of `fallback_scope_variables()` is unchanged.
   - Wire values are opaque to clients — they just pass them back in subsequent requests. Only the internal encoding formula changes.
   - Variable-expansion behavior (from `handle_variables()`) is transparent to the ref arithmetic; no consumer code breaks.

5. **Require deep-review sign-off.**
   - Per PR #1444 closure, DAP identity-semantics changes must pass deep-review before merge.
   - The reviewer must confirm: disjoint-band invariant holds by construction, round-trip correctness, guard test effectiveness, and no regressions in variable-expansion tests.

## Alternatives rejected

- **Alternative A: Apply the same fix to `compute_child_reference()` in scope_variables.rs.**
  - Rejected: `compute_child_reference()` uses `parent*1000+index` and is part of the "real" variable rendering (not fallback placeholders). Changing its arithmetic would alter wire values for all actual variables and potentially break client-side caching or ordering. Explicitly left out-of-scope in PR #1444. This fix addresses only `fallback_scope_variables()`, which is a placeholder-only path.

- **Alternative B: Cap the child index to prevent overflow.**
  - Rejected: Capping would mask the root problem (wrong wire band) rather than fix it. The issue is not that the index is too large; it's that the arithmetic formula produces values in the wrong band. Using the codec fixes the root cause.

- **Alternative C: Use a different offset formula (e.g., `parent*10000 + index`) to avoid collision.**
  - Rejected: Ad-hoc arithmetic is precisely what the codec was designed to retire. Even if a new offset formula avoided collision for the current frame_id range, it could collide with future expansions of the Scope band or frame_id range. The codec provides a principled, future-proof solution.

- **Alternative D: Leave fallback_scope_variables alone and add a boundary check in handle_variables().**
  - Rejected: Boundary checks are reactive (catch collisions at runtime) rather than preventive. The codec-based approach prevents collisions by construction. Boundary checks don't satisfy the DAP-1 invariant requirement (provable disjointness).

## Prior art / duplicates

The var_ref codec (`crates/perl-dap/src/debug_adapter/var_ref.rs`) was introduced in PR #1430 to solve issue #1219 (ID-space collision). The codec provides three disjoint bands with pure-range decode logic. PR #1432 (consumer migration) updated most call sites to use the codec; `fallback_scope_variables()` was overlooked. This fix completes the migration.

**Learnings:**
- [docs/learnings/2026-06-dap-ref-space-collision.md](../learnings/2026-06-dap-ref-space-collision.md) — documents the original #1219 incident and the codec design.
- [docs/learnings/2026-06-tagged-range-codec-band-overflow.md](../learnings/2026-06-tagged-range-codec-band-overflow.md) — explains the band-based codec pattern and hazard class mitigation.

**Related code:**
- `crates/perl-dap/src/debug_adapter/var_ref.rs` — the codec module (stable, no changes needed).
- `crates/perl-dap/src/debug_adapter/parsing/scope_variables.rs::compute_child_reference()` — uses `parent*1000+index` (deliberately not migrated; different contract).
- `crates/perl-dap/src/debug_adapter/variables.rs::handle_variables()` — calls `fallback_scope_variables()` on the fallback path.

**No duplicates found.** This is a targeted fix for a specific unmigrated code path identified during deep-review of PR #1444.

## Links

- **Issue:** #1445 — fallback_scope_variables child refs collide with EvalResult wire band
- **Related PR:** #1444 — refactor(dap): migrate variablesReference consumers to VariableReference codec (#1432)
- **Original incident:** #1219 — DAP ID/ref-space collision class identified
- **Codec introduction:** #1430 — DAP variable-reference codec (var_ref.rs)
- **Learnings:**
  - [docs/learnings/2026-06-dap-ref-space-collision.md](../learnings/2026-06-dap-ref-space-collision.md) — #1219 incident and codec design
  - [docs/learnings/2026-06-tagged-range-codec-band-overflow.md](../learnings/2026-06-tagged-range-codec-band-overflow.md) — band-based codec and hazard mitigation
- **Contracts:**
  - [docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md — DAP-1](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md) — ID/ref-space collision invariant
  - [crates/perl-dap/src/debug_adapter/var_ref.rs §Solution](../../crates/perl-dap/src/debug_adapter/var_ref.rs) — disjoint-band codec specification (lines 16–29)
  - [DAP wire-band security model](../concepts/hazard-class-invariants.md) — Class 1 (ID/ref-space collision)
- **Design patterns:**
  - [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) — six cross-subsystem hazard classes; this fix addresses Class 1
  - [docs/concepts/multi-angle-haiku-early-spec.md](../concepts/multi-angle-haiku-early-spec.md) — spec-builder workflow used to validate this fix's hazard coverage
