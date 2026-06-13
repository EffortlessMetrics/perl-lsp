# Acceptance Criteria: #1432 — Migrate 6 DAP variablesReference consumers to VariableReference codec

## §Behavior

| Input / Condition | Expected Result | Test Name |
|---|---|---|
| Encode Scope with frame_id=0, kind=Locals | Wire value = 1 (matches old `0*10+1`) | `scope_encode_frame_id_0_locals` |
| Encode Scope with frame_id=99999, kind=Globals | Wire value = 999_993 (matches old `99999*10+3`) | `scope_encode_frame_id_max_globals` |
| Decode wire value 1 | Returns `Some(Scope { frame_id: 0, kind: Locals })` | `scope_decode_wire_1_locals` |
| Decode wire value 999_993 | Returns `Some(Scope { frame_id: 99_999, kind: Globals })` | `scope_decode_wire_999993_globals` |
| Encode EvalResult with counter=0 | Wire value = 1_000_000 | `evalresult_encode_counter_0` |
| Encode EvalResult with counter=999_999 | Wire value = 1_999_999 | `evalresult_encode_counter_max` |
| Decode wire value 1_000_000 | Returns `Some(EvalResult { counter: 0 })` | `evalresult_decode_wire_1000000` |
| Decode wire value 1_000_001 | Returns `Some(EvalResult { counter: 1 })` | `evalresult_decode_wire_1000001` |
| Decode wire value 0 (reserved, no children) | Returns `None` (invalid varref) | `decode_zero_invalid` |
| Decode wire value -1 (negative, invalid) | Returns `None` (invalid varref) | `decode_negative_invalid` |
| variables.rs receive variablesReference=0 | Return DAP error response, not crash | `variables_handle_invalid_varref_zero` |
| variables.rs receive variablesReference=-999 | Return DAP error response, not crash | `variables_handle_invalid_varref_negative` |
| parsing.rs parse_scope_variables receive invalid varref | Return empty Vec, not crash | `parsing_scope_variables_invalid_varref_graceful` |
| frames.rs handle_scopes for frameId=0 | Create 3 Scope refs (Locals, Package, Globals), wire values 1, 2, 3 | `frames_scopes_frame_0_basic` |
| frames.rs handle_scopes for frameId=42 | Create 3 Scope refs with wire values 421, 422, 423 | `frames_scopes_frame_42_encode` |
| evaluation.rs allocate_evaluate_result_ref with counter=0 | Return varref 1_000_000 | `evaluation_allocate_evalresult_counter_0` |
| evaluation.rs allocate_evaluate_result_ref with counter=999_999 | Return varref 1_999_999 | `evaluation_allocate_evalresult_counter_999999` |
| Round-trip: encode(Scope {...}) then decode(wire) | Decode returns exact original variant | `roundtrip_scope_variants_all` |
| Round-trip: encode(EvalResult {...}) then decode(wire) | Decode returns exact original variant | `roundtrip_evalresult_variants_all` |

## §Hazards

| Hazard Class | Surface | Severity | Mitigation |
|---|---|---|---|
| **DAP-1: Wire protocol collision** | `frames.rs:148-150`, `evaluation.rs:569`, `variables.rs:120-121` | HIGH | VariableReference codec ensures disjoint bands [1,999_999], [1_000_000,1_999_999_999], [2_000_000_000,i32::MAX]. No decode site performs arithmetic; uses exhaustive `decode(raw)` only. |
| **DAP-2: Decode returns None on invalid input** | `variables.rs:120-121`, `parsing.rs:124,228` | HIGH | All decode sites explicitly handle `None` with DAP-correct error/empty response. No unwrap/expect/panic on decode. Test: `variables_ref ∈ {0, -1, 999_999_998}` all return graceful error. |
| **DAP-3: Scope frame_id boundary validation** | `frames.rs:148-150` | MEDIUM | Frame_id ∈ [0, 99_999] is safe (encodes to [1, 999_993]). Larger values saturate but stay in Scope band. Test: frame_id=100_000 encodes correctly (saturates to 999_999). No false EvalResult/Child matches. |
| **DAP-4: Old formula backward compat** | All encode sites | MEDIUM | H4 wire-identity test: Scope/EvalResult encode output is byte-identical to old formula for canonical frame_ids. Proof: encode(Scope{0,Locals})=1, encode(Scope{99999,Globals})=999993, encode(EvalResult{0})=1_000_000, encode(EvalResult{1})=1_000_001. Client-cached varref values remain valid. |
| **DAP-5: Arithmetic saturation (i32::MAX inputs)** | `frames.rs`, `evaluation.rs` | LOW | VariableReference codec uses saturating arithmetic throughout. Extreme inputs (frame_id=i32::MAX, counter=i32::MAX) saturate to i32::MAX wire, not panic. Round-trip may fail (oversaturated values decode to None), but no crash. Test: encode(Scope{i32::MAX, ...}), encode(EvalResult{i32::MAX}) all saturate without panic. |
| **DAP-6: Scope-child ref semantic mix-up** | `parsing/scope_variables.rs:60-65` | MEDIUM | compute_child_reference (Scope-child arithmetic: parent*1000+index) is DISTINCT from VariableReference::Child (parent<<16\|index). This function is NOT a consumer of the codec and MUST NOT be migrated. Grep post-migration confirms no % 10 / / 10 / * 10 in compute_child_reference. |

