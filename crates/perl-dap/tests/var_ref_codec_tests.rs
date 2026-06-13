//! H1-H5 hazard-class codec tests + adversarial round-trip tests for VariableReference.
//!
//! # Hazard classes tested
//!
//! - **H1 (ID/ref-space collision, #1219 repro):** Scope and EvalResult bands are
//!   pairwise disjoint — no wire value can be both a Scope and an EvalResult.
//! - **H2 (Protocol-safe round-trip):** `decode(encode(v)) == Some(v)` for all variants.
//! - **H3 (Bounds/overflow safety):** Extreme inputs (i32::MAX, u32::MAX) handled by
//!   saturating arithmetic — no panic.
//! - **H5 (Exhaustive decode):** Invalid wire values decode to `None`, never panic.
//!
//! # Disjoint-band invariant
//!
//! ```text
//! Scope:      [1, 999_999]              (frame_id ∈ [0, 99_999], kind ∈ [1,3])
//! EvalResult: [1_000_000, 1_999_999_999]
//! Child:      [2_000_000_000, i32::MAX]
//! ```
//!
//! Decode is pure-range — no residue disambiguation is needed or used.

use perl_dap::var_ref::{ScopeKind, VariableReference};

// ============================================================================
// H1: No-collision (ID/ref-space collision, #1219 repro)
// ============================================================================

/// H1: Scope and EvalResult must have distinct wire values.
/// This is the #1219 collision test: frame_id=5000, kind=Locals encodes to 50_001,
/// which must be distinct from any EvalResult value.
#[test]
fn test_h1_no_collision_scope_vs_evalresult() -> Result<(), Box<dyn std::error::Error>> {
    let scope = VariableReference::Scope { frame_id: 5000, kind: ScopeKind::Locals };
    let eval = VariableReference::EvalResult { counter: 1 };

    let scope_wire = scope.encode().ok_or("H1: Scope(5000, Locals) encode must succeed")?;
    let eval_wire = eval.encode().ok_or("H1: EvalResult(1) encode must succeed")?;

    assert_ne!(
        scope_wire, eval_wire,
        "H1: Scope and EvalResult must have different wire values; got scope={scope_wire}, eval={eval_wire}"
    );
    assert_eq!(
        scope_wire, 50_001,
        "H1: Scope{{frame_id: 5000, kind: Locals}} should encode as 50_001"
    );
    assert_eq!(eval_wire, 1_000_001, "H1: EvalResult{{counter: 1}} should encode as 1_000_001");

    // Critical test: decode(50_001) must return Scope, NOT EvalResult
    let decoded = VariableReference::decode(50_001).ok_or("H1: decode(50_001) should succeed")?;
    match decoded {
        VariableReference::Scope { frame_id, kind } => {
            assert_eq!(frame_id, 5000, "H1: decoded Scope frame_id should be 5000, got {frame_id}");
            assert_eq!(
                kind,
                ScopeKind::Locals,
                "H1: decoded Scope kind should be Locals, got {kind:?}"
            );
        }
        _ => {
            panic!(
                "H1 VIOLATION: decode(50_001) returned {decoded:?}, expected Scope(5000, Locals)"
            );
        }
    }

    // Also verify decode(1_000_001) returns EvalResult, not Scope — the round-trip
    // that was broken in the original implementation.
    let decoded_eval =
        VariableReference::decode(1_000_001).ok_or("H1: decode(1_000_001) should succeed")?;
    match decoded_eval {
        VariableReference::EvalResult { counter } => {
            assert_eq!(
                counter, 1,
                "H1: decode(1_000_001) must be EvalResult{{counter: 1}}, got counter={counter}"
            );
        }
        _ => {
            panic!(
                "H1 VIOLATION: decode(1_000_001) returned {decoded_eval:?}, expected EvalResult{{counter: 1}}"
            );
        }
    }

    Ok(())
}

