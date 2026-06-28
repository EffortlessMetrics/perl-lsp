# Acceptance Criteria: #1445 — fix(dap): fallback_scope_variables child refs collide with EvalResult wire band

## §Behavior

| Input / Condition | Expected Result | Notes |
|---|---|---|
| Call `fallback_scope_variables(variables_ref, 0, 10)` where `variables_ref` encodes `Scope { frame_id: 0, kind: Locals }` | Returns vec of placeholder variables with child refs in Scope/Child band (no collision) | Normal low-frame case; child refs use Child band `[2_000_000_000, ..)` |
| Call `fallback_scope_variables(variables_ref, 0, 10)` where `variables_ref` encodes `Scope { frame_id: 50_000, kind: Locals }` | Returns vec of placeholder variables with child refs in Child band, NOT EvalResult band | Deep-frame case (pre-fix would collide with `[1_000_000, 1_999_999_999]`); post-fix uses Child band |
| Call `fallback_scope_variables(variables_ref, 0, 10)` where `variables_ref` encodes `Scope { frame_id: 99_999, kind: Locals }` | Returns vec of placeholder variables with child refs in Child band, decodable as `VariableReference::Child` | Maximum valid Scope frame_id; child refs are provably non-colliding |
| Each child ref wire value produced by `fallback_scope_variables` is passed to `VariableReference::decode()` | Returns `Some(VariableReference::Child { parent, index })`, never `EvalResult` or error | Round-trip correctness: encode-decode cycle preserves identity |
| Guard test scans `crates/perl-dap/src/debug_adapter/parsing.rs` for raw arithmetic patterns (%, /, *, 1_000_000, 2_000_000_000, etc.) | No matches found outside comments/tests (or fails with specific line numbers) | Mechanical enforcement: only `var_ref.rs` produces variablesReference via arithmetic |
| Existing variable-expansion behavior (e.g., expanding locals/package/globals scopes) | Unchanged; client-side variable tree expansion still works | Backward compatibility: wire values are opaque to clients; they just pass them back in future requests |

**All tests pass:**
- `cargo test -p perl-dap --lib`
- `cargo test -p perl-dap --test dap_fallback_scope_variables_collision_tests`
- `cargo test -p perl-dap --test dap_var_ref_arithmetic_guard_tests`
- `cargo test --workspace --lib` (no regressions)

**No clippy warnings:** `cargo clippy -p perl-dap --lib -- -D warnings`

**Formatted:** `cargo xtask fmt`

---

## §Hazards

**Subsystem-specific defaults consulted:** [SUBSYSTEM_HAZARD_DEFAULTS.md — DAP section](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md)