## §Contracts

| Contract | File:fn | Status |
|---|---|---|
| PARSER_CONTRACTS.md: N/A — this is DAP, not parser | N/A | N/A |
| LSP_CONTRACTS.md: N/A — this is DAP, not LSP | N/A | N/A |
| DAP wire protocol (RFC §6.2 Variables): variablesReference must never be 0 for expandable types; decode must handle all i32 values gracefully | `variables.rs:handle_variables`, `evaluation.rs:allocate_evaluate_result_ref` | Enforced by codec: Scope/EvalResult/Child encode to [1,∞), decode None for 0/negative, no panic |
| Type-safe variablesReference separation (#1351 codec contract): Three disjoint wire bands, each with unambiguous decode semantics (no discriminant-based disambiguation needed) | `frames.rs`, `evaluation.rs`, `variables.rs`, `parsing.rs` | Enforced by codec: pure-range decode (Child ≥2_000_000_000, Scope [1,999_999], EvalResult [1_000_000,1_999_999_999]), not residue-based |
| Saturation safety (no panic on i32 boundary overflow) | All encode sites | Enforced by codec: all arithmetic uses saturating ops, never panics |
| Round-trip correctness (encode→decode must be identity for all valid inputs) | `crates/perl-dap/tests/var_ref.rs` | Test coverage: codec tests + integration tests in Step 9 |

## §API-Shape

| Item | Type | Location | Callers | Change |
|---|---|---|---|---|
| `VariableReference::encode(&self) -> i32` | pub fn | `crates/perl-dap/src/debug_adapter/var_ref.rs:143` | `frames.rs` (3 sites), `evaluation.rs` (1 site) | No change; already exists from #1430 |
| `VariableReference::decode(raw: i32) -> Option<Self>` | pub fn | `crates/perl-dap/src/debug_adapter/var_ref.rs:188` | `variables.rs` (1 site), `parsing.rs` (2 sites) | No change; already exists from #1430 |
| `ScopeKind` enum | pub enum | `crates/perl-dap/src/debug_adapter/var_ref.rs:66` | `frames.rs` (3 uses), `variables.rs` (3 match arms) | No change; already exists from #1430 |
| `VariableReference::Scope { frame_id: i32, kind: ScopeKind }` | enum variant | `crates/perl-dap/src/debug_adapter/var_ref.rs:112` | `frames.rs` (3 sites), `variables.rs` (1 site) | No change; already exists from #1430 |
| `VariableReference::EvalResult { counter: i32 }` | enum variant | `crates/perl-dap/src/debug_adapter/var_ref.rs:119` | `evaluation.rs` (1 site), `variables.rs` (1 site) | No change; already exists from #1430 |

**Dup-risk grep:**
```bash
grep -r "frame_id \* 10\|variables_ref % 10\|variables_ref / 10" crates/perl-dap/src/debug_adapter/
grep -r "1_000_000" crates/perl-dap/src/debug_adapter/
```
Post-migration, first grep should return zero matches (except comments/tests). Second grep should only hit the codec itself and tests.

**Caller count (post-migration):**
- `VariableReference::encode()`: 4 call sites (all in perl-dap, all migrated)
- `VariableReference::decode()`: 3 call sites (all in perl-dap, all migrated)
- No new public APIs; existing codec used exclusively

## §Test-Grid

| Class | Input/Condition | Test Name | Invariant |
|---|---|---|---|
| **Positive: Scope encoding** | frame_id=0, kind=Locals | `scope_encode_locals_frame_0` | wire = 1 |
| | frame_id=5000, kind=Locals | `scope_encode_locals_frame_5000` | wire = 50_001 (5000*10+1) |
| | frame_id=99_999, kind=Globals | `scope_encode_globals_frame_max` | wire = 999_993 (99_999*10+3) |
| **Positive: Scope decoding** | wire=1 | `scope_decode_wire_1` | frame_id=0, kind=Locals |
| | wire=50_001 | `scope_decode_wire_50001` | frame_id=5000, kind=Locals |
| | wire=999_993 | `scope_decode_wire_999993` | frame_id=99_999, kind=Globals |
| **Positive: EvalResult encoding** | counter=0 | `evalresult_encode_counter_0` | wire = 1_000_000 |
| | counter=999_999 | `evalresult_encode_counter_999999` | wire = 1_999_999 |
| **Positive: EvalResult decoding** | wire=1_000_000 | `evalresult_decode_wire_1000000` | counter=0 |
| | wire=1_999_999 | `evalresult_decode_wire_1999999` | counter=999_999 |
| **Negative: Invalid varref (zero)** | variables_ref=0 | `variables_handle_zero_invalid` | decode returns None; DAP error response, no crash |
| **Negative: Invalid varref (negative)** | variables_ref=-1 | `variables_handle_negative_invalid` | decode returns None; DAP error response, no crash |
| | variables_ref=-999 | `variables_handle_large_negative_invalid` | decode returns None; no crash |
| **Negative: Invalid Scope kind** | wire=999_994 (ends in 4, invalid kind) | `scope_decode_invalid_kind_disc` | decode returns None or falls through to EvalResult band; no match as Scope |
| **Negative: Overflow boundary** | frame_id=100_000 (> 99_999 max) | `scope_encode_frame_id_overflow` | encodes but saturates; round-trip may decode to None or stay in Scope band |
| **Adversarial: Round-trip identity** | all Scope variants {0..99_999} × {Locals,Package,Globals} | `roundtrip_scope_all` | encode(v).decode() == Some(v) for all valid v |
| | all EvalResult counters {0..999_999} | `roundtrip_evalresult_all` | encode(v).decode() == Some(v) for all valid v |
| **State transition: Scope refs across frames** | frameId sequence 0,1,2,...,10 | `frames_scopes_sequence_consistency` | Locals refs are 1,11,21,...,101; decoded back have frame_ids 0,1,2,...,10 |
| **Backward compat: Old formula identity** | frame_id ∈ [0,999], all kinds | `compat_scope_old_formula_identity` | new encode() output == old formula (frame_id*10 + kind) for all small frame_ids |
| | counter ∈ [0,999_999] | `compat_evalresult_old_formula_identity` | new encode() output == old formula (1_000_000 + counter) for all counters |
| **Integration: frames.rs → variables.rs → parsing.rs** | handle_scopes(frameId=42) → encode 3 refs → handle_variables(varRef=421) → decode to Scope{42,Locals} → parse_scope_variables(varRef=421) | `integration_frame_scopes_parse_variables` | Locals/Package/Globals refs encode, decode, and parse without loss or crash |

## §Blast-Radius

| Consumer | Files Changed | Impact | Verification |
|---|---|---|---|
| DAP server (native adapter) | `frames.rs`, `evaluation.rs`, `variables.rs`, `parsing.rs` | Core request handlers; varref encoding/decoding is critical path. Any regression breaks variable inspection. | `cargo test -p perl-dap variables_tests`, `cargo test -p perl-dap evaluate_allocation_tests`, integration tests |
| Scope-child references | `parsing/scope_variables.rs` | NOT changed; compute_child_reference is distinct from VariableReference::Child codec. | Grep confirms no % 10 / / 10 / * 10 arithmetic in this file |
| Variable cache lookup | `variable_cache.rs` | NOT changed; uses varref as HashMap key, no arithmetic. | Grep confirms no encoding/decoding in this file |
| Test suite | `crates/perl-dap/tests/var_ref.rs` | NEW integration tests for H4 wire-identity + roundtrip | All var_ref tests pass; codec unit tests remain green |
| LSP crate | None | No cross-crate varref usage; DAP is independent. | No changes to `crates/perl-lsp*` |
| Parser crate | None | No varref handling; DAP-specific. | No changes to `crates/perl-parser*` |

**Boundary enforcement:**
- Codec is private to perl-dap (`var_ref.rs` module, no re-export to other crates)
- VariableReference and ScopeKind are imported within `debug_adapter/mod.rs` scope only
- No varref logic leaks to LSP, parser, or workspace crates
- Cross-crate callers (if any) must go through DAP protocol layer (already using i32 wire values)

**Downstream crate impact:**
- **perl-lsp-rs:** No impact; LSP server does not manage DAP variablesReference (DAP is its own server)
- **perl-dap-eval, perl-dap-stack, perl-dap-variables, perl-dap-breakpoint:** No impact; these microcrates do not touch varref encoding
- **vscode-extension:** No impact; client-side cache of varref values is read-only; server assigns all varref values

**Merge conflict risk:** LOW
- 6 files are in narrow, distinct function scopes (not shared helpers)
- No recent in-flight PRs touching these files (verified: no open PRs matching `frames.rs`, `evaluation.rs`, `variables.rs`, `parsing.rs`)
- Codec is already merged (#1430); no conflicts on var_ref.rs itself