// ============================================================================
// H2: Wire round-trip (encode/decode preserves value and type)
// ============================================================================

/// H2: Scope{frame_id: 5000, kind: Locals} round-trips correctly.
#[test]
fn test_h2_roundtrip_scope_locals_5000() -> Result<(), Box<dyn std::error::Error>> {
    let original = VariableReference::Scope { frame_id: 5000, kind: ScopeKind::Locals };
    let wire = original.encode().ok_or("H2: Scope(5000, Locals) encode must succeed")?;
    let decoded =
        VariableReference::decode(wire).ok_or("H2: decode should succeed for valid Scope wire")?;

    assert_eq!(
        original, decoded,
        "H2: round-trip failed for Scope{{frame_id: 5000, kind: Locals}}; decoded={decoded:?}"
    );

    // Verify re-encoding matches
    let rewired = decoded.encode().ok_or("H2: re-encode of decoded Scope must succeed")?;
    assert_eq!(
        wire, rewired,
        "H2: encode(decode(w)) should equal w; wire={wire}, rewired={rewired}"
    );

    Ok(())
}

/// H2: Scope{frame_id: 0, kind: Globals} round-trips.
#[test]
fn test_h2_roundtrip_scope_globals() -> Result<(), Box<dyn std::error::Error>> {
    let original = VariableReference::Scope { frame_id: 0, kind: ScopeKind::Globals };
    let wire = original.encode().ok_or("H2: Scope(0, Globals) encode must succeed")?;
    let decoded =
        VariableReference::decode(wire).ok_or("H2: decode should succeed for valid Scope wire")?;

    assert_eq!(original, decoded, "H2: round-trip failed for Scope{{frame_id: 0, kind: Globals}}");

    let rewired = decoded.encode().ok_or("H2: re-encode must succeed")?;
    assert_eq!(wire, rewired, "H2: encode(decode(w)) == w for Scope Globals");

    Ok(())
}

/// H2: Scope{frame_id: 99_999, kind: Package} round-trips (true max frame_id under disjoint bands).
///
/// Under the disjoint-band design, max frame_id is 99_999 (wire = 999_992).
/// frame_id=999_999 would overflow into the EvalResult band and is rejected by encode.
#[test]
fn test_h2_roundtrip_scope_package_max() -> Result<(), Box<dyn std::error::Error>> {
    let original = VariableReference::Scope { frame_id: 99_999, kind: ScopeKind::Package };
    let wire = original.encode().ok_or("H2: Scope(99_999, Package) encode must succeed")?;
    assert_eq!(
        wire, 999_992,
        "H2: Scope{{frame_id: 99_999, kind: Package}} wire should be 999_992"
    );

    let decoded = VariableReference::decode(wire)
        .ok_or("H2: decode should succeed for max Scope frame_id")?;

    assert_eq!(
        original, decoded,
        "H2: round-trip failed for Scope{{frame_id: 99_999, kind: Package}}"
    );

    let rewired = decoded.encode().ok_or("H2: re-encode must succeed")?;
    assert_eq!(wire, rewired, "H2: encode(decode(w)) == w for max Scope");

    Ok(())
}

/// H2: EvalResult{counter: 1_000_000} round-trips (large counter).
#[test]
fn test_h2_roundtrip_evalresult_counter_large() -> Result<(), Box<dyn std::error::Error>> {
    let original = VariableReference::EvalResult { counter: 1_000_000 };
    let wire = original.encode().ok_or("H2: EvalResult encode must succeed")?;
    let decoded =
        VariableReference::decode(wire).ok_or("H2: decode should succeed for EvalResult")?;

    assert_eq!(original, decoded, "H2: round-trip failed for EvalResult{{counter: 1_000_000}}");

    let rewired = decoded.encode().ok_or("H2: re-encode must succeed")?;
    assert_eq!(wire, rewired, "H2: encode(decode(w)) == w for EvalResult large counter");

    Ok(())
}