| Class | Invariant | Surface (specific file/fn this change touches) | Required adversarial test |
|---|---|---|---|
| **DAP-1: ID/ref-space collision** | All numeric reference spaces (variablesReference, frameId, scope IDs, evaluate-result refs) are provably disjoint. No two allocators share an untyped integer range without a named constant boundary and a compile-time or test-time disjointness proof. | `crates/perl-dap/src/debug_adapter/parsing.rs:fallback_scope_variables()` — child ref arithmetic (lines 248, 256) | `test_fallback_scope_variables_deep_frame_child_ref_no_collision`: Encode a Scope ref with `frame_id=50_000`; verify all child refs decode as Child, not EvalResult. Assert wire values >= `2_000_000_000`. |
| **DAP-2: Bounds/overflow on client-supplied IDs** | All `frameId`, `variablesReference`, `threadId` values originating from DAP client requests are validated before use. Out-of-range → honest `ErrorResponse`, never panic or silent wrap. | `crates/perl-dap/src/debug_adapter/parsing.rs:fallback_scope_variables()` — accepts `variables_ref: i32` from `handle_variables()` | `test_fallback_scope_variables_deep_frame_child_ref_no_collision`: Pass a high `frame_id` (99_999) and confirm no overflow or panic; wire values stay in `[2_000_000_000, i32::MAX]` via saturating arithmetic. |
| **DAP-3: Protocol-safety** | Every DAP request handler tolerates unknown commands, missing required fields, empty body, and non-existent sessions. Response is honest `ErrorResponse` or empty — never crash, never fabricated data. | `crates/perl-dap/src/debug_adapter/variables.rs:handle_variables()` — calls `fallback_scope_variables()` on fallback path | `test_var_ref_arithmetic_guard_tests`: Verify guard passes; no panic on any input. (This is indirect: fallback_scope_variables is deterministic and does not read network state.) |
| **DAP-4: Running-vs-stopped state** | Requests valid only when debuggee is stopped (`variables`, `stackTrace`) return error when called while running. Handler checks session state before touching frame/variable caches. | `crates/perl-dap/src/debug_adapter/variables.rs:handle_variables()` — calls `fallback_scope_variables()` conditionally | N/A — fallback_scope_variables is a deterministic placeholder function. Session state checks are in handle_variables, not in fallback_scope_variables itself. Fallback only invoked after debugger output is parsed (offline operation). |
| **DAP-5: Stale-after-resume (refs from a previous stop are rejected)** | All variablesReferences, frameIds, scope IDs are invalidated on every `continue`/`next`/`stepIn`. Client sending a variables request with a ref from before the resume receives `ErrorResponse`, never stale data. | `crates/perl-dap/src/debug_adapter/variables.rs:handle_variables()` — calls `fallback_scope_variables()` as part of variable retrieval | N/A — fallback_scope_variables generates placeholder refs on-demand during the current stop. Invalidation is handled by handle_variables' session state check, not fallback_scope_variables. The child refs are only valid for the current stop (they are session-private and ephemeral). |
| **DAP-6: No-active-session behavior** | DAP request arriving with no active session returns `ErrorResponse`. Handler must not dereference a `None` session. | `crates/perl-dap/src/debug_adapter/variables.rs:handle_variables()` — calls `fallback_scope_variables()` after session state check | N/A — fallback_scope_variables is a pure function (no session state dependency). It is called only from handle_variables, which already validates session state. If invoked on a None session, the error occurs upstream, not in fallback_scope_variables. |
| **DAP-7: ripr-seam-anticipation** | Inline `#[cfg(test)]` helper functions or test-only code in production DAP source files will be flagged by ripr 0.5.0. Spec must pre-declare handling: relocate to tests/ (preferred) or pre-planned `#[allow]` with ripr issue citation. | N/A — no new `#[cfg(test)]` blocks added to `crates/perl-dap/src/**`. New code is production-only. Test code is relocated to `crates/perl-dap/tests/dap_fallback_scope_variables_collision_tests.rs` and `crates/perl-dap/tests/dap_var_ref_arithmetic_guard_tests.rs`. | N/A — no ripr-seam issues introduced. All test code is in designated test files. |

---

## §Contracts

| Contract | Source document + section | How this change satisfies or extends it |
|---|---|---|
| **Tagged-range-codec band-overflow invariant** | [docs/learnings/2026-06-tagged-range-codec-band-overflow.md](../learnings/2026-06-tagged-range-codec-band-overflow.md) — the #1430 incident analysis documenting the disjoint-band design | This fix applies the tagged-range-codec pattern to a previously-unmigrated call site (`fallback_scope_variables`). The placeholder child refs now use the `VariableReference::Child` codec (band `[2_000_000_000, i32::MAX]`), eliminating the collision with EvalResult band. Reinforces the contract: **only VariableReference::encode/decode produce variablesReference values**. |
| **Disjoint-band wire-safety contract** | [crates/perl-dap/src/debug_adapter/var_ref.rs](../../crates/perl-dap/src/debug_adapter/var_ref.rs) §Solution (lines 16–29) — the codec module docstring | Scope band: `[1, 999_999]`, EvalResult band: `[1_000_000, 1_999_999_999]`, Child band: `[2_000_000_000, i32::MAX]`. This fix ensures all child refs produced by fallback_scope_variables land in the Child band, satisfying the disjoint-band invariant by construction. The codec's range-based decode (lines 233–263) guarantees unambiguous classification. |
| **No raw arithmetic on variablesReference outside var_ref.rs** | Implicit contract enforced by the review of PR #1444 (see closure comment: "deliberately NOT migrated...compute_child_reference uses parent*1000+index...left unchanged"). Extended here: fallback_scope_variables also migrates away from raw arithmetic. | This fix completes the migration of all consumer sites to use VariableReference::encode/decode. The mechanical guard test (dap_var_ref_arithmetic_guard_tests) enforces this contract going forward, preventing future ad-hoc arithmetic in parsing.rs or elsewhere. |

---

## §API-Shape

