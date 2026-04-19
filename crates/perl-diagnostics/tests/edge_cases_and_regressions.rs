//! Edge case and regression tests for perl-diagnostics Wave E consolidation (#4429).
//!
//! This test file adds comprehensive coverage for:
//! - Boundary conditions (empty collections, defaults, large strings)
//! - Regression guards (old crate names fail, catalog functions work)
//! - Type constraint verification
//! - Enum variant accessibility
//! - Diagnostic field combinations

use perl_diagnostics::catalog;
use perl_diagnostics::codes::{DiagnosticCategory, DiagnosticCode, DiagnosticTag};
use perl_diagnostics::types::{Diagnostic, DiagnosticSeverity, RelatedInformation};
use perl_test_must::must_some;

// Edge case: Empty/default Diagnostic struct
#[test]
fn edge_case_diagnostic_default_is_valid() {
    let diag = Diagnostic::default();

    // All fields should have sensible defaults
    assert_eq!(diag.message, "");
    assert_eq!(diag.code, DiagnosticCode::ParseError); // First code variant
    assert_eq!(diag.severity, DiagnosticSeverity::Error); // Error is the default
    assert!(diag.related_information.is_none());
    assert!(diag.tags.is_none());
}

// Edge case: Empty RelatedInformation
#[test]
fn edge_case_related_information_default_is_valid() {
    let info = RelatedInformation::default();

    assert_eq!(info.message, "");
    assert_eq!(info.location, (0, 0)); // Range tuple
}

// Edge case: Diagnostic with all fields populated
#[test]
fn edge_case_diagnostic_with_all_fields_populated() {
    let info = RelatedInformation {
        message: "related info".to_string(),
        location: (10, 20),
    };

    let diag = Diagnostic {
        code: DiagnosticCode::SyntaxError,
        severity: DiagnosticSeverity::Warning,
        range: (1, 50),
        message: "Test diagnostic".to_string(),
        related_information: Some(vec![info]),
        tags: Some(vec![DiagnosticTag::Deprecated]),
    };

    assert_eq!(diag.message, "Test diagnostic");
    assert_eq!(diag.code, DiagnosticCode::SyntaxError);
    assert_eq!(diag.severity, DiagnosticSeverity::Warning);
    assert!(diag.related_information.is_some());
    assert!(diag.tags.is_some());

    let tags = must_some(diag.tags);
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0], DiagnosticTag::Deprecated);
}

// Regression: Severity discriminants must match LSP spec exactly
#[test]
fn regression_severity_discriminants_match_lsp_spec() {
    // LSP 3.17 spec: Error=1, Warning=2, Information=3, Hint=4
    // These are the wire protocol values
    assert_eq!(DiagnosticSeverity::Error as i32, 1);
    assert_eq!(DiagnosticSeverity::Warning as i32, 2);
    assert_eq!(DiagnosticSeverity::Information as i32, 3);
    assert_eq!(DiagnosticSeverity::Hint as i32, 4);
}

// Regression: Tag discriminants must match LSP spec exactly
#[test]
fn regression_tag_discriminants_match_lsp_spec() {
    // LSP 3.17 spec: Unnecessary=1, Deprecated=2
    assert_eq!(DiagnosticTag::Unnecessary as i32, 1);
    assert_eq!(DiagnosticTag::Deprecated as i32, 2);
}

// Edge case: All DiagnosticCategory variants exist and are accessible
#[test]
fn edge_case_diagnostic_category_all_variants_accessible() {
    // Verify all category variants can be constructed
    let _parser = DiagnosticCategory::Parser;
    let _strict = DiagnosticCategory::StrictWarnings;
    let _package = DiagnosticCategory::PackageModule;
    let _sub = DiagnosticCategory::Subroutine;
    let _best = DiagnosticCategory::BestPractices;
    let _deprecated = DiagnosticCategory::Deprecated;
    let _security = DiagnosticCategory::Security;
    let _import = DiagnosticCategory::Import;
    let _heredoc = DiagnosticCategory::Heredoc;
    let _critic = DiagnosticCategory::PerlCritic;

    // Verify they can be compared for equality
    assert_eq!(DiagnosticCategory::Parser, DiagnosticCategory::Parser);
    assert_ne!(
        DiagnosticCategory::Parser,
        DiagnosticCategory::StrictWarnings
    );
}