/// H2: EvalResult{counter: 0} round-trips (base case).
#[test]
fn test_h2_roundtrip_evalresult_counter_zero() -> Result<(), Box<dyn std::error::Error>> {
    let original = VariableReference::EvalResult { counter: 0 };
    let wire = original.encode().ok_or("H2: EvalResult(0) encode must succeed")?;
    let decoded = VariableReference::decode(wire)
        .ok_or("H2: decode should succeed for EvalResult counter=0")?;

    assert_eq!(original, decoded, "H2: round-trip failed for EvalResult{{counter: 0}}");

    let rewired = decoded.encode().ok_or("H2: re-encode must succeed")?;
    assert_eq!(wire, rewired, "H2: encode(decode(w)) == w for EvalResult zero");

    Ok(())
}

/// H2: Child{parent: 1000, index: 50} round-trips.
#[test]
fn test_h2_roundtrip_child_basic() -> Result<(), Box<dyn std::error::Error>> {
    let original = VariableReference::Child { parent: 1000, index: 50 };
    let wire = original.encode().ok_or("H2: Child encode must succeed")?;
    let decoded = VariableReference::decode(wire).ok_or("H2: decode should succeed for Child")?;

    assert_eq!(original, decoded, "H2: round-trip failed for Child{{parent: 1000, index: 50}}");

    let rewired = decoded.encode().ok_or("H2: re-encode must succeed")?;
    assert_eq!(wire, rewired, "H2: encode(decode(w)) == w for Child");

    Ok(())
}

/// H2: Child at base (parent: 0, index: 0) round-trips.
#[test]
fn test_h2_roundtrip_child_base() -> Result<(), Box<dyn std::error::Error>> {
    let original = VariableReference::Child { parent: 0, index: 0 };
    let wire = original.encode().ok_or("H2: Child(0,0) encode must succeed")?;
    assert_eq!(
        wire, 2_000_000_000,
        "H2: Child{{parent: 0, index: 0}} should encode to base 2_000_000_000"
    );

    let decoded =
        VariableReference::decode(wire).ok_or("H2: decode should succeed for Child at base")?;

    assert_eq!(original, decoded, "H2: round-trip failed for Child{{parent: 0, index: 0}}");

    Ok(())
}

// ============================================================================
// H3: Bounds/overflow safety (no panic on extreme inputs)
// ============================================================================

/// H3: Encode with i32::MAX in frame_id does not panic (out-of-range → None, no panic).
#[test]
fn test_h3_encode_frame_id_i32_max_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    let var = VariableReference::Scope { frame_id: i32::MAX, kind: ScopeKind::Locals };
    // Must not panic; frame_id > 99_999 → None
    let _wire = var.encode();
    Ok(())
}

/// H3: Encode with i32::MAX in counter does not panic.
#[test]
fn test_h3_encode_counter_i32_max_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    let var = VariableReference::EvalResult { counter: i32::MAX };
    // Must not panic; should saturate
    let _wire = var.encode();
    Ok(())
}

/// H3: Decode(i32::MAX) does not panic; returns Some or None.
#[test]
fn test_h3_decode_i32_max_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    // Must not panic; should return Some(Child variant) or None
    let result = VariableReference::decode(i32::MAX);
    // Result can be Some or None, just must not panic
    let _ = result;
    Ok(())
}

/// H3: Decode(i32::MIN) does not panic.
#[test]
fn test_h3_decode_i32_min_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    // Must not panic
    let result = VariableReference::decode(i32::MIN);
    let _ = result;
    Ok(())
}

/// H3: Child with index: u32::MAX does not panic (bit-packing).
#[test]
fn test_h3_child_index_u32_max_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    let var = VariableReference::Child { parent: 1000, index: u32::MAX };
    // Must not panic; bit-packing should saturate or overflow safely
    let _wire = var.encode();
    Ok(())
}

