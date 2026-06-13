# Context: #1432 — Migrate 6 DAP variablesReference consumers to VariableReference codec

## Decision Log

**Decision 1: Codec location and module structure**
- **Rationale:** PR #1430 merged VariableReference codec into `crates/perl-dap/src/debug_adapter/var_ref.rs` as a new private module within the debug_adapter subsystem.
- **Alternative considered:** Placing codec in a shared microcrate (`perl-dap-var-ref`) for potential LSP reuse. Rejected because:
  1. LSP does not manage variablesReference (that's DAP's domain)
  2. Over-engineering for a single consumer family (6 files in perl-dap)
  3. Microcrate overhead not justified for internal codec
- **Decision:** Keep codec private to perl-dap/src/debug_adapter. Re-export VariableReference and ScopeKind through mod.rs for internal use.

**Decision 2: Scope-child encoding separation**
- **Rationale:** Issue #1432 mentions "migrate parsing/scope_variables.rs" but that file's `compute_child_reference` implements Scope-child encoding (parent * 1000 + index), NOT the VariableReference::Child codec (parent << 16 | index).
- **Alternative considered:** Migrate compute_child_reference to use VariableReference::Child variant. Rejected because:
  1. Scope-children are children of Scope refs, not a separate DAP class
  2. compute_child_reference arithmetic is tuned for Scope (1000x multiplier) to avoid collision with EvalResult band [1_000_000,...)
  3. VariableReference::Child is for DAP Child refs (distinct semantics)
  4. Mixing Scope-children with VariableReference::Child breaks type safety
- **Decision:** Leave compute_child_reference unchanged. Do NOT migrate it. Verify post-migration that no % 10 / / 10 / * 10 arithmetic remains in that file.