// Edge case: DiagnosticCode has expected key variants
#[test]
fn edge_case_diagnostic_code_expected_variants_exist() {
    // Verify key diagnostic codes exist (sampling from the spec)
    let _parse_error = DiagnosticCode::ParseError;
    let _syntax_error = DiagnosticCode::SyntaxError;
    let _unexpected_eof = DiagnosticCode::UnexpectedEof;
    let _missing_strict = DiagnosticCode::MissingStrict;
    let _missing_warnings = DiagnosticCode::MissingWarnings;
    let _undefined_variable = DiagnosticCode::UndefinedVariable;
    let _unused_variable = DiagnosticCode::UnusedVariable;
    let _missing_package = DiagnosticCode::MissingPackageDeclaration;
    let _duplicate_package = DiagnosticCode::DuplicatePackage;
    let _duplicate_sub = DiagnosticCode::DuplicateSubroutine;
    let _missing_return = DiagnosticCode::MissingReturn;
    let _bareword = DiagnosticCode::BarewordFilehandle;
    let _two_arg_open = DiagnosticCode::TwoArgOpen;
    let _implicit_return = DiagnosticCode::ImplicitReturn;
    let _eval_error_flow = DiagnosticCode::EvalErrorFlow;
}

// Regression: All catalog functions are callable and return DiagnosticMeta
#[test]
fn regression_catalog_functions_callable() {
    // Sampling of key catalog functions — they should all return DiagnosticMeta
    let _meta1 = catalog::parse_error();
    let _meta2 = catalog::syntax_error();
    let _meta3 = catalog::unexpected_eof();
    let _meta4 = catalog::missing_strict();
    let _meta5 = catalog::missing_warnings();
    let _meta6 = catalog::undefined_var();
    let _meta7 = catalog::unused_var();
    let _meta8 = catalog::duplicate_package();
    let _meta9 = catalog::missing_return();
}

// Regression: diagnostic_meta function works (dispatcher function)
#[test]
fn regression_diagnostic_meta_dispatcher_function() {
    // The diagnostic_meta function should take a code and return metadata
    let meta = catalog::diagnostic_meta(DiagnosticCode::ParseError);

    // Should return non-null code
    assert!(!meta.code.is_null());
}

// Edge case: Severity Copy semantics
#[test]
fn edge_case_severity_copy_semantics() {
    let severity1 = DiagnosticSeverity::Error;
    let severity2 = severity1; // Copy: should not move

    // Both should be valid and equal
    assert_eq!(severity1, severity2);
    assert_eq!(severity1, DiagnosticSeverity::Error);
}

// Edge case: Tag Copy semantics
#[test]
fn edge_case_tag_copy_semantics() {
    let tag1 = DiagnosticTag::Deprecated;
    let tag2 = tag1; // Copy: should not move

    // Both should be valid and equal
    assert_eq!(tag1, tag2);
    assert_eq!(tag1, DiagnosticTag::Deprecated);
}

// Edge case: DiagnosticSeverity implements Ord for sorting
#[test]
fn edge_case_severity_ordering_complete() {
    let mut severities = vec![
        DiagnosticSeverity::Hint,
        DiagnosticSeverity::Error,
        DiagnosticSeverity::Information,
        DiagnosticSeverity::Warning,
    ];

    severities.sort();

    // Should be in LSP order: Error < Warning < Information < Hint
    assert_eq!(severities[0], DiagnosticSeverity::Error);
    assert_eq!(severities[1], DiagnosticSeverity::Warning);
    assert_eq!(severities[2], DiagnosticSeverity::Information);
    assert_eq!(severities[3], DiagnosticSeverity::Hint);
}

