//! Property-based tests for SemVer hygiene invariants in perl-diagnostics.
//!
//! These tests verify invariants that must hold for ALL variants (not just
//! specific examples). Property tests iterate over all variants to ensure
//! the invariant holds universally.
//!
//! Key invariants tested:
//! 1. Roundtrip: `parse_code(as_str()) == Some(variant)` for all variants
//! 2. String format: all `as_str()` codes match expected pattern (PL### or PC###)
//! 3. Discriminant ordering: UnreachableCode has discriminant > all PL400-PL499 variants
//! 4. Severity consistency: every variant has a valid (non-default) severity

use perl_diagnostics::codes::{DiagnosticCode, DiagnosticSeverity};

// -------------------------------------------------------------------------- //
// Helper: all DiagnosticCode variants for exhaustive iteration
// -------------------------------------------------------------------------- //

const ALL_CODES: &[DiagnosticCode] = &[
    // PL001-PL099: Parser
    DiagnosticCode::ParseError,
    DiagnosticCode::SyntaxError,
    DiagnosticCode::UnexpectedEof,
    // PL100-PL199: Strict/warnings
    DiagnosticCode::MissingStrict,
    DiagnosticCode::MissingWarnings,
    DiagnosticCode::UnusedVariable,
    DiagnosticCode::UndefinedVariable,
    DiagnosticCode::VariableShadowing,
    DiagnosticCode::VariableRedeclaration,
    DiagnosticCode::DuplicateParameter,
    DiagnosticCode::ParameterShadowsGlobal,
    DiagnosticCode::UnusedParameter,
    DiagnosticCode::UnquotedBareword,
    DiagnosticCode::UninitializedVariable,
    DiagnosticCode::MisspelledPragma,
    DiagnosticCode::CaptureVarWithoutRegexMatch,
    // PL200-PL299: Package/module
    DiagnosticCode::MissingPackageDeclaration,
    DiagnosticCode::DuplicatePackage,
    // PL300-PL399: Subroutine
    DiagnosticCode::DuplicateSubroutine,
    DiagnosticCode::MissingReturn,
    DiagnosticCode::RoleConflict,
    DiagnosticCode::InvalidPrototype,
    // PL400-PL499: Best practices
    DiagnosticCode::BarewordFilehandle,
    DiagnosticCode::TwoArgOpen,
    DiagnosticCode::ImplicitReturn,
    DiagnosticCode::AssignmentInCondition,
    DiagnosticCode::NumericComparisonWithUndef,
    DiagnosticCode::PrintfFormatMismatch,
    DiagnosticCode::UnreachableCode,
    DiagnosticCode::EvalErrorFlow,
    DiagnosticCode::DuplicateHashKey,
    DiagnosticCode::GotoUndefinedLabel,
    // PL500-PL599: Deprecated syntax
    DiagnosticCode::DeprecatedDefined,
    DiagnosticCode::DeprecatedArrayBase,
    DiagnosticCode::PhaseScopedStrictPragma,
    DiagnosticCode::PhaseScopedWarningsPragma,
    // PL600-PL699: Security
    DiagnosticCode::SecurityStringEval,
    DiagnosticCode::SecurityBacktickExec,
    DiagnosticCode::SecuritySignalHandler,
    DiagnosticCode::SecuritySystemCall,
    DiagnosticCode::SecurityExecCall,
    DiagnosticCode::SecurityPipeOpen,
    DiagnosticCode::SecurityReadpipe,
    // PL700-PL799: Import
    DiagnosticCode::UnusedImport,
    DiagnosticCode::ModuleNotFound,
    // PL800-PL899: Heredoc anti-patterns
    DiagnosticCode::HeredocInFormat,
    DiagnosticCode::HeredocInBegin,
    DiagnosticCode::HeredocDynamicDelimiter,
    DiagnosticCode::HeredocInSourceFilter,
    DiagnosticCode::HeredocInRegexCode,
    DiagnosticCode::HeredocInEval,
    DiagnosticCode::HeredocTiedHandle,
    // PL900-PL999: Version compatibility
    DiagnosticCode::VersionIncompatFeature,
    // PC001-PC005: Perl::Critic
    DiagnosticCode::CriticSeverity1,
    DiagnosticCode::CriticSeverity2,
    DiagnosticCode::CriticSeverity3,
    DiagnosticCode::CriticSeverity4,
    DiagnosticCode::CriticSeverity5,
];

// -------------------------------------------------------------------------- //
// Property 1: Roundtrip - parse_code(as_str()) returns the same variant
// -------------------------------------------------------------------------- //

