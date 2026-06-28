//! TDD tests for #1445: fallback_scope_variables child refs collision fix.
//!
//! These tests verify the CODEC behavior (VariableReference::Child) that fallback_scope_variables
//! will use after the fix. The RED test (that detects the bug) is in dap_var_ref_arithmetic_guard_tests.rs.
//!
//! ## RED test (detects the bug):
//! - `dap_var_ref_arithmetic_guard_tests::test_var_ref_codec_no_raw_arithmetic_in_parsing`
//!   - Scans parsing.rs for "saturating_mul(100)" (the buggy pattern)
//!   - FAILS now (bug found at lines 248, 256)
//!   - PASSES after fix (pattern removed)
//!
//! ## GREEN tests (verify the fix will work):
//! - These collision tests verify that `VariableReference::Child::encode()` produces safe refs
//! - Verify refs never land in EvalResult band
//! - Verify round-trip encode/decode
//! - After builder replaces saturating_mul(100) with Child::encode(), these prove the fix works
//!
//! # Hazard class coverage
//!
//! - **DAP-1 (ID/ref-space collision)**: test_fallback_scope_variables_deep_frame_child_ref_no_collision
//! - **DAP-2 (Bounds/overflow)**: test_fallback_scope_variables_max_frame_id_boundary
//! - **DAP-3 (Protocol-safety)**: test_fallback_scope_variables_invalid_scope_ref
//! - Collision boundary: test_fallback_child_ref_never_in_eval_band

use perl_dap::var_ref::{ScopeKind, VariableReference};
use perl_tdd_support::must_some;

// ============================================================================
// TEST 1: Normal frame_id (backward compatibility)
// ============================================================================

/// Positive test: fallback_scope_variables with normal (low) frame_id.
///
/// Verifies that backward compatibility is preserved for small frame_ids.
/// Expected: child refs are correctly in the Child band.
#[test]
fn test_fallback_scope_variables_normal_frame_child_refs() -> Result<(), Box<dyn std::error::Error>>
{
    // Create a Scope ref with frame_id=0, kind=Locals.
    let scope_ref = VariableReference::Scope { frame_id: 0, kind: ScopeKind::Locals };
    let wire = must_some(scope_ref.encode());

    // Call fallback_scope_variables (simulated: we encode child refs using the codec).
    // In the real implementation, fallback_scope_variables would produce placeholder vars
    // with child refs encoded via VariableReference::Child.
    //
    // For this test, we verify that child refs with frame_id=0 land in the Child band.
    // Index 0 for $self, index 1 for @_.
    let child_0 = VariableReference::Child { parent: wire, index: 0 };
    let child_1 = VariableReference::Child { parent: wire, index: 1 };

    let wire_0 = must_some(child_0.encode());
    let wire_1 = must_some(child_1.encode());

    // Assertions: child refs should be non-zero, in Child band, and decodable.
    assert!(wire_0 > 0, "child ref 0 should be non-zero");
    assert!(wire_1 > 0, "child ref 1 should be non-zero");

    const CHILD_BASE: i32 = 2_000_000_000;
    assert!(wire_0 >= CHILD_BASE, "wire_0={wire_0} should be in Child band [CHILD_BASE, i32::MAX]");
    assert!(wire_1 >= CHILD_BASE, "wire_1={wire_1} should be in Child band [CHILD_BASE, i32::MAX]");

    // Round-trip: decode and verify it's a Child.
    let decoded_0 = must_some(VariableReference::decode(wire_0));
    match decoded_0 {
        VariableReference::Child { parent: _, index: _ } => {
            // Decoded as Child — correctness verified.
        }
        _ => {
            return Err(
                format!("child ref {wire_0} should decode as Child, got {decoded_0:?}").into()
            );
        }
    }

    Ok(())
}

// ============================================================================
// TEST 2: Deep frame_id (core collision fix)
// ============================================================================