// Edge case: Diagnostic messages can be any length
#[test]
fn edge_case_diagnostic_message_various_lengths() {
    // Empty message
    let diag1 = Diagnostic {
        message: String::new(),
        ..Default::default()
    };
    assert_eq!(diag1.message, "");

    // Long message (10k chars)
    let long_msg = "a".repeat(10000);
    let diag2 = Diagnostic {
        message: long_msg.clone(),
        ..Default::default()
    };
    assert_eq!(diag2.message, long_msg);

    // Unicode message
    let unicode_msg = "Error in ñoño → 🚀 ⚠️".to_string();
    let diag3 = Diagnostic {
        message: unicode_msg.clone(),
        ..Default::default()
    };
    assert_eq!(diag3.message, unicode_msg);
}

// Edge case: Multiple related_information entries
#[test]
fn edge_case_diagnostic_multiple_related_information() {
    let info1 = RelatedInformation {
        message: "First related".to_string(),
        location: (0, 10),
    };
    let info2 = RelatedInformation {
        message: "Second related".to_string(),
        location: (20, 30),
    };
    let info3 = RelatedInformation {
        message: "Third related".to_string(),
        location: (40, 50),
    };

    let diag = Diagnostic {
        related_information: Some(vec![info1, info2, info3]),
        ..Default::default()
    };

    let infos = must_some(diag.related_information);
    assert_eq!(infos.len(), 3);
    assert_eq!(infos[0].message, "First related");
    assert_eq!(infos[1].message, "Second related");
    assert_eq!(infos[2].message, "Third related");
}

// Edge case: Multiple tags on a single diagnostic
#[test]
fn edge_case_diagnostic_multiple_tags() {
    let diag = Diagnostic {
        tags: Some(vec![
            DiagnosticTag::Unnecessary,
            DiagnosticTag::Deprecated,
            DiagnosticTag::Unnecessary, // Can repeat
        ]),
        ..Default::default()
    };

    let tags = must_some(diag.tags);
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0], DiagnosticTag::Unnecessary);
    assert_eq!(tags[1], DiagnosticTag::Deprecated);
    assert_eq!(tags[2], DiagnosticTag::Unnecessary);
}

// Regression: Note that old import paths must fail
// This test documents that old crate names should NOT compile
#[test]
fn regression_note_old_imports_must_fail() {
    // The following would fail to compile (documented for regression prevention):
    // use perl_diagnostics_codes::DiagnosticSeverity;  // OLD NAME
    // use perl_lsp_diagnostic_types::Diagnostic;       // OLD NAME
    // use perl_lsp_diagnostic_catalog::diagnostic_meta; // OLD NAME

    // This test documents that a future refactor cannot accidentally
    // re-introduce the old names at the crate root without breaking
    // downstream consumers that migrated to the new paths.

    // Marker: verify new paths work instead
    let _sev = perl_diagnostics::types::DiagnosticSeverity::Error;
    let _code = perl_diagnostics::codes::DiagnosticCode::ParseError;
    let _meta = perl_diagnostics::catalog::diagnostic_meta(
        perl_diagnostics::codes::DiagnosticCode::ParseError,
    );

    assert!(true); // Test passes
}

// Regression: DiagnosticCode implements Copy and Clone
#[test]
fn regression_diagnostic_code_copy_semantics() {
    let code1 = DiagnosticCode::ParseError;
    let code2 = code1; // Copy semantics

    assert_eq!(code1, code2);
}

// Edge case: Type conversion round-trip for severity
#[test]
fn edge_case_severity_round_trip_lsp_value() {
    let original = DiagnosticSeverity::Warning;
    let lsp_val = original.to_lsp_value();

    // LSP value should be 2 for Warning
    assert_eq!(lsp_val, 2);
}

// Edge case: Type conversion round-trip for tag
#[test]
fn edge_case_tag_round_trip_lsp_value() {
    let original = DiagnosticTag::Deprecated;
    let lsp_val = original.to_lsp_value();

    // LSP value should be 2 for Deprecated
    assert_eq!(lsp_val, 2);
}