// ============================================================================
// H5: Exhaustive decode (invalid ranges return None, not panic)
// ============================================================================

/// H5: Decode(0) returns None (out of range, no variant maps to 0).
#[test]
fn test_h5_decode_zero_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let result = VariableReference::decode(0);
    assert_eq!(
        result, None,
        "H5: decode(0) should return None (out of Scope [1..999_999], EvalResult [1_000_000..], Child [2_000_000_000..] ranges)"
    );
    Ok(())
}

/// H5: Decode(-1) returns None (negative, out of range).
#[test]
fn test_h5_decode_negative_one_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let result = VariableReference::decode(-1);
    assert_eq!(result, None, "H5: decode(-1) should return None");
    Ok(())
}

/// H5: Decode(i32::MIN) returns None.
#[test]
fn test_h5_decode_i32_min_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let result = VariableReference::decode(i32::MIN);
    // Child base is 2_000_000_000 (positive), so i32::MIN is definitely None
    assert_eq!(result, None, "H5: decode(i32::MIN) should return None");
    Ok(())
}

/// H5: Decode(999_999) is None (kind_disc=9 is invalid).
///
/// Wire 999_999 is in the Scope band [1, 999_999] but has kind_disc = 9 (invalid;
/// must be 1-3). Confirms the Scope band's upper edge is correctly rejected.
#[test]
fn test_h5_decode_gap_999999_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let result = VariableReference::decode(999_999);
    assert_eq!(
        result, None,
        "H5: decode(999_999) should return None (frame_id=99_999, kind_disc=9 is invalid; must be 1-3)"
    );
    Ok(())
}

/// H5: Decode at Scope boundary (900_001): frame_id=90_000, kind_disc=1 (valid Locals).
#[test]
fn test_h5_decode_scope_boundary_valid() -> Result<(), Box<dyn std::error::Error>> {
    let result = VariableReference::decode(900_001);
    assert!(
        result.is_some(),
        "H5: decode(900_001) should be Some (frame_id=90_000, kind_disc=1 is valid)"
    );
    match result.unwrap() {
        VariableReference::Scope { frame_id, kind } => {
            assert_eq!(frame_id, 90_000, "H5: frame_id should be 90_000");
            assert_eq!(kind, ScopeKind::Locals, "H5: kind should be Locals (disc=1)");
        }
        _ => panic!("H5: decode(900_001) should return Scope, not {result:?}"),
    }
    Ok(())
}

/// H5: Decode with invalid scope kind discriminant (kind_disc=4).
/// Wire 900_004: frame_id=90_000, kind_disc=4 (invalid). Should return None.
#[test]
fn test_h5_decode_invalid_scope_kind_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let wire = 900_004; // frame_id=90_000, kind_disc=4
    let result = VariableReference::decode(wire);
    assert_eq!(
        result, None,
        "H5: decode(900_004) should return None (kind_disc=4 is invalid for Scope)"
    );
    Ok(())
}

/// H5: Decode at EvalResult base edge (1_000_000) should return EvalResult counter=0.
#[test]
fn test_h5_decode_evalresult_base_valid() -> Result<(), Box<dyn std::error::Error>> {
    let result = VariableReference::decode(1_000_000);
    assert!(result.is_some(), "H5: decode(1_000_000) should be Some (EvalResult base)");
    match result.unwrap() {
        VariableReference::EvalResult { counter } => {
            assert_eq!(counter, 0, "H5: EvalResult at base should have counter=0");
        }
        _ => panic!("H5: decode(1_000_000) should return EvalResult, not {result:?}"),
    }
    Ok(())
}

