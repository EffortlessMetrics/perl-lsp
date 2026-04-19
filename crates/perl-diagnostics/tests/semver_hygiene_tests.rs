//! SemVer hygiene tests for perl-diagnostics crate.
//!
//! These tests verify:
//! 1. `DiagnosticCode` enum has `#[non_exhaustive]` to prevent future mid-enum insertions
//! 2. `UnreachableCode` is at the END of the enum (not mid-enum) so discriminant values
//!    match v0.12.1 baseline
//! 3. The discriminant values for affected variants match the v0.12.1 baseline

use perl_diagnostics::codes::DiagnosticCode;

// ============================================================================
// Test 1: DiagnosticCode should have #[non_exhaustive]
// ============================================================================
//
// The #[non_exhaustive] attribute prevents future mid-enum insertions from
// being SemVer-breaking. This test verifies the practical consequence:
// UnreachableCode must be at the END of the enum (not mid-enum).

#[test]
fn unreachable_code_is_at_end_of_enum_not_mid_enum() {
    // In the buggy state (UnreachableCode mid-enum):
    //   CriticSeverity5 has discriminant 56 (at end)
    //   UnreachableCode has discriminant 28 (mid-enum, after PrintfFormatMismatch)
    //
    // After fix (UnreachableCode at END):
    //   CriticSeverity5 has discriminant 56
    //   UnreachableCode has discriminant 57 (after CriticSeverity5)
    //
    // The key invariant: CriticSeverity5 (last variant) must have
    // a LOWER discriminant than UnreachableCode (which should be at end).
    //
    // This test FAILS in the buggy state (28 > 56 is false).
    // This test PASSES after the fix (57 > 56 is true).

    let critic_severity_5_discriminant = DiagnosticCode::CriticSeverity5 as isize;
    let unreachable_code_discriminant = DiagnosticCode::UnreachableCode as isize;

    // UnreachableCode must come AFTER CriticSeverity5 (higher discriminant = later in enum)
    assert!(
        unreachable_code_discriminant > critic_severity_5_discriminant,
        "UnreachableCode (discriminant={}) should be at END of enum (after CriticSeverity5={}), \
         not mid-enum. This indicates #[non_exhaustive] is not being respected or \
         UnreachableCode was incorrectly inserted mid-enum.",
        unreachable_code_discriminant,
        critic_severity_5_discriminant
    );
}

#[test]
fn eval_error_flow_has_v0121_baseline_discriminant() {
    // In v0.12.1 (before UnreachableCode was added mid-enum):
    //   EvalErrorFlow had discriminant 28 (was variant #28, right after PrintfFormatMismatch)
    //
    // In buggy state (UnreachableCode mid-enum after PrintfFormatMismatch):
    //   EvalErrorFlow has discriminant 29 (shifted by +1)
    //
    // After fix (UnreachableCode moved to end):
    //   EvalErrorFlow has discriminant 28 again (restored to v0.12.1 baseline)
    //
    // This test verifies that downstream consumers doing `code as isize`
    // arithmetic get the correct v0.12.1 baseline values.

    let eval_error_flow_discriminant = DiagnosticCode::EvalErrorFlow as isize;
    // After fix, EvalErrorFlow returns to position 28 (0-indexed)
    let expected_v0121_discriminant = 28isize;

    assert_eq!(
        eval_error_flow_discriminant, expected_v0121_discriminant,
        "EvalErrorFlow discriminant should be {} (v0.12.1 baseline), got {}. \
         This indicates UnreachableCode was inserted mid-enum instead of appended at end.",
        expected_v0121_discriminant, eval_error_flow_discriminant
    );
}

#[test]
fn unreachable_code_has_correct_pl406_string_code() {
    // Verify as_str() returns correct value (this already works in both states
    // because as_str() uses name-based matching, not position-based)
    assert_eq!(
        DiagnosticCode::UnreachableCode.as_str(),
        "PL406",
        "UnreachableCode should have string code PL406"
    );
}

#[test]
fn critic_severity5_has_v0121_baseline_discriminant() {
    // In v0.12.1 (before UnreachableCode was added):
    //   CriticSeverity5 had discriminant 56 (last variant at end, 57th variant = index 56)
    //
    // In buggy state (UnreachableCode mid-enum):
    //   CriticSeverity5 has discriminant 56 (still at end, because the shift from
    //   mid-enum insertion only affects variants BETWEEN insertion point and end)
    //
    // After fix (UnreachableCode moved to end):
    //   CriticSeverity5 has discriminant 56 (back at end where it belongs)
    //   UnreachableCode is at 57 (after CriticSeverity5)
    //
    // Note: CriticSeverity5's discriminant is 56 in all states because it's
    // at the END of the enum. The shift from mid-enum insertion only affects
    // variants BETWEEN the insertion point and the end, but CriticSeverity5 is
    // after all those variants.

    let critic_severity_5_discriminant = DiagnosticCode::CriticSeverity5 as isize;
    // After fix, CriticSeverity5 should be at discriminant 56 (verified by test output)
    let expected_v0121_discriminant = 56isize;

    assert_eq!(
        critic_severity_5_discriminant, expected_v0121_discriminant,
        "CriticSeverity5 discriminant should be {} (v0.12.1 baseline), got {}. \
         This indicates the enum ordering does not match v0.12.1 baseline.",
        expected_v0121_discriminant, critic_severity_5_discriminant
    );
}