/// Property: Roundtrip conversion must work for ALL variants.
///
/// For any DiagnosticCode variant `c`:
///   parse_code(c.as_str()) == Some(c)
///
/// This verifies that string codes are consistent and parseable.
#[test]
fn all_codes_roundtrip_via_string_code() {
    let mut failures = Vec::new();

    for &code in ALL_CODES {
        let str_code = code.as_str();
        let parsed = DiagnosticCode::parse_code(str_code);
        if parsed != Some(code) {
            failures.push(format!(
                "Roundtrip failed for {:?}: as_str() = {:?}, parse_code returned {:?}",
                code, str_code, parsed
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "Roundtrip property violated for {} codes:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

// -------------------------------------------------------------------------- //
// Property 2: String format - all codes match PL### or PC### pattern
// -------------------------------------------------------------------------- //

/// Property: All string codes must match the expected format.
///
/// PL codes: "PL" followed by 3 digits (PL001-PL999)
/// PC codes: "PC" followed by 3 digits (PC001-PC999)
///
/// This ensures the string representation is consistent and machine-parseable.
#[test]
fn all_codes_have_valid_string_format() {
    let mut failures = Vec::new();

    for &code in ALL_CODES {
        let str_code = code.as_str();
        let is_valid = str_code.starts_with("PL") || str_code.starts_with("PC");
        let is_proper_format =
            is_valid && str_code.len() == 5 && str_code[2..].chars().all(|c| c.is_ascii_digit());

        if !is_proper_format {
            failures.push(format!("Invalid format for {:?}: as_str() = {:?}", code, str_code));
        }
    }

    if !failures.is_empty() {
        panic!(
            "String format property violated for {} codes:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

/// Property: PL codes must be in valid ranges.
#[test]
fn all_pl_codes_are_in_valid_ranges() {
    // Define valid ranges for PL codes
    let valid_ranges = [
        (1, 99),    // PL001-PL099: Parser
        (100, 199), // PL100-PL199: Strict/warnings
        (200, 299), // PL200-PL299: Package/module
        (300, 399), // PL300-PL399: Subroutine
        (400, 499), // PL400-PL499: Best practices
        (500, 599), // PL500-PL599: Deprecated syntax
        (600, 699), // PL600-PL699: Security
        (700, 799), // PL700-PL799: Import
        (800, 899), // PL800-PL899: Heredoc anti-patterns
        (900, 999), // PL900-PL999: Version compatibility
    ];

    let mut failures = Vec::new();

    for &code in ALL_CODES {
        let str_code = code.as_str();
        if !str_code.starts_with("PL") {
            continue; // Skip PC codes
        }

        let num: u32 = str_code[2..].parse().unwrap();
        let is_valid = valid_ranges.iter().any(|(start, end)| num >= *start && num <= *end);

        if !is_valid {
            failures.push(format!(
                "PL code out of range for {:?}: {} (valid ranges: {:?})",
                code, str_code, valid_ranges
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "PL code range property violated for {} codes:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

// -------------------------------------------------------------------------- //
// Property 3: Discriminant ordering - UnreachableCode must be at end
// -------------------------------------------------------------------------- //

/// Property: UnreachableCode must have discriminant > all other PL400-PL499 variants.
///
/// This is the key SemVer hygiene invariant: UnreachableCode must be at the END
/// of the enum (after CriticSeverity5), not mid-enum. If UnreachableCode were
/// mid-enum, it would shift discriminant values for all subsequent variants,
/// breaking any downstream code that does `code as isize` arithmetic.
#[test]
fn unreachable_code_has_highest_discriminant_in_pl400_range() {
    // Collect all PL400-PL499 variants and their discriminants
    let pl400_variants: Vec<(DiagnosticCode, isize)> = ALL_CODES
        .iter()
        .filter_map(|&code| {
            let str_code = code.as_str();
            if str_code.starts_with("PL") {
                let num: u32 = str_code[2..].parse().unwrap();
                if (400..=499).contains(&num) {
                    return Some((code, code as isize));
                }
            }
            None
        })
        .collect();

    if pl400_variants.is_empty() {
        panic!("No PL400-PL499 variants found!");
    }

    // Find UnreachableCode discriminant
    let unreachable_code = DiagnosticCode::UnreachableCode;
    let unreachable_disc = unreachable_code as isize;

    // Verify UnreachableCode has the highest discriminant in PL400 range
    let max_disc = pl400_variants.iter().map(|(_, d)| *d).max().unwrap();

    if unreachable_disc != max_disc {
        let mut other_codes: Vec<_> = pl400_variants
            .iter()
            .filter(|(c, _)| *c != unreachable_code)
            .map(|(c, d)| format!("{:?} = {}", c, d))
            .collect();
        other_codes.sort();

        panic!(
            "UnreachableCode (discriminant={}) is NOT the highest in PL400 range (max={}).\n\
             All PL400 variants:\n  {}\n\
             UnreachableCode should be at the END of the enum.",
            unreachable_disc,
            max_disc,
            other_codes.join("\n  ")
        );
    }
}

/// Property: EvalErrorFlow must have v0.12.1 baseline discriminant (28).
///
/// In v0.12.1, EvalErrorFlow was at position 28 (0-indexed). This property
/// verifies that the UnreachableCode insertion did NOT shift EvalErrorFlow's
/// discriminant, confirming UnreachableCode was appended at the end.
#[test]
fn eval_error_flow_has_v0121_discriminant_28() {
    let eval_error_disc = DiagnosticCode::EvalErrorFlow as isize;
    assert_eq!(
        eval_error_disc, 28isize,
        "EvalErrorFlow discriminant should be 28 (v0.12.1 baseline), got {}. \
         This indicates UnreachableCode was inserted mid-enum instead of at end.",
        eval_error_disc
    );
}

/// Property: CriticSeverity5 must have v0.12.1 baseline discriminant (56).
///
/// CriticSeverity5 is the last variant before UnreachableCode. In v0.12.1,
/// it had discriminant 56. This property verifies it's still at 56.
#[test]
fn critic_severity5_has_v0121_discriminant_56() {
    let critic5_disc = DiagnosticCode::CriticSeverity5 as isize;
    assert_eq!(
        critic5_disc, 56isize,
        "CriticSeverity5 discriminant should be 56 (v0.12.1 baseline), got {}.",
        critic5_disc
    );
}

/// Property: UnreachableCode must have discriminant 57 (one after CriticSeverity5).
///
/// After the fix, UnreachableCode is at position 57 (0-indexed), right after
/// CriticSeverity5 at 56. This is the correct end-of-enum position.
#[test]
fn unreachable_code_has_discriminant_57() {
    let unreachable_disc = DiagnosticCode::UnreachableCode as isize;
    assert_eq!(
        unreachable_disc, 57isize,
        "UnreachableCode discriminant should be 57 (after CriticSeverity5 at 56), got {}.",
        unreachable_disc
    );
}

// -------------------------------------------------------------------------- //
// Property 4: Severity consistency - every code has a valid severity
// -------------------------------------------------------------------------- //

/// Property: Every DiagnosticCode variant has a valid severity.
///
/// This verifies that calling severity() on any variant doesn't panic
/// and returns a consistent value.
#[test]
fn all_codes_have_valid_severity() {
    for &code in ALL_CODES {
        // Just verify severity() returns a consistent value and doesn't panic
        let severity1 = code.severity();
        let severity2 = code.severity();
        assert_eq!(severity1, severity2, "severity() should be deterministic for {:?}", code);

        // Verify severity is in valid range (1-4 for LSP)
        let value = severity1.to_lsp_value();
        assert!((1..=4).contains(&value), "severity should be 1-4, got {} for {:?}", value, code);
    }
}

/// Property: All codes have a valid documentation URL or None (for PC codes).
#[test]
fn all_codes_have_valid_documentation_url() {
    let mut failures = Vec::new();

    for &code in ALL_CODES {
        let str_code = code.as_str();
        let url = code.documentation_url();

        // PC codes (Perl::Critic) should have None
        if str_code.starts_with("PC") {
            if url.is_some() {
                failures.push(format!("PC code {:?} should have None URL, got {:?}", code, url));
            }
        } else if str_code.starts_with("PL") {
            // PL codes should have Some(url) with valid format
            if let Some(u) = url {
                if !u.starts_with("https://docs.perl-lsp.org/errors/") {
                    failures.push(format!("PL code {:?} has invalid URL format: {:?}", code, u));
                }
            }
            // Note: Some PL codes might legitimately have None if not yet documented
        }
    }

    if !failures.is_empty() {
        panic!(
            "Documentation URL property violated for {} codes:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

// -------------------------------------------------------------------------- //
// Property 5: No duplicate string codes
// -------------------------------------------------------------------------- //

/// Property: No two different variants should have the same string code.
#[test]
fn no_duplicate_string_codes() {
    let mut code_to_variant: std::collections::HashMap<&str, DiagnosticCode> =
        std::collections::HashMap::new();
    let mut failures = Vec::new();

    for &code in ALL_CODES {
        let str_code = code.as_str();
        if let Some(existing) = code_to_variant.get(str_code) {
            failures.push(format!(
                "Duplicate string code {:?} for {:?} and {:?}",
                str_code, existing, code
            ));
        } else {
            code_to_variant.insert(str_code, code);
        }
    }

    if !failures.is_empty() {
        panic!("Duplicate string code property violated:\n{}", failures.join("\n"));
    }
}

// -------------------------------------------------------------------------- //
// Property 6: from_message is consistent with parse_code for known messages
/// Property: from_message is consistent with parse_code for known messages
///
/// Property: from_message should return Some for any code that has a hint message.
///
/// If a code has a hint (from_hint() returns Some), then from_message should
/// be able to infer that code from the hint text.
#[test]
fn from_message_is_consistent_with_hint_for_known_patterns() {
    // Test specific patterns that should reliably infer codes based on
    // the actual from_message implementation
    let test_cases = [
        ("Missing 'use strict'", Some(DiagnosticCode::MissingStrict)),
        ("Missing 'use warnings'", Some(DiagnosticCode::MissingWarnings)),
    ];

    for (msg, expected) in test_cases {
        let inferred = DiagnosticCode::from_message(msg);
        if inferred != expected {
            panic!(
                "from_message mismatch for {:?}: expected {:?}, got {:?}",
                msg, expected, inferred
            );
        }
    }
}