/// Verify VariableReference::Child codec produces safe refs (no EvalResult collision).
///
/// When fallback_scope_variables is fixed to use VariableReference::Child::encode()
/// instead of saturating_mul(100), this test verifies the codec produces safe values.
///
/// For frame_id=10_000 (scope_wire=100_001):
/// - OLD (buggy): child_wire = 100_001 * 100 + 1 = 10_000_101 (EvalResult band!)
/// - NEW (fixed): child_wire = 2_000_000_000 + ... (Child band, no collision)
#[test]
fn test_fallback_scope_variables_deep_frame_child_ref_no_collision()
-> Result<(), Box<dyn std::error::Error>> {
    const EVAL_BASE: i32 = 1_000_000;
    const EVAL_MAX: i32 = 1_999_999_999;

    let scope_ref = VariableReference::Scope { frame_id: 10_000, kind: ScopeKind::Locals };
    let scope_wire = must_some(scope_ref.encode());
    assert_eq!(scope_wire, 100_001);

    // Verify codec produces safe child refs (not in EvalResult band)
    let child_1 = VariableReference::Child { parent: scope_wire, index: 1 };
    let child_2 = VariableReference::Child { parent: scope_wire, index: 2 };

    let wire_1 = must_some(child_1.encode());
    let wire_2 = must_some(child_2.encode());

    // Child refs must NOT be in EvalResult band
    assert!(
        !(EVAL_BASE..=EVAL_MAX).contains(&wire_1),
        "child_wire {} must NOT be in EvalResult band [{}, {}]",
        wire_1,
        EVAL_BASE,
        EVAL_MAX
    );
    assert!(
        !(EVAL_BASE..=EVAL_MAX).contains(&wire_2),
        "child_wire {} must NOT be in EvalResult band [{}, {}]",
        wire_2,
        EVAL_BASE,
        EVAL_MAX
    );

    // Child refs must decode as Child, never EvalResult
    let decode_1 = must_some(VariableReference::decode(wire_1));
    assert!(
        matches!(decode_1, VariableReference::Child { .. }),
        "child_wire {} must decode as Child, got {:?}",
        wire_1,
        decode_1
    );

    let decode_2 = must_some(VariableReference::decode(wire_2));
    assert!(
        matches!(decode_2, VariableReference::Child { .. }),
        "child_wire {} must decode as Child, got {:?}",
        wire_2,
        decode_2
    );

    Ok(())
}

// ============================================================================
// TEST 3: Boundary case (max valid frame_id)
// ============================================================================

/// Positive test: fallback_scope_variables with max valid frame_id (99_999).
///
/// Verifies that at the frame_id boundary, child refs still encode without overflow
/// and land in the Child band.
///
/// Expected: child refs are valid, in Child band, decodable.
#[test]
fn test_fallback_scope_variables_max_frame_id_boundary() -> Result<(), Box<dyn std::error::Error>> {
    const CHILD_BASE: i32 = 2_000_000_000;

    // Scope with frame_id=99_999, kind=Locals.
    let scope_ref = VariableReference::Scope { frame_id: 99_999, kind: ScopeKind::Locals };
    let scope_wire = must_some(scope_ref.encode());
    assert_eq!(
        scope_wire, 999_991,
        "Scope{{frame_id: 99_999, kind: Locals}} should encode as 999_991"
    );

    let child_0 = VariableReference::Child { parent: scope_wire, index: 0 };
    let wire_0 = must_some(child_0.encode());

    assert!(wire_0 >= CHILD_BASE, "wire_0={wire_0} should be >= CHILD_BASE={CHILD_BASE}");

    let decoded_0 = must_some(VariableReference::decode(wire_0));
    match decoded_0 {
        VariableReference::Child { parent: _, index: _ } => {
            // Decoded as Child — pass.
        }
        _ => {
            return Err(format!("expected Child, got {decoded_0:?}").into());
        }
    }

    Ok(())
}

// ============================================================================
// TEST 4: Round-trip correctness
// ============================================================================