| Item | Kind | Signature / Range | Dup-risk (grep result) | Caller count |
|---|---|---|---|---|
| `VariableReference::Child` variant | enum variant | `Child { parent: i32, index: u32 }` — already exists in var_ref.rs | None (stable, no naming conflict) | Previously 0 (new call site); now called from `fallback_scope_variables()` |
| `fallback_scope_variables()` function | function | `fn fallback_scope_variables(variables_ref: i32, start: usize, count: usize) -> Vec<Variable>` — **signature unchanged** | Already defined at line 234; no new overload | 2 callers (handle_variables + test suite) — **unchanged** |
| Child ref wire band | numeric range | `[2_000_000_000, i32::MAX]` — `CHILD_BASE = 2_000_000_000` | Already defined as constant in var_ref.rs line 164; no new range allocated | N/A (constant, not a caller-facing API) |
| Placeholder child index space | N/A — internal design choice | Each placeholder child assigned a stable index (0, 1, ...) in the fallback vec order. Index values fit in `u32`, clamped by `VariableReference::Child` encoder (line 213: `index & 0xFFFF`). | N/A — no public API surface | N/A |

**Summary:** No new public API surface introduced. `fallback_scope_variables()` function signature unchanged; only internal arithmetic changed from ad-hoc to codec-based. `VariableReference::Child` already exists and stable.

---

## §Test-Grid

| Scenario | Kind | Test name | Invariant discharged |
|---|---|---|---|
| Fallback vars with normal frame_id (e.g., 0 or 100) | positive | `test_fallback_scope_variables_normal_frame_child_refs` | Child refs are correctly encoded in Child band for low frame IDs; backward compatibility |
| Fallback vars with deep frame_id (50_000) | positive | `test_fallback_scope_variables_deep_frame_child_ref_no_collision` | Child refs with deep frame IDs land in Child band `[2_000_000_000, ..)`, not EvalResult band. This is the core fix invariant. |
| Fallback vars with max valid frame_id (99_999) | positive | `test_fallback_scope_variables_max_frame_id_boundary` | At frame_id=99_999, child refs still encode without overflow, land in Child band, and decode correctly. Boundary case coverage. |
| Child refs produced by fallback_scope_variables encode/decode round-trip | positive | `test_child_ref_encode_decode_roundtrip_deep_frame` | `VariableReference::Child { parent, index }.encode()` followed by `decode()` preserves identity. Wire value is in Child band. |
| Invalid scope ref (decodes to non-Scope or None) | negative | `test_fallback_scope_variables_invalid_scope_ref` | Gracefully returns empty vec (or placeholder vars for invalid scope); does not panic. Tests DAP-3 (protocol-safety). |
| Pagination with high frame_id | negative | `test_fallback_scope_variables_pagination_deep_frame` | Pagination (start > 0, count < 10) works correctly even with deep frame_id. No off-by-one errors. |
| Mechanical guard: no raw arithmetic in parsing.rs | adversarial | `test_var_ref_codec_no_raw_arithmetic_in_parsing` | Grep-based scan of parsing.rs finds NO patterns matching `%`, `/`, `*`, `1_000_000 +`, `2_000_000_000` (outside comments/tests). Enforces codec-only rule. This prevents future #1445 instances. |
| ID collision: fallback child vs EvalResult | adversarial | `test_fallback_child_ref_never_in_eval_band` | For any valid (frame_id, scope_kind, child_index) triple, the child ref wire value is NOT in `[1_000_000, 1_999_999_999]`. Assert via direct band membership test on wire values. |
| Wire value at Child band boundary (2_000_000_000) | edge case | `test_child_ref_wire_at_band_base` | Construct `Child { parent: 0, index: 0 }` and verify wire = 2_000_000_000 (CHILD_BASE). Decode returns the correct Child variant. |

---

## §Blast-Radius

**Consumers and downstream impact:**

| Consumer | Crate | Dependency type | Impact | Required update |
|---|---|---|---|---|
| `handle_variables()` | `crates/perl-dap/src/debug_adapter/variables.rs` | direct call | None — `fallback_scope_variables()` signature unchanged. Child refs are opaque wire values passed back to client; no producer-side logic change breaks callers. | None |
| `handle_scopes()` | `crates/perl-dap/src/debug_adapter.rs` or similar | transitive | None — `handle_scopes()` does not call `fallback_scope_variables()`; it calls `VariableReference::Scope { ... }.encode()`. This change is isolated to the fallback path. | None |
| Variable cache/storage logic | `crates/perl-dap/src/debug_adapter/variables.rs` | transitive | None — cache keys are based on decoded scope refs and indices; child refs are only used as wire values for the client. No cache logic depends on the arithmetic formula. | None |
| LSP crates (perl-lsp, perl-lsp-rs) | `crates/perl-lsp*` | transitive via DAP bridge | None — LSP does not directly consume `fallback_scope_variables()`. The DAP server produces wire values; the client passes them back in requests. Wire values are opaque. Only `VariableReference::decode()` is used on the DAP consumer side, which is unaffected. | None |
| Test suite: dap_scope_filtering_tests.rs | `crates/perl-dap/tests/` | direct (calls fallback_scope_variables) | Minor — snapshot or expected-output tests may need updating if they assert on specific child ref wire values. However, the test only checks that the child refs are non-zero and decodable, not the exact arithmetic. | Likely none; review if test output changes. |
| Integration tests (e.g., variable expansion end-to-end) | `crates/perl-dap/tests/` | integration | None expected — fallback_scope_variables is a deterministic function with stable inputs/outputs. Variable expansion tests operate on the full handle_variables flow, which is transparent to wire value arithmetic. | None |