**Decision 3: None handling strategy for decode()** (from #1430 deep-review)
- **Rationale:** VariableReference::decode() returns Option<Self> because 0 (reserved in DAP) and negative values are invalid. All decode sites must handle None gracefully.
- **Alternative considered:** Panic or return a sentinel "unknown" variant on invalid varref. Rejected because:
  1. DAP spec requires graceful handling (empty variables list, not crash)
  2. Malformed client input must not crash server (security boundary)
  3. Test grid includes adversarial cases (varref = 0, -1, 999_999_998)
- **Decision:** All decode sites in Step 4-6 MUST explicitly handle `None` with DAP-correct error or empty response. Tests prove no crash on invalid input.

**Decision 4: Backward-compatibility (H4 wire-identity)**
- **Rationale:** Clients may cache variablesReference values across debug sessions. Old formula (frame_id * 10 + kind) for Scope must produce identical wire values.
- **Alternative considered:** Accept minor wire-value shifts for codec "cleanliness." Rejected because:
  1. Cached varref invalidation breaks existing debugger workflows
  2. Wire identity is part of DAP contract (RFC §6.2)
  3. Codec was designed with disjoint bands to preserve this invariant
- **Decision:** Integration test (Step 9) MUST prove Scope and EvalResult wire values match old formula exactly for canonical frame_ids. This is non-negotiable for merge.

## Objections Addressed

### From oppositional-planner (not yet invoked, but anticipated)

**Objection 1: "Why not migrate compute_child_reference?"**
- **Response:** compute_child_reference is Scope-child-specific arithmetic (parent * 1000 + index), not a consumer of VariableReference::Child codec (parent << 16 | index). They are semantically distinct. Migrating it would break type-safety and introduce false collisions. Leaving it unchanged preserves the separation.

**Objection 2: "decode() returning None adds complexity; why not sentinel variant?"**
- **Response:** DAP spec and security boundary require graceful handling. A "sentinel" variant would obscure the error condition and potentially allow invalid refs to propagate. Explicit `None` handling forces correctness. Tests prove it's manageable.

**Objection 3: "Wire-identity backward compat is over-engineered; just bump the protocol version."**
- **Response:** Clients cache varref values; invalidating them is a user-facing UX break. Wire identity is the definition of backward compatibility in DAP. The codec was designed with disjoint bands to preserve this without extra work — we must prove it.

### From plan-reviewer (anticipated, pre-emptively addressed in spec)

**Objection: "Step 3 is wrong — encode() doesn't return Option for EvalResult."**
- **Response:** Codec tests and inspection of var_ref.rs confirm: `encode()` returns `i32`, NOT `Option`. It saturates on overflow. Only `decode()` returns `Option`. Spec clarified in Step 3 notes and in Flags for builder.

## Research Findings

### Source 1: Merged PR #1430 codec implementation
- **Claim:** VariableReference codec provides type-safe encoding/decoding with disjoint wire bands.
- **Verified:** ✓ Codec exists in `crates/perl-dap/src/debug_adapter/var_ref.rs` (lines 1-224 as of commit 0ce7cb9f8).
- **Details:**
  - Scope range: [1, 999_999] with frame_id ∈ [0, 99_999]
  - EvalResult range: [1_000_000, 1_999_999_999]
  - Child range: [2_000_000_000, i32::MAX]
  - Bands are pairwise disjoint by construction (no overlap possible)

### Source 2: Codec test suite (var_ref.rs lines 226-287)
- **Claim:** Round-trip encode/decode is identity for all valid inputs.
- **Verified:** ✓ 27 unit tests cover:
  - Basic encode/decode for all 3 variants
  - Kind discriminant validation
  - Zero and negative rejection
  - Overflow saturation without panic
- **Coverage gap:** Integration tests with real DAP flow (frames → variables → parsing) not yet present. Planned for Step 9.

### Source 3: Current consumer arithmetic (verified by grep)
- **Claim:** 6 consumer files use raw % 10 / / 10 / * 10 or 1_000_000 base arithmetic.
- **Verified:** ✓ Found exact call sites:
  - frames.rs:148-150 — 3 × (frame_id * 10 + kind)
  - evaluation.rs:569 — 1_000_000 + counter
  - variables.rs:120-121 — frame_id = variables_ref / 10; match variables_ref % 10
  - parsing.rs:124 — scope_type = variables_ref % 10
  - parsing.rs:228 — match variables_ref % 10
  - parsing/scope_variables.rs:60-65 — parent * 1000 + index (NOT migrated)

### Source 4: DAP specification (RFC §6.2 Variables request)
- **Claim:** variablesReference is a single i32 encoding multiple logical reference types.
- **Verified:** ✓ RFC defines variablesReference as i32, 0 = "no children" (reserved).
- **Implication:** Codec must preserve 0 as invalid (decode → None).

### Source 5: Issue #1219 collision class (root cause analysis)
- **Claim:** Old formula (frame_id * 10 + kind) collides with EvalResult band [1_000_000, ...).
- **Verified:** ✓ Example: EvalResult{counter: 1} → wire 1_000_001 misclassified as Scope{frame_id: 100_000, kind: Locals} in old decode (100_000 * 10 + 1 = 1_000_001).
- **Implication:** New codec with disjoint bands eliminates this collision. Decode must be range-first (Child ≥ 2_000_000_000, Scope [1,999_999], EvalResult ≥ 1_000_000), not residue-first.

## Related Issues

- **#1351:** Original spec for VariableReference codec (merged in PR #1430). This issue (#1432) is the direct follow-up, migrating consumers to use the codec.
- **#1430:** Merged PR introducing the codec. Provides:
  - Type definition: VariableReference enum, ScopeKind enum
  - encode() and decode() methods
  - Unit tests for round-trip and adversarial cases
  - Spec document for the codec design
- **#1219:** Original collision-class issue that motivated the codec design. Identified the frame_id * 10 + kind collision with EvalResult band. Retired at type level once #1432 consumers use the codec.
- **#1433:** Parallel learning issue (merged). Documents codec band-overflow hazards from deep-review. Key learnings feed into §Hazards (DAP-1 through DAP-6).

## Upstream and Related Work

**Prerequisite:** PR #1430 must be merged before this issue can be built. ✓ Confirmed merged (commit 0ce7cb9f8 on origin/main).

**Parallel work:** No known in-flight PRs touching the 6 consumer files. Low merge-conflict risk.

**Follow-up opportunities (future issues):**
1. Extend VariableReference to support Child references explicitly (currently compute_child_reference is Scope-specific)
2. Add metric-based monitoring of varref allocation patterns (detect pathological expansion)
3. Formalize DAP type-safety invariants in codec documentation

## Implementation Notes for Builder

1. **var_ref module visibility:** Check if `var_ref` is re-exported in `debug_adapter/mod.rs`. If not, add:
   ```rust
   pub use self::var_ref::{VariableReference, ScopeKind};
   ```
   This makes the codec types available to sibling files (frames.rs, evaluation.rs, etc.).

2. **Encode return type:** Codec's `encode()` returns `i32` (saturates at i32::MAX). For Scope refs with small frame_ids, this is always in-band and safe. For EvalResult refs, saturating at i32::MAX is acceptable (counter overflow is a recovery point, not a hard error).

3. **Decode must match exhaustively:** All match statements on `VariableReference::decode()` must handle all 4 cases (Scope, EvalResult, Child, None). Use `Some(VariableReference::Scope { ... })` syntax, not wildcards.

4. **Scope-child ref gotcha:** compute_child_reference (parent * 1000 + index) is NOT the same as VariableReference::Child (parent << 16 | index). This function must NOT be migrated. If touched, verify it still uses the original arithmetic.

5. **H4 wire-identity proof:** Integration test must demonstrate:
   - encode(Scope{0, Locals}) == old formula(0, 1) == 1
   - encode(Scope{99_999, Globals}) == old formula(99_999, 3) == 999_993
   - encode(EvalResult{0}) == old formula(0) == 1_000_000
   - encode(EvalResult{1}) == old formula(1) == 1_000_001
   - For all these wire values, decode() recovers the original variant exactly.

6. **Test for graceful None handling:** Adversarial tests must verify:
   - variables.rs receives variablesReference=0 → returns error response (not crash)
   - variables.rs receives variablesReference=-1 → returns error response (not crash)
   - parsing.rs receives invalid varref → returns empty Vec (not crash or panic)
   - No unwrap/expect/panic on decode() anywhere in consumers

7. **Final audit:** Post-migration, run:
   ```bash
   grep -r "% 10\|/ 10\|\* 10" crates/perl-dap/src/debug_adapter/*.rs
   ```
   Should return ZERO matches (except var_ref.rs codec itself and comments).