/// H5: Decode at Child base edge (2_000_000_000) should return Child{parent: 0, index: 0}.
#[test]
fn test_h5_decode_child_base_valid() -> Result<(), Box<dyn std::error::Error>> {
    let result = VariableReference::decode(2_000_000_000);
    assert!(result.is_some(), "H5: decode(2_000_000_000) should be Some (Child base)");
    match result.unwrap() {
        VariableReference::Child { parent, index } => {
            assert_eq!(parent, 0, "H5: Child at base should have parent=0");
            assert_eq!(index, 0, "H5: Child at base should have index=0");
        }
        _ => panic!("H5: decode(2_000_000_000) should return Child, not {result:?}"),
    }
    Ok(())
}

// ============================================================================
// ScopeKind::TryFrom<i32> tests (bonus: ensures ScopeKind validity)
// ============================================================================

/// Verify ScopeKind::try_from accepts valid discriminants (1, 2, 3).
#[test]
fn test_scope_kind_try_from_valid_locals() -> Result<(), Box<dyn std::error::Error>> {
    let kind = ScopeKind::try_from(1)?;
    assert_eq!(kind, ScopeKind::Locals, "ScopeKind::try_from(1) should be Locals");
    Ok(())
}

#[test]
fn test_scope_kind_try_from_valid_package() -> Result<(), Box<dyn std::error::Error>> {
    let kind = ScopeKind::try_from(2)?;
    assert_eq!(kind, ScopeKind::Package, "ScopeKind::try_from(2) should be Package");
    Ok(())
}

#[test]
fn test_scope_kind_try_from_valid_globals() -> Result<(), Box<dyn std::error::Error>> {
    let kind = ScopeKind::try_from(3)?;
    assert_eq!(kind, ScopeKind::Globals, "ScopeKind::try_from(3) should be Globals");
    Ok(())
}

/// Verify ScopeKind::try_from rejects invalid discriminants (0, 4, -1).
#[test]
fn test_scope_kind_try_from_invalid_zero() -> Result<(), Box<dyn std::error::Error>> {
    let result = ScopeKind::try_from(0);
    assert!(result.is_err(), "ScopeKind::try_from(0) should be Err");
    Ok(())
}

#[test]
fn test_scope_kind_try_from_invalid_four() -> Result<(), Box<dyn std::error::Error>> {
    let result = ScopeKind::try_from(4);
    assert!(result.is_err(), "ScopeKind::try_from(4) should be Err");
    Ok(())
}

#[test]
fn test_scope_kind_try_from_invalid_negative() -> Result<(), Box<dyn std::error::Error>> {
    let result = ScopeKind::try_from(-1);
    assert!(result.is_err(), "ScopeKind::try_from(-1) should be Err");
    Ok(())
}

// ============================================================================
// Adversarial round-trip tests — EvalResult counters that previously collided
// ============================================================================

/// EvalResult round-trip for counters 0-9 (all residues 0-9 covered).
///
/// Under the old %10 disambiguation, counters 1, 2, 3 produced wires ending in 1/2/3
/// which were misclassified as Scope. These must now all decode as EvalResult.
#[test]
fn test_adversarial_evalresult_counters_0_to_9() -> Result<(), Box<dyn std::error::Error>> {
    for counter in 0..=9_i32 {
        let original = VariableReference::EvalResult { counter };
        let wire = original.encode().ok_or(format!("encode EvalResult({counter}) must succeed"))?;
        let decoded = VariableReference::decode(wire)
            .ok_or(format!("decode(encode(EvalResult({counter}))) must succeed"))?;
        assert_eq!(
            original, decoded,
            "adversarial: EvalResult{{counter: {counter}}} round-trip failed; wire={wire}, decoded={decoded:?}"
        );
    }
    Ok(())
}

/// EvalResult round-trip for additional critical counters (1001, 5001).
#[test]
fn test_adversarial_evalresult_counters_critical() -> Result<(), Box<dyn std::error::Error>> {
    for counter in [10_i32, 1001, 5001, 9999, 999_991, 999_992, 999_993] {
        let original = VariableReference::EvalResult { counter };
        let wire = original.encode().ok_or(format!("encode EvalResult({counter}) must succeed"))?;
        let decoded = VariableReference::decode(wire)
            .ok_or(format!("decode(encode(EvalResult({counter}))) must succeed"))?;
        assert_eq!(
            original, decoded,
            "adversarial: EvalResult{{counter: {counter}}} round-trip failed; wire={wire}, decoded={decoded:?}"
        );
    }
    Ok(())
}