**Must-not-touch boundary:**

- `crates/perl-dap/src/debug_adapter/var_ref.rs` — codec is stable and correct; no changes allowed
- `crates/perl-dap/src/debug_adapter/parsing/scope_variables.rs::compute_child_reference()` — uses `parent*1000+index` arithmetic (different contract; not in collision danger due to scope-based parent). Left unchanged per PR #1444 design decision.
- All LSP handler code — no changes (opaque wire values)
- All DAP frame/scope management logic outside parsing.rs — no changes (migration is local)

---

## §Coverage-Map

| New code path | Covered by | Test file |
|---|---|---|
| `fallback_scope_variables()` with normal frame_id and Locals kind | `test_fallback_scope_variables_normal_frame_child_refs` | `crates/perl-dap/tests/dap_fallback_scope_variables_collision_tests.rs` |
| `fallback_scope_variables()` with deep frame_id (50_000) and Locals kind | `test_fallback_scope_variables_deep_frame_child_ref_no_collision` | `crates/perl-dap/tests/dap_fallback_scope_variables_collision_tests.rs` |
| `fallback_scope_variables()` with max frame_id (99_999) and Locals kind | `test_fallback_scope_variables_max_frame_id_boundary` | `crates/perl-dap/tests/dap_fallback_scope_variables_collision_tests.rs` |
| `fallback_scope_variables()` with Package/Globals kinds | `test_fallback_scope_variables_package_and_globals_kinds` | `crates/perl-dap/tests/dap_fallback_scope_variables_collision_tests.rs` |
| `VariableReference::Child::encode()` called from fallback_scope_variables | `test_child_ref_encode_decode_roundtrip_deep_frame` | `crates/perl-dap/tests/dap_fallback_scope_variables_collision_tests.rs` |
| Mechanical guard: regex scan of parsing.rs for raw arithmetic | `test_var_ref_codec_no_raw_arithmetic_in_parsing` | `crates/perl-dap/tests/dap_var_ref_arithmetic_guard_tests.rs` |
| Collision boundary: child ref vs EvalResult band | `test_fallback_child_ref_never_in_eval_band` | `crates/perl-dap/tests/dap_fallback_scope_variables_collision_tests.rs` |

**Coverage summary:** All code paths in the modified `fallback_scope_variables()` function are covered. The mechanical guard test covers the enforcement of the codec-only rule. Boundary and collision tests cover the disjoint-band invariant.

---

## Implementation Notes for Reviewers

1. **Wire-band invariant:** After this fix, the invariant holds by construction:
   - Scope refs: `[1, 999_999]` (parent is in this band)
   - EvalResult refs: `[1_000_000, 1_999_999_999]`
   - Child refs produced by fallback_scope_variables: `[2_000_000_000, i32::MAX]` (via `VariableReference::Child::encode()`)
   - No overlap between bands.

2. **Round-trip correctness:** The `VariableReference::Child::encode()` function uses the formula `2_000_000_000 + (parent << 16 | (index & 0xFFFF))`. The decode function reverses this exactly, so `encode(Child{p,i}).and_then(decode)` recovers `Child{p,i}` with high fidelity.

3. **Backward compatibility:** Wire values are opaque to clients. The client sees a `variablesReference` integer and passes it back in future requests. The change is invisible at the protocol level; only the internal encoding changes.

4. **Mechanical guard test:** The grep-based guard prevents future ad-hoc arithmetic by ensuring parsing.rs has no patterns matching raw variablesReference operations. This is a form of static enforcement.

5. **Deep-review required:** Per PR #1444, DAP identity-semantics changes require deep-review sign-off before merge. This fix is low-risk and scoped, but the reviewer must confirm:
   - The disjoint-band invariant holds by construction.
   - Round-trip encode/decode is correct.
   - All existing variable-expansion behavior is preserved.
   - The guard test is effective at preventing future instances.