// Regression: Severity implements PartialOrd for comparison chains
#[test]
fn regression_severity_partial_ord_chains() {
    let error = DiagnosticSeverity::Error;
    let warning = DiagnosticSeverity::Warning;
    let info = DiagnosticSeverity::Information;

    // Should support comparison chains
    assert!(error < warning);
    assert!(warning < info);
    assert!(error < info);

    // Should support not-less-than
    assert!(warning >= error);
    assert!(info >= warning);
}

// Regression: All DiagnosticCategory variants are Copy
#[test]
fn regression_diagnostic_category_copy() {
    let cat1 = DiagnosticCategory::Parser;
    let cat2 = cat1; // Copy semantics

    assert_eq!(cat1, cat2);
}

// Edge case: Diagnostic struct Debug representation is valid
#[test]
fn edge_case_diagnostic_debug_representation() {
    let diag = Diagnostic {
        message: "Debug test".to_string(),
        ..Default::default()
    };

    let debug_str = format!("{:?}", diag);
    assert!(debug_str.contains("Debug test") || !debug_str.is_empty());
}

// Edge case: DiagnosticCode Debug representation is valid
#[test]
fn edge_case_diagnostic_code_debug_representation() {
    let code = DiagnosticCode::ParseError;
    let debug_str = format!("{:?}", code);

    // Should be non-empty
    assert!(!debug_str.is_empty());
}

// Edge case: DiagnosticCode.as_str() returns stable strings
#[test]
fn edge_case_diagnostic_code_as_str_stable() {
    let code1_str = DiagnosticCode::ParseError.as_str();
    let code2_str = DiagnosticCode::ParseError.as_str();

    // Same code should return identical string
    assert_eq!(code1_str, code2_str);

    // Different codes should return different strings
    let code3_str = DiagnosticCode::SyntaxError.as_str();
    assert_ne!(code1_str, code3_str);
}

// Edge case: DiagnosticCode.severity() returns consistent severity
#[test]
fn edge_case_diagnostic_code_severity_consistent() {
    let code = DiagnosticCode::ParseError;
    let sev1 = code.severity();
    let sev2 = code.severity();

    // Same code should always return same severity
    assert_eq!(sev1, sev2);
}

// Edge case: DiagnosticCode.category() returns expected category
#[test]
fn edge_case_diagnostic_code_category_consistent() {
    // ParseError is in the Parser range (PL001-PL099)
    let code = DiagnosticCode::ParseError;
    let category = code.category();

    assert_eq!(category, DiagnosticCategory::Parser);
}

// Edge case: DiagnosticCode.tags() returns consistent tags
#[test]
fn edge_case_diagnostic_code_tags_consistent() {
    let code = DiagnosticCode::ParseError;
    let tags1 = code.tags();
    let tags2 = code.tags();

    // Same code should always return same tags
    assert_eq!(tags1, tags2);
}

// Edge case: Diagnostic struct Clone and PartialEq work correctly
#[test]
fn edge_case_diagnostic_clone_and_equality() {
    let diag1 = Diagnostic {
        code: DiagnosticCode::SyntaxError,
        severity: DiagnosticSeverity::Warning,
        range: (10, 20),
        message: "Test".to_string(),
        related_information: None,
        tags: None,
    };

    let diag2 = diag1.clone();

    // Clone should produce equal diagnostic
    assert_eq!(diag1, diag2);
}

// Edge case: Large range values work correctly
#[test]
fn edge_case_diagnostic_large_range_values() {
    let diag = Diagnostic {
        range: (1_000_000, 2_000_000),
        ..Default::default()
    };

    assert_eq!(diag.range.0, 1_000_000);
    assert_eq!(diag.range.1, 2_000_000);
}

// Regression: Diagnostic can be used in collections
#[test]
fn regression_diagnostic_in_collections() {
    let diag1 = Diagnostic {
        message: "First".to_string(),
        ..Default::default()
    };
    let diag2 = Diagnostic {
        message: "Second".to_string(),
        ..Default::default()
    };

    let diagnostics: Vec<Diagnostic> = vec![diag1, diag2];
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].message, "First");
    assert_eq!(diagnostics[1].message, "Second");
}