/// Exhaustive EvalResult round-trip sweep: counter 0 to 5000.
///
/// Verifies that every counter in [0, 5000] round-trips correctly through the
/// disjoint-band codec, including all residues 0-9.
#[test]
fn test_adversarial_evalresult_sweep_0_to_5000() -> Result<(), Box<dyn std::error::Error>> {
    for counter in 0..=5000_i32 {
        let original = VariableReference::EvalResult { counter };
        let wire = original.encode().ok_or(format!("encode EvalResult({counter}) must succeed"))?;
        let decoded = VariableReference::decode(wire).ok_or(format!(
            "decode(encode(EvalResult({counter}))) = decode({wire}) must succeed"
        ))?;
        assert_eq!(
            original, decoded,
            "sweep: EvalResult{{counter: {counter}}} failed round-trip; wire={wire}"
        );
    }
    Ok(())
}

/// Sparse EvalResult round-trip sweep across upper EvalResult band.
///
/// Samples ~190 points across [5001, 999_999_000] to confirm the upper band
/// is also correct (step ~5_263_157).
#[test]
fn test_adversarial_evalresult_sparse_upper_sweep() -> Result<(), Box<dyn std::error::Error>> {
    let step = 5_263_157_i32;
    let mut counter = 5001_i32;
    while counter < 999_999_000 {
        let original = VariableReference::EvalResult { counter };
        let wire = original.encode().ok_or(format!("encode EvalResult({counter}) must succeed"))?;
        let decoded = VariableReference::decode(wire).ok_or(format!(
            "decode(encode(EvalResult({counter}))) = decode({wire}) must succeed"
        ))?;
        assert_eq!(
            original, decoded,
            "sparse sweep: EvalResult{{counter: {counter}}} failed round-trip; wire={wire}"
        );
        counter = counter.saturating_add(step);
    }
    Ok(())
}

/// Scope max boundary: frame_id=99_999 (true max) round-trips for all three kinds.
#[test]
fn test_adversarial_scope_max_frame_id_all_kinds() -> Result<(), Box<dyn std::error::Error>> {
    let frame_id = 99_999_i32;
    for kind in [ScopeKind::Locals, ScopeKind::Package, ScopeKind::Globals] {
        let original = VariableReference::Scope { frame_id, kind };
        let wire =
            original.encode().ok_or(format!("encode Scope({frame_id}, {kind:?}) must succeed"))?;
        // Confirm wire is within Scope band [1, 999_999]
        assert!(wire >= 1 && wire <= 999_999, "Scope max wire {wire} must be in [1, 999_999]");
        let decoded = VariableReference::decode(wire)
            .ok_or(format!("decode({wire}) for Scope({frame_id}, {kind:?}) must succeed"))?;
        assert_eq!(
            original, decoded,
            "Scope max frame_id={frame_id}, kind={kind:?} round-trip failed"
        );
    }
    Ok(())
}

/// frame_id=100_000 encode must return None (would overflow into EvalResult band).
#[test]
fn test_adversarial_scope_frame_id_100000_encode_none() -> Result<(), Box<dyn std::error::Error>> {
    let over = VariableReference::Scope { frame_id: 100_000, kind: ScopeKind::Locals };
    assert_eq!(
        over.encode(),
        None,
        "Scope{{frame_id: 100_000}} encode must return None (wire 1_000_001 is in EvalResult band)"
    );
    Ok(())
}