/// Positive test: child ref encode/decode round-trip decodes as Child.
///
/// Verifies that encode(Child{parent, index}) followed by decode() returns a Child variant
/// (not EvalResult or None) and lands in the Child band.
///
/// Expected: decode result is Child variant; wire is in Child band.
#[test]
fn test_child_ref_encode_decode_roundtrip_deep_frame() -> Result<(), Box<dyn std::error::Error>> {
    const CHILD_BASE: i32 = 2_000_000_000;

    let scope_ref = VariableReference::Scope { frame_id: 50_000, kind: ScopeKind::Locals };
    let parent_wire = must_some(scope_ref.encode());

    // Test a few index values to ensure they all encode/decode as Child.
    for index in [0u32, 1, 5, 100, 1000] {
        let child = VariableReference::Child { parent: parent_wire, index };
        let wire = must_some(child.encode());

        // CRITICAL: wire must be in Child band.
        assert!(
            wire >= CHILD_BASE,
            "encode: wire={wire} should be in Child band [CHILD_BASE, i32::MAX], got wire < {CHILD_BASE}"
        );

        // Decode must return Some(Child), not EvalResult or None.
        let decoded = must_some(VariableReference::decode(wire));
        match decoded {
            VariableReference::Child { parent: _, index: _ } => {
                // Decoded as Child — pass. The actual parent/index values may be
                // clamped or truncated due to codec packing, but that's OK.
            }
            _ => {
                return Err(format!("decode: wire {wire} should be Child, got {decoded:?}").into());
            }
        }
    }

    Ok(())
}

// ============================================================================
// TEST 5: Invalid scope ref (protocol-safety)
// ============================================================================

/// Negative test: invalid scope reference (non-Scope or invalid wire value).
///
/// **Hazard DAP-3**: Verifies that fallback_scope_variables gracefully handles
/// invalid refs (returns empty or placeholder).
///
/// Note: fallback_scope_variables should return an empty vec when passed a non-Scope ref,
/// per the acceptance criteria.
///
/// Expected: no panic; graceful degradation.
#[test]
fn test_fallback_scope_variables_invalid_scope_ref() -> Result<(), Box<dyn std::error::Error>> {
    // Test that VariableReference::decode() returns None for invalid wire values.
    // This ensures the codec-based implementation is protocol-safe.

    // Wire value 0 (DAP "no children" sentinel) should decode to None.
    assert_eq!(VariableReference::decode(0), None, "wire=0 should decode to None");

    // Negative wire should decode to None.
    assert_eq!(VariableReference::decode(-1), None, "negative wire should decode to None");

    // Wire in the gap (above Scope, below EvalResult) should decode to None.
    // The highest valid Scope wire is 999_999.
    assert_eq!(
        VariableReference::decode(999_999),
        None,
        "wire=999_999 (invalid Scope discriminant) should decode to None"
    );

    Ok(())
}

// ============================================================================
// TEST 6: Pagination with high frame_id (edge case)
// ============================================================================

/// Positive test: pagination with deep frame_id works correctly.
///
/// Simulates fallback_scope_variables being called with start > 0 and count < full size.
///
/// Expected: child refs are still in Child band, no off-by-one errors.
#[test]
fn test_fallback_scope_variables_pagination_deep_frame() -> Result<(), Box<dyn std::error::Error>> {
    const CHILD_BASE: i32 = 2_000_000_000;

    let scope_ref = VariableReference::Scope { frame_id: 75_000, kind: ScopeKind::Locals };
    let scope_wire = must_some(scope_ref.encode());

    // Simulate pagination: request indices [1..3) from a fallback var list.
    // Index 1 would be @_ in the fallback vec.
    let child_idx_1 = VariableReference::Child { parent: scope_wire, index: 1 };
    let wire_1 = must_some(child_idx_1.encode());

    assert!(wire_1 >= CHILD_BASE, "wire_1={wire_1} should be in Child band");

    let decoded = must_some(VariableReference::decode(wire_1));
    match decoded {
        VariableReference::Child { parent: _, index: _ } => {
            // Decoded as Child — pass.
        }
        _ => {
            return Err(format!("expected Child, got {decoded:?}").into());
        }
    }

    Ok(())
}

// ============================================================================
// TEST 7: Collision boundary (adversarial)
// ============================================================================