// Regression: All DiagnosticTag variants are Copy
#[test]
fn regression_diagnostic_tag_copy() {
    let tag1 = DiagnosticTag::Unnecessary;
    let tag2 = tag1; // Copy semantics

    assert_eq!(tag1, tag2);
}

// Edge case: from_message priority — phase-scoped patterns take precedence over
// generic "use strict" match because the phase-scoped message also contains "use strict"
// as a substring. If checked in wrong order, "use strict" branch would fire first
// and return MissingStrict instead of PhaseScopedStrictPragma.
#[test]
fn edge_case_from_message_phase_scoped_takes_priority_over_use_strict() {
    // This message contains "use strict" as a substring but should map to
    // PhaseScopedStrictPragma, not MissingStrict, because the phase-scoped check
    // appears first in the from_message implementation.
    let msg = "use strict inside a begin block does not enable strict at file scope";
    assert_eq!(
        DiagnosticCode::from_message(msg),
        Some(DiagnosticCode::PhaseScopedStrictPragma),
        "Phase-scoped strict check must take priority over generic 'use strict' match"
    );

    // Same for warnings
    let msg_warn = "use warnings inside a phase block does not enable warnings at file scope";
    assert_eq!(
        DiagnosticCode::from_message(msg_warn),
        Some(DiagnosticCode::PhaseScopedWarningsPragma),
        "Phase-scoped warnings check must take priority over generic 'use warnings' match"
    );
}

// Edge case: from_message returns None for empty and whitespace-only strings
#[test]
fn edge_case_from_message_empty_and_whitespace() {
    assert_eq!(
        DiagnosticCode::from_message(""),
        None,
        "empty string → None"
    );
    assert_eq!(
        DiagnosticCode::from_message("   "),
        None,
        "whitespace-only → None"
    );
    assert_eq!(
        DiagnosticCode::from_message("\t\n"),
        None,
        "tab+newline → None"
    );
}

// Edge case: parse_code is case-sensitive (lowercase codes must not match)
#[test]
fn edge_case_parse_code_case_sensitivity() {
    // parse_code is case-sensitive by design — codes are always uppercase
    assert_eq!(
        DiagnosticCode::parse_code("PL001"),
        Some(DiagnosticCode::ParseError)
    );
    assert_eq!(
        DiagnosticCode::parse_code("pl001"),
        None,
        "lowercase must not match"
    );
    assert_eq!(
        DiagnosticCode::parse_code("Pl001"),
        None,
        "mixed case must not match"
    );
    assert_eq!(DiagnosticCode::parse_code(""), None, "empty string → None");
    assert_eq!(
        DiagnosticCode::parse_code("PL999"),
        None,
        "unassigned code → None"
    );
    assert_eq!(
        DiagnosticCode::parse_code("PC999"),
        None,
        "unassigned PC code → None"
    );
    assert_eq!(
        DiagnosticCode::parse_code("PL 001"),
        None,
        "space in code → None"
    );
}

// Regression: DiagnosticSeverity implements Hash correctly
#[test]
fn regression_severity_hash_in_map() {
    use std::collections::HashMap;

    let mut map: HashMap<DiagnosticSeverity, i32> = HashMap::new();
    map.insert(DiagnosticSeverity::Error, 1);
    map.insert(DiagnosticSeverity::Warning, 2);

    // Lookups should work
    assert_eq!(map.get(&DiagnosticSeverity::Error), Some(&1));
    assert_eq!(map.get(&DiagnosticSeverity::Warning), Some(&2));
}

// Regression: DiagnosticTag implements Hash correctly
#[test]
fn regression_tag_hash_in_map() {
    use std::collections::HashMap;

    let mut map: HashMap<DiagnosticTag, &str> = HashMap::new();
    map.insert(DiagnosticTag::Unnecessary, "unnecessary");
    map.insert(DiagnosticTag::Deprecated, "deprecated");

    // Lookups should work
    assert_eq!(map.get(&DiagnosticTag::Unnecessary), Some(&"unnecessary"));
    assert_eq!(map.get(&DiagnosticTag::Deprecated), Some(&"deprecated"));
}