/// Cross-variant non-collision: no Scope wire equals any EvalResult wire.
///
/// Since Scope ⊂ [1, 999_999] and EvalResult ⊂ [1_000_000, ...], the bands are
/// disjoint by construction. This test verifies the boundary directly.
#[test]
fn test_adversarial_scope_evalresult_no_overlap() -> Result<(), Box<dyn std::error::Error>> {
    // Max possible Scope wire: 99_999 * 10 + 3 = 999_993
    let max_scope_wire = 99_999_i32 * 10 + 3;
    assert_eq!(max_scope_wire, 999_993);
    // Min EvalResult wire: 1_000_000
    let min_eval_wire = 1_000_000_i32;
    assert!(
        max_scope_wire < min_eval_wire,
        "Scope max wire {max_scope_wire} must be strictly less than EvalResult min wire {min_eval_wire}"
    );

    // Decode the boundary values to confirm they land in the right variants
    // 999_993 = 99_999 * 10 + 3 → frame_id=99_999, kind_disc=3 → Globals
    assert_eq!(
        VariableReference::decode(999_993),
        Some(VariableReference::Scope { frame_id: 99_999, kind: ScopeKind::Globals }),
        "999_993 must decode as Scope(99_999, Globals) (disc=3)"
    );
    assert_eq!(
        VariableReference::decode(1_000_000),
        Some(VariableReference::EvalResult { counter: 0 }),
        "1_000_000 must decode as EvalResult(0)"
    );

    Ok(())
}

/// Decode gap between Scope and EvalResult bands: [1_000_000, 999_999] is empty by design.
/// Values in (999_999, 1_000_000) would be gap values — but there are none (gap is zero-width).
/// This confirms the bands are adjacent (no numeric gap, no overlap).
#[test]
fn test_adversarial_scope_evalresult_band_adjacency() -> Result<(), Box<dyn std::error::Error>> {
    // The value just above Scope max is the EvalResult base:
    // 999_999 is in Scope band (kind_disc=9 → None), 1_000_000 is EvalResult base.
    // There is no integer in (999_999, 1_000_000) — the bands are adjacent.
    assert_eq!(VariableReference::decode(999_999), None, "999_999: invalid kind_disc=9 → None");
    assert_eq!(
        VariableReference::decode(1_000_000),
        Some(VariableReference::EvalResult { counter: 0 }),
        "1_000_000 is EvalResult base"
    );
    Ok(())
}

/// EvalResult upper-bound: encode rejects counter values that would push the wire into the Child band.
///
/// The EvalResult band is [1_000_000, 1_999_999_999]. Any counter >= 1_999_000_000 causes
/// wire = 1_000_000 + counter >= 2_000_000_000 = CHILD_BASE, which would be misclassified as
/// Child on decode. The encode() contract rejects these counters with None.
#[test]
fn test_adversarial_evalresult_counter_upper_bound_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    // First counter that overflows into Child zone: CHILD_BASE - EVAL_BASE = 1_999_000_000
    let overflow_counter = 1_999_000_000_i32;
    let result = VariableReference::EvalResult { counter: overflow_counter }.encode();
    assert_eq!(
        result,
        None,
        "EvalResult{{counter: {overflow_counter}}} encode must return None \
         (wire would be {}, which equals CHILD_BASE and decodes as Child)",
        1_000_000_i64 + overflow_counter as i64
    );

    // i32::MAX counter also returns None
    let result_max = VariableReference::EvalResult { counter: i32::MAX }.encode();
    assert_eq!(
        result_max, None,
        "EvalResult{{counter: i32::MAX}} encode must return None (wire overflows EVAL_MAX)"
    );

    Ok(())
}