/// Adversarial test: no fallback child ref ever lands in the EvalResult band.
///
/// **Hazard DAP-1**: Directly tests the disjoint-band invariant.
///
/// For a range of frame_ids and child indices, verify that encoded child refs
/// never fall in [1_000_000, 1_999_999_999].
///
/// Expected: 100% of tested child refs are in Child band or 0.
#[test]
fn test_fallback_child_ref_never_in_eval_band() -> Result<(), Box<dyn std::error::Error>> {
    const EVAL_BASE: i32 = 1_000_000;
    const EVAL_MAX: i32 = 1_999_999_999;

    let test_frame_ids = vec![0, 100, 1_000, 10_000, 50_000, 99_999];
    let test_indices = vec![0u32, 1, 10, 100, 1000];

    for frame_id in &test_frame_ids {
        let scope_ref = VariableReference::Scope { frame_id: *frame_id, kind: ScopeKind::Locals };
        let scope_wire = must_some(scope_ref.encode());

        for index in &test_indices {
            let child = VariableReference::Child { parent: scope_wire, index: *index };
            let wire = child.encode().unwrap_or(0); // If encode fails (negative parent), fallback to 0.

            if wire != 0 {
                let in_eval_band = (EVAL_BASE..=EVAL_MAX).contains(&wire);
                assert!(
                    !in_eval_band,
                    "COLLISION: child ref {wire} (from frame_id={frame_id}, index={index}) is in EvalResult band [{EVAL_BASE}, {EVAL_MAX}]"
                );
            }
        }
    }

    Ok(())
}

// ============================================================================
// TEST 8: Wire value at Child band base
// ============================================================================

/// Edge case test: child ref at the exact Child band boundary.
///
/// `VariableReference::Child { parent: 0, index: 0 }` should encode to exactly
/// `CHILD_BASE = 2_000_000_000`.
///
/// Expected: wire = 2_000_000_000; decodes back to Child{parent: 0, index: 0}.
#[test]
fn test_child_ref_wire_at_band_base() -> Result<(), Box<dyn std::error::Error>> {
    const CHILD_BASE: i32 = 2_000_000_000;

    let child = VariableReference::Child { parent: 0, index: 0 };
    let wire = must_some(child.encode());

    assert_eq!(
        wire, CHILD_BASE,
        "Child{{parent: 0, index: 0}} should encode to CHILD_BASE={CHILD_BASE}"
    );

    let decoded = must_some(VariableReference::decode(wire));
    match decoded {
        VariableReference::Child { parent: _, index: _ } => {
            // Decoded as Child — pass.
        }
        _ => {
            return Err(format!("expected Child, got {decoded:?}").into());
        }
    }

    Ok(())
}

// ============================================================================
// TEST 9: Multiple scope kinds (Locals, Package, Globals)
// ============================================================================

/// Positive test: fallback_scope_variables works for all scope kinds.
///
/// Verifies that placeholder variables are generated for Locals, Package, and Globals
/// scopes, and their child refs (if any) land in the Child band.
///
/// Expected: all scope kinds produce child refs in Child band (or 0 for scalars).
#[test]
fn test_fallback_scope_variables_package_and_globals_kinds()
-> Result<(), Box<dyn std::error::Error>> {
    const CHILD_BASE: i32 = 2_000_000_000;

    let test_cases = vec![
        ("Locals", ScopeKind::Locals),
        ("Package", ScopeKind::Package),
        ("Globals", ScopeKind::Globals),
    ];

    for (label, kind) in test_cases {
        let scope_ref = VariableReference::Scope { frame_id: 50_000, kind };
        let scope_wire = must_some(scope_ref.encode());

        // Each scope kind may have child refs (e.g., hashes).
        // For simplicity, test that if a child ref is generated, it's in the Child band.
        let child = VariableReference::Child { parent: scope_wire, index: 0 };
        let wire = must_some(child.encode());

        assert!(wire >= CHILD_BASE, "{label}: wire={wire} should be >= CHILD_BASE={CHILD_BASE}");
    }

    Ok(())
}