/// EvalResult max valid counter round-trips correctly.
///
/// The last counter that stays within the EvalResult band:
/// EVAL_MAX - EVAL_BASE = 1_999_999_999 - 1_000_000 = 1_998_999_999.
#[test]
fn test_adversarial_evalresult_counter_max_valid_roundtrip()
-> Result<(), Box<dyn std::error::Error>> {
    let max_valid_counter = 1_998_999_999_i32;
    let original = VariableReference::EvalResult { counter: max_valid_counter };
    let wire = original
        .encode()
        .ok_or(format!("EvalResult{{counter: {max_valid_counter}}} encode must succeed"))?;
    assert_eq!(wire, 1_999_999_999, "wire at max valid counter must equal EVAL_MAX");
    let decoded =
        VariableReference::decode(wire).ok_or(format!("decode(EVAL_MAX = {wire}) must succeed"))?;
    assert_eq!(
        original, decoded,
        "EvalResult max valid counter must round-trip: counter={max_valid_counter}"
    );
    Ok(())
}

// ============================================================================
// Adversarial round-trip tests — Child negative parent (band-bleed hazard)
// ============================================================================

/// Child encode with negative parent must return None.
///
/// A negative `parent` in Child encoding produces a wire value in the EvalResult
/// band [1_000_000, 1_999_999_999], violating the disjoint-band invariant and
/// causing round-trip failures (e.g. parent=-1, index=0 → wire 1_999_934_464,
/// which decodes as EvalResult{counter: 998_934_464}, not as Child).
///
/// Valid DAP variablesReferences are always non-negative, so encode() must
/// reject negative parents.
#[test]
fn test_adversarial_child_negative_parent_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // parent=-1 would encode to wire 1_999_934_464 (EvalResult band) — must be None
    let result = VariableReference::Child { parent: -1, index: 0 }.encode();
    assert_eq!(
        result, None,
        "Child{{parent: -1, index: 0}} encode must return None \
         (wire 1_999_934_464 would bleed into EvalResult band)"
    );

    // parent=i32::MIN also rejected
    let result_min = VariableReference::Child { parent: i32::MIN, index: 0 }.encode();
    assert_eq!(
        result_min, None,
        "Child{{parent: i32::MIN, index: 0}} encode must return None (negative parent)"
    );

    // parent=-1 with max index also rejected
    let result_max_idx = VariableReference::Child { parent: -1, index: u32::MAX }.encode();
    assert_eq!(
        result_max_idx, None,
        "Child{{parent: -1, index: u32::MAX}} encode must return None (negative parent)"
    );

    Ok(())
}

/// Child encode with parent=0 is valid (minimum non-negative parent).
///
/// Regression guard: the negative-parent fix must not accidentally reject parent=0,
/// which is a legitimate variablesReference value (encodes to CHILD_BASE).
#[test]
fn test_adversarial_child_parent_zero_valid() -> Result<(), Box<dyn std::error::Error>> {
    let original = VariableReference::Child { parent: 0, index: 5 };
    let wire = original.encode().ok_or("Child{{parent: 0, index: 5}} encode must succeed")?;
    // wire = CHILD_BASE + (0 * 65_536) + 5 = 2_000_000_000 + 5 = 2_000_000_005
    assert_eq!(wire, 2_000_000_005, "Child{{parent: 0, index: 5}} should encode as 2_000_000_005");
    let decoded =
        VariableReference::decode(wire).ok_or("decode(2_000_000_005) must succeed for Child")?;
    assert_eq!(original, decoded, "Child{{parent: 0, index: 5}} must round-trip");
    Ok(())
}

/// EvalResult negative counter is rejected by encode().
///
/// Counters are logically non-negative (monotonically increasing allocation IDs).
/// Negative counters must not produce any wire value.
#[test]
fn test_adversarial_evalresult_negative_counter_rejected() -> Result<(), Box<dyn std::error::Error>>
{
    let result = VariableReference::EvalResult { counter: -1 }.encode();
    assert_eq!(
        result, None,
        "EvalResult{{counter: -1}} encode must return None (negative counter is invalid)"
    );
    let result_min = VariableReference::EvalResult { counter: i32::MIN }.encode();
    assert_eq!(result_min, None, "EvalResult{{counter: i32::MIN}} encode must return None");
    Ok(())
}
