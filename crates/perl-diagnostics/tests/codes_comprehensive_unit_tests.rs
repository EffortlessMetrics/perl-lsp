//! Comprehensive unit tests for perl-diagnostics-codes crate.
//!
//! Covers every variant and method of:
//! - DiagnosticSeverity
//! - DiagnosticTag
//! - DiagnosticCode
//! - DiagnosticCategory

use perl_diagnostics::codes::{
    DiagnosticCategory, DiagnosticCode, DiagnosticSeverity, DiagnosticTag,
};

// ---------------------------------------------------------------------------
// Helper: all DiagnosticCode variants for exhaustive iteration
// ---------------------------------------------------------------------------

const ALL_CODES: &[DiagnosticCode] = &[
    DiagnosticCode::ParseError,
    DiagnosticCode::SyntaxError,
    DiagnosticCode::UnexpectedEof,
    DiagnosticCode::MissingStrict,
    DiagnosticCode::MissingWarnings,
    DiagnosticCode::PhaseScopedStrictPragma,
    DiagnosticCode::PhaseScopedWarningsPragma,
    DiagnosticCode::UnusedVariable,
    DiagnosticCode::UndefinedVariable,
    DiagnosticCode::CaptureVarWithoutRegexMatch,
    DiagnosticCode::MissingPackageDeclaration,
    DiagnosticCode::DuplicatePackage,
    DiagnosticCode::DuplicateSubroutine,
    DiagnosticCode::MissingReturn,
    DiagnosticCode::RoleConflict,
    DiagnosticCode::InvalidPrototype,
    DiagnosticCode::BarewordFilehandle,
    DiagnosticCode::TwoArgOpen,
    DiagnosticCode::ImplicitReturn,
    DiagnosticCode::PrintfFormatMismatch,
    DiagnosticCode::SecuritySignalHandler,
    DiagnosticCode::CriticSeverity1,
    DiagnosticCode::CriticSeverity2,
    DiagnosticCode::CriticSeverity3,
    DiagnosticCode::CriticSeverity4,
    DiagnosticCode::CriticSeverity5,
];

// ===========================================================================
// DiagnosticSeverity tests
// ===========================================================================

#[test]
fn severity_to_lsp_value_error() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticSeverity::Error.to_lsp_value(), 1);
    Ok(())
}

#[test]
fn severity_to_lsp_value_warning() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticSeverity::Warning.to_lsp_value(), 2);
    Ok(())
}

#[test]
fn severity_to_lsp_value_information() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticSeverity::Information.to_lsp_value(), 3);
    Ok(())
}

#[test]
fn severity_to_lsp_value_hint() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticSeverity::Hint.to_lsp_value(), 4);
    Ok(())
}

#[test]
fn severity_display() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(format!("{}", DiagnosticSeverity::Error), "error");
    assert_eq!(format!("{}", DiagnosticSeverity::Warning), "warning");
    assert_eq!(format!("{}", DiagnosticSeverity::Information), "info");
    assert_eq!(format!("{}", DiagnosticSeverity::Hint), "hint");
    Ok(())
}

#[test]
fn severity_clone_and_copy() -> Result<(), Box<dyn std::error::Error>> {
    let s = DiagnosticSeverity::Error;
    let cloned = s;
    let copied = s;
    assert_eq!(s, cloned);
    assert_eq!(s, copied);
    Ok(())
}

#[test]
fn severity_ordering() -> Result<(), Box<dyn std::error::Error>> {
    // Error(1) < Warning(2) < Information(3) < Hint(4) per repr(u8)
    assert!(DiagnosticSeverity::Error < DiagnosticSeverity::Warning);
    assert!(DiagnosticSeverity::Warning < DiagnosticSeverity::Information);
    assert!(DiagnosticSeverity::Information < DiagnosticSeverity::Hint);
    Ok(())
}

#[test]
fn severity_debug() -> Result<(), Box<dyn std::error::Error>> {
    let dbg = format!("{:?}", DiagnosticSeverity::Error);
    assert!(dbg.contains("Error"));
    Ok(())
}

#[test]
fn severity_hash_consistency() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(DiagnosticSeverity::Error);
    set.insert(DiagnosticSeverity::Error);
    assert_eq!(set.len(), 1);
    set.insert(DiagnosticSeverity::Warning);
    assert_eq!(set.len(), 2);
    Ok(())
}

// ===========================================================================
// DiagnosticTag tests
// ===========================================================================

#[test]
fn tag_to_lsp_value_unnecessary() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticTag::Unnecessary.to_lsp_value(), 1);
    Ok(())
}

#[test]
fn tag_to_lsp_value_deprecated() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticTag::Deprecated.to_lsp_value(), 2);
    Ok(())
}

#[test]
fn tag_clone_and_copy() -> Result<(), Box<dyn std::error::Error>> {
    let t = DiagnosticTag::Unnecessary;
    let cloned = t;
    let copied = t;
    assert_eq!(t, cloned);
    assert_eq!(t, copied);
    Ok(())
}

#[test]
fn tag_debug() -> Result<(), Box<dyn std::error::Error>> {
    assert!(format!("{:?}", DiagnosticTag::Unnecessary).contains("Unnecessary"));
    assert!(format!("{:?}", DiagnosticTag::Deprecated).contains("Deprecated"));
    Ok(())
}

#[test]
fn tag_hash_consistency() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(DiagnosticTag::Unnecessary);
    set.insert(DiagnosticTag::Unnecessary);
    assert_eq!(set.len(), 1);
    set.insert(DiagnosticTag::Deprecated);
    assert_eq!(set.len(), 2);
    Ok(())
}

// ===========================================================================
// DiagnosticCode::as_str — exhaustive
// ===========================================================================

#[test]
fn code_as_str_parser_range() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::ParseError.as_str(), "PL001");
    assert_eq!(DiagnosticCode::SyntaxError.as_str(), "PL002");
    assert_eq!(DiagnosticCode::UnexpectedEof.as_str(), "PL003");
    Ok(())
}

#[test]
fn code_as_str_strict_warnings_range() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::MissingStrict.as_str(), "PL100");
    assert_eq!(DiagnosticCode::MissingWarnings.as_str(), "PL101");
    assert_eq!(DiagnosticCode::UnusedVariable.as_str(), "PL102");
    assert_eq!(DiagnosticCode::UndefinedVariable.as_str(), "PL103");
    assert_eq!(
        DiagnosticCode::CaptureVarWithoutRegexMatch.as_str(),
        "PL112"
    );
    Ok(())
}

#[test]
fn code_as_str_phase_scoped_pragma_range() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::PhaseScopedStrictPragma.as_str(), "PL502");
    assert_eq!(DiagnosticCode::PhaseScopedWarningsPragma.as_str(), "PL503");
    Ok(())
}

#[test]
fn code_as_str_package_module_range() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::MissingPackageDeclaration.as_str(), "PL200");
    assert_eq!(DiagnosticCode::DuplicatePackage.as_str(), "PL201");
    Ok(())
}

#[test]
fn code_as_str_subroutine_range() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::DuplicateSubroutine.as_str(), "PL300");
    assert_eq!(DiagnosticCode::MissingReturn.as_str(), "PL301");
    assert_eq!(DiagnosticCode::InvalidPrototype.as_str(), "PL302");
    assert_eq!(DiagnosticCode::RoleConflict.as_str(), "PL303");
    Ok(())
}

#[test]
fn code_as_str_best_practices_range() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::BarewordFilehandle.as_str(), "PL400");
    assert_eq!(DiagnosticCode::TwoArgOpen.as_str(), "PL401");
    assert_eq!(DiagnosticCode::ImplicitReturn.as_str(), "PL402");
    Ok(())
}

#[test]
fn code_as_str_critic_range() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::CriticSeverity1.as_str(), "PC001");
    assert_eq!(DiagnosticCode::CriticSeverity2.as_str(), "PC002");
    assert_eq!(DiagnosticCode::CriticSeverity3.as_str(), "PC003");
    assert_eq!(DiagnosticCode::CriticSeverity4.as_str(), "PC004");
    assert_eq!(DiagnosticCode::CriticSeverity5.as_str(), "PC005");
    Ok(())
}

// ===========================================================================
// DiagnosticCode::Display
// ===========================================================================

#[test]
fn code_display_matches_as_str() -> Result<(), Box<dyn std::error::Error>> {
    for code in ALL_CODES {
        assert_eq!(format!("{code}"), code.as_str());
    }
    Ok(())
}

// ===========================================================================
// DiagnosticCode::parse_code — exhaustive round-trip
// ===========================================================================

#[test]
fn parse_code_round_trip_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    for code in ALL_CODES {
        let parsed = DiagnosticCode::parse_code(code.as_str());
        assert_eq!(
            parsed,
            Some(*code),
            "round-trip failed for {}",
            code.as_str()
        );
    }
    Ok(())
}

#[test]
fn parse_code_invalid_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::parse_code("INVALID"), None);
    assert_eq!(DiagnosticCode::parse_code(""), None);
    assert_eq!(DiagnosticCode::parse_code("PL999"), None);
    assert_eq!(DiagnosticCode::parse_code("PC999"), None);
    assert_eq!(DiagnosticCode::parse_code("pl001"), None); // case sensitive
    assert_eq!(DiagnosticCode::parse_code("PL 001"), None); // spaces
    Ok(())
}

// ===========================================================================
// DiagnosticCode::severity — exhaustive
// ===========================================================================

#[test]
fn severity_error_codes() -> Result<(), Box<dyn std::error::Error>> {
    let error_codes = [
        DiagnosticCode::ParseError,
        DiagnosticCode::SyntaxError,
        DiagnosticCode::UnexpectedEof,
        DiagnosticCode::UndefinedVariable,
    ];
    for code in &error_codes {
        assert_eq!(
            code.severity(),
            DiagnosticSeverity::Error,
            "{} should be Error severity",
            code.as_str()
        );
    }
    Ok(())
}

#[test]
fn severity_warning_codes() -> Result<(), Box<dyn std::error::Error>> {
    let warning_codes = [
        DiagnosticCode::MissingStrict,
        DiagnosticCode::MissingWarnings,
        DiagnosticCode::UnusedVariable,
        DiagnosticCode::MissingPackageDeclaration,
        DiagnosticCode::DuplicatePackage,
        DiagnosticCode::DuplicateSubroutine,
        DiagnosticCode::MissingReturn,
        DiagnosticCode::RoleConflict,
        DiagnosticCode::BarewordFilehandle,
        DiagnosticCode::TwoArgOpen,
        DiagnosticCode::ImplicitReturn,
        DiagnosticCode::CriticSeverity1,
        DiagnosticCode::CriticSeverity2,
    ];
    for code in &warning_codes {
        assert_eq!(
            code.severity(),
            DiagnosticSeverity::Warning,
            "{} should be Warning severity",
            code.as_str()
        );
    }
    Ok(())
}

#[test]
fn severity_information_codes() -> Result<(), Box<dyn std::error::Error>> {
    let information_codes = [DiagnosticCode::CaptureVarWithoutRegexMatch];
    for code in &information_codes {
        assert_eq!(
            code.severity(),
            DiagnosticSeverity::Information,
            "{} should be Information severity",
            code.as_str()
        );
    }
    Ok(())
}

#[test]
fn severity_hint_codes() -> Result<(), Box<dyn std::error::Error>> {
    let hint_codes = [
        DiagnosticCode::CriticSeverity3,
        DiagnosticCode::CriticSeverity4,
        DiagnosticCode::CriticSeverity5,
    ];
    for code in &hint_codes {
        assert_eq!(
            code.severity(),
            DiagnosticSeverity::Hint,
            "{} should be Hint severity",
            code.as_str()
        );
    }
    Ok(())
}

// ===========================================================================
// DiagnosticCode::documentation_url — exhaustive
// ===========================================================================

#[test]
fn documentation_url_pl_codes_have_urls() -> Result<(), Box<dyn std::error::Error>> {
    let pl_codes = [
        (DiagnosticCode::ParseError, "PL001"),
        (DiagnosticCode::SyntaxError, "PL002"),
        (DiagnosticCode::UnexpectedEof, "PL003"),
        (DiagnosticCode::MissingStrict, "PL100"),
        (DiagnosticCode::MissingWarnings, "PL101"),
        (DiagnosticCode::UnusedVariable, "PL102"),
        (DiagnosticCode::UndefinedVariable, "PL103"),
        (DiagnosticCode::CaptureVarWithoutRegexMatch, "PL112"),
        (DiagnosticCode::MissingPackageDeclaration, "PL200"),
        (DiagnosticCode::DuplicatePackage, "PL201"),
        (DiagnosticCode::DuplicateSubroutine, "PL300"),
        (DiagnosticCode::MissingReturn, "PL301"),
        (DiagnosticCode::InvalidPrototype, "PL302"),
        (DiagnosticCode::RoleConflict, "PL303"),
        (DiagnosticCode::BarewordFilehandle, "PL400"),
        (DiagnosticCode::TwoArgOpen, "PL401"),
        (DiagnosticCode::ImplicitReturn, "PL402"),
    ];
    for (code, expected_suffix) in &pl_codes {
        let url = code.documentation_url();
        assert!(
            url.is_some(),
            "{} should have a documentation URL",
            code.as_str()
        );
        let url_str = url.ok_or("missing url")?;
        assert!(
            url_str.ends_with(expected_suffix),
            "URL for {} should end with {}, got {}",
            code.as_str(),
            expected_suffix,
            url_str,
        );
        assert!(
            url_str.starts_with("https://docs.perl-lsp.org/errors/"),
            "URL for {} should start with base URL, got {}",
            code.as_str(),
            url_str,
        );
    }
    Ok(())
}

#[test]
fn documentation_url_critic_codes_return_none() -> Result<(), Box<dyn std::error::Error>> {
    let critic_codes = [
        DiagnosticCode::CriticSeverity1,
        DiagnosticCode::CriticSeverity2,
        DiagnosticCode::CriticSeverity3,
        DiagnosticCode::CriticSeverity4,
        DiagnosticCode::CriticSeverity5,
    ];
    for code in &critic_codes {
        assert_eq!(
            code.documentation_url(),
            None,
            "{} should not have a documentation URL",
            code.as_str()
        );
    }
    Ok(())
}

// ===========================================================================
// DiagnosticCode::tags — exhaustive
// ===========================================================================

#[test]
fn tags_unused_variable_has_unnecessary() -> Result<(), Box<dyn std::error::Error>> {
    let tags = DiagnosticCode::UnusedVariable.tags();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0], DiagnosticTag::Unnecessary);
    Ok(())
}

#[test]
fn tags_all_other_codes_empty() -> Result<(), Box<dyn std::error::Error>> {
    for code in ALL_CODES {
        if *code == DiagnosticCode::UnusedVariable {
            continue;
        }
        assert!(
            code.tags().is_empty(),
            "{} should have empty tags but has {:?}",
            code.as_str(),
            code.tags()
        );
    }
    Ok(())
}

// ===========================================================================
// DiagnosticCode::category — exhaustive
// ===========================================================================

#[test]
fn category_parser() -> Result<(), Box<dyn std::error::Error>> {
    let parser_codes = [
        DiagnosticCode::ParseError,
        DiagnosticCode::SyntaxError,
        DiagnosticCode::UnexpectedEof,
    ];
    for code in &parser_codes {
        assert_eq!(
            code.category(),
            DiagnosticCategory::Parser,
            "{} should be Parser category",
            code.as_str()
        );
    }
    Ok(())
}

#[test]
fn category_strict_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let sw_codes = [
        DiagnosticCode::MissingStrict,
        DiagnosticCode::MissingWarnings,
        DiagnosticCode::PhaseScopedStrictPragma,
        DiagnosticCode::PhaseScopedWarningsPragma,
        DiagnosticCode::UnusedVariable,
        DiagnosticCode::UndefinedVariable,
        DiagnosticCode::CaptureVarWithoutRegexMatch,
    ];
    for code in &sw_codes {
        assert_eq!(
            code.category(),
            DiagnosticCategory::StrictWarnings,
            "{} should be StrictWarnings category",
            code.as_str()
        );
    }
    Ok(())
}

#[test]
fn category_package_module() -> Result<(), Box<dyn std::error::Error>> {
    let pm_codes = [
        DiagnosticCode::MissingPackageDeclaration,
        DiagnosticCode::DuplicatePackage,
    ];
    for code in &pm_codes {
        assert_eq!(
            code.category(),
            DiagnosticCategory::PackageModule,
            "{} should be PackageModule category",
            code.as_str()
        );
    }
    Ok(())
}

#[test]
fn category_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    let sub_codes = [
        DiagnosticCode::DuplicateSubroutine,
        DiagnosticCode::MissingReturn,
        DiagnosticCode::RoleConflict,
    ];
    for code in &sub_codes {
        assert_eq!(
            code.category(),
            DiagnosticCategory::Subroutine,
            "{} should be Subroutine category",
            code.as_str()
        );
    }
    Ok(())
}

#[test]
fn category_best_practices() -> Result<(), Box<dyn std::error::Error>> {
    let bp_codes = [
        DiagnosticCode::BarewordFilehandle,
        DiagnosticCode::TwoArgOpen,
        DiagnosticCode::ImplicitReturn,
    ];
    for code in &bp_codes {
        assert_eq!(
            code.category(),
            DiagnosticCategory::BestPractices,
            "{} should be BestPractices category",
            code.as_str()
        );
    }
    Ok(())
}

#[test]
fn category_perl_critic() -> Result<(), Box<dyn std::error::Error>> {
    let critic_codes = [
        DiagnosticCode::CriticSeverity1,
        DiagnosticCode::CriticSeverity2,
        DiagnosticCode::CriticSeverity3,
        DiagnosticCode::CriticSeverity4,
        DiagnosticCode::CriticSeverity5,
    ];
    for code in &critic_codes {
        assert_eq!(
            code.category(),
            DiagnosticCategory::PerlCritic,
            "{} should be PerlCritic category",
            code.as_str()
        );
    }
    Ok(())
}

// ===========================================================================
// DiagnosticCode::from_message — positive and negative cases
// ===========================================================================

#[test]
fn from_message_use_strict() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        DiagnosticCode::from_message("Missing 'use strict' pragma"),
        Some(DiagnosticCode::MissingStrict)
    );
    // Case-insensitive
    assert_eq!(
        DiagnosticCode::from_message("USE STRICT is missing"),
        Some(DiagnosticCode::MissingStrict)
    );
    Ok(())
}

#[test]
fn from_message_use_warnings() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        DiagnosticCode::from_message("Missing 'use warnings' pragma"),
        Some(DiagnosticCode::MissingWarnings)
    );
    assert_eq!(
        DiagnosticCode::from_message("File lacks USE WARNINGS"),
        Some(DiagnosticCode::MissingWarnings)
    );
    Ok(())
}

#[test]
fn from_message_unused_variable() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        DiagnosticCode::from_message("Unused variable $foo"),
        Some(DiagnosticCode::UnusedVariable)
    );
    assert_eq!(
        DiagnosticCode::from_message("$bar is never used"),
        Some(DiagnosticCode::UnusedVariable)
    );
    Ok(())
}

#[test]
fn from_message_undefined_variable() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        DiagnosticCode::from_message("Global symbol $x is undefined"),
        Some(DiagnosticCode::UndefinedVariable)
    );
    assert_eq!(
        DiagnosticCode::from_message("Variable $y not declared"),
        Some(DiagnosticCode::UndefinedVariable)
    );
    Ok(())
}

#[test]
fn from_message_bareword_filehandle() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        DiagnosticCode::from_message("Bareword filehandle detected"),
        Some(DiagnosticCode::BarewordFilehandle)
    );
    Ok(())
}

#[test]
fn from_message_two_arg_open() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        DiagnosticCode::from_message("Two-argument open() used"),
        Some(DiagnosticCode::TwoArgOpen)
    );
    assert_eq!(
        DiagnosticCode::from_message("Found 2-arg open"),
        Some(DiagnosticCode::TwoArgOpen)
    );
    Ok(())
}

#[test]
fn from_message_parse_error() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        DiagnosticCode::from_message("parse error near line 5"),
        Some(DiagnosticCode::ParseError)
    );
    assert_eq!(
        DiagnosticCode::from_message("Syntax error at line 10"),
        Some(DiagnosticCode::ParseError)
    );
    Ok(())
}

#[test]
fn from_message_returns_none_for_unrecognized() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::from_message(""), None);
    assert_eq!(
        DiagnosticCode::from_message("something else entirely"),
        None
    );
    assert_eq!(
        DiagnosticCode::from_message("implicit return detected"),
        None
    );
    Ok(())
}

// ===========================================================================
// DiagnosticCode trait derivations
// ===========================================================================

#[test]
fn code_clone_and_copy() -> Result<(), Box<dyn std::error::Error>> {
    let c = DiagnosticCode::ParseError;
    let cloned = c;
    let copied = c;
    assert_eq!(c, cloned);
    assert_eq!(c, copied);
    Ok(())
}

#[test]
fn code_debug_contains_variant_name() -> Result<(), Box<dyn std::error::Error>> {
    assert!(format!("{:?}", DiagnosticCode::ParseError).contains("ParseError"));
    assert!(format!("{:?}", DiagnosticCode::CriticSeverity5).contains("CriticSeverity5"));
    Ok(())
}

#[test]
fn code_hash_consistency() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    for code in ALL_CODES {
        set.insert(*code);
    }
    assert_eq!(set.len(), ALL_CODES.len());
    // Re-inserting shouldn't change length
    for code in ALL_CODES {
        set.insert(*code);
    }
    assert_eq!(set.len(), ALL_CODES.len());
    Ok(())
}

#[test]
fn code_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::ParseError, DiagnosticCode::ParseError);
    assert_ne!(DiagnosticCode::ParseError, DiagnosticCode::SyntaxError);
    Ok(())
}

// ===========================================================================
// DiagnosticCategory trait derivations
// ===========================================================================

#[test]
fn category_clone_and_copy() -> Result<(), Box<dyn std::error::Error>> {
    let c = DiagnosticCategory::Parser;
    let cloned = c;
    let copied = c;
    assert_eq!(c, cloned);
    assert_eq!(c, copied);
    Ok(())
}

#[test]
fn category_debug_contains_variant_name() -> Result<(), Box<dyn std::error::Error>> {
    assert!(format!("{:?}", DiagnosticCategory::Parser).contains("Parser"));
    assert!(format!("{:?}", DiagnosticCategory::PerlCritic).contains("PerlCritic"));
    Ok(())
}

#[test]
fn category_hash_consistency() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(DiagnosticCategory::Parser);
    set.insert(DiagnosticCategory::Parser);
    assert_eq!(set.len(), 1);
    set.insert(DiagnosticCategory::StrictWarnings);
    set.insert(DiagnosticCategory::PackageModule);
    set.insert(DiagnosticCategory::Subroutine);
    set.insert(DiagnosticCategory::BestPractices);
    set.insert(DiagnosticCategory::PerlCritic);
    assert_eq!(set.len(), 6);
    Ok(())
}

// ===========================================================================
// Cross-cutting: code string uniqueness
// ===========================================================================

#[test]
fn all_code_strings_are_unique() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for code in ALL_CODES {
        let s = code.as_str();
        assert!(
            seen.insert(s),
            "Duplicate code string: {} (variant {:?})",
            s,
            code,
        );
    }
    Ok(())
}

// ===========================================================================
// Cross-cutting: every code maps to exactly one category
// ===========================================================================

#[test]
fn every_code_has_a_category() -> Result<(), Box<dyn std::error::Error>> {
    for code in ALL_CODES {
        // Just verify it doesn't panic and returns a valid category
        let _cat = code.category();
    }
    Ok(())
}

// ===========================================================================
// Cross-cutting: every code maps to exactly one severity
// ===========================================================================

#[test]
fn every_code_has_a_severity() -> Result<(), Box<dyn std::error::Error>> {
    for code in ALL_CODES {
        let sev = code.severity();
        let lsp_val = sev.to_lsp_value();
        assert!(
            (1..=4).contains(&lsp_val),
            "{} has out-of-range LSP severity {}",
            code.as_str(),
            lsp_val,
        );
    }
    Ok(())
}

// ===========================================================================
// Cross-cutting: parse_code → as_str consistency for all codes
// ===========================================================================

#[test]
fn parse_code_as_str_bijection() -> Result<(), Box<dyn std::error::Error>> {
    let code_strings: Vec<&str> = ALL_CODES.iter().map(|c| c.as_str()).collect();
    for s in &code_strings {
        let parsed = DiagnosticCode::parse_code(s);
        assert!(parsed.is_some(), "parse_code should accept {}", s);
        assert_eq!(
            parsed.ok_or("missing")?.as_str(),
            *s,
            "round-trip mismatch for {}",
            s,
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Additional comprehensive tests
// ---------------------------------------------------------------------------

// --- DiagnosticSeverity additional coverage ---

#[test]
fn severity_equality_same_variant() {
    assert_eq!(DiagnosticSeverity::Error, DiagnosticSeverity::Error);
    assert_eq!(DiagnosticSeverity::Warning, DiagnosticSeverity::Warning);
    assert_eq!(
        DiagnosticSeverity::Information,
        DiagnosticSeverity::Information
    );
    assert_eq!(DiagnosticSeverity::Hint, DiagnosticSeverity::Hint);
}

#[test]
fn severity_inequality_different_variants() {
    assert_ne!(DiagnosticSeverity::Error, DiagnosticSeverity::Warning);
    assert_ne!(DiagnosticSeverity::Warning, DiagnosticSeverity::Information);
    assert_ne!(DiagnosticSeverity::Information, DiagnosticSeverity::Hint);
    assert_ne!(DiagnosticSeverity::Error, DiagnosticSeverity::Hint);
}

#[test]
fn severity_ordering_is_ascending_by_lsp_value() {
    // Error(1) < Warning(2) < Information(3) < Hint(4)
    assert!(DiagnosticSeverity::Error < DiagnosticSeverity::Warning);
    assert!(DiagnosticSeverity::Warning < DiagnosticSeverity::Information);
    assert!(DiagnosticSeverity::Information < DiagnosticSeverity::Hint);
}

#[test]
fn severity_lsp_values_cover_1_through_4() {
    let values = [
        DiagnosticSeverity::Error.to_lsp_value(),
        DiagnosticSeverity::Warning.to_lsp_value(),
        DiagnosticSeverity::Information.to_lsp_value(),
        DiagnosticSeverity::Hint.to_lsp_value(),
    ];
    assert_eq!(values, [1, 2, 3, 4]);
}

#[test]
fn severity_display_all_variants() {
    assert_eq!(format!("{}", DiagnosticSeverity::Error), "error");
    assert_eq!(format!("{}", DiagnosticSeverity::Warning), "warning");
    assert_eq!(format!("{}", DiagnosticSeverity::Information), "info");
    assert_eq!(format!("{}", DiagnosticSeverity::Hint), "hint");
}

#[test]
fn severity_information_lsp_value_is_3() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticSeverity::Information.to_lsp_value(), 3);
    Ok(())
}

// --- DiagnosticTag additional coverage ---

#[test]
fn tag_equality_and_inequality() {
    assert_eq!(DiagnosticTag::Unnecessary, DiagnosticTag::Unnecessary);
    assert_eq!(DiagnosticTag::Deprecated, DiagnosticTag::Deprecated);
    assert_ne!(DiagnosticTag::Unnecessary, DiagnosticTag::Deprecated);
}

#[test]
fn tag_lsp_values_are_distinct() {
    assert_ne!(
        DiagnosticTag::Unnecessary.to_lsp_value(),
        DiagnosticTag::Deprecated.to_lsp_value()
    );
}

// --- DiagnosticCode: code string prefix validation ---

#[test]
fn parser_codes_start_with_pl_and_are_below_100() -> Result<(), Box<dyn std::error::Error>> {
    let parser_codes = [
        DiagnosticCode::ParseError,
        DiagnosticCode::SyntaxError,
        DiagnosticCode::UnexpectedEof,
    ];
    for code in &parser_codes {
        let s = code.as_str();
        assert!(
            s.starts_with("PL"),
            "parser code should start with PL: {}",
            s
        );
        let num: u32 = s[2..]
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        assert!((1..=99).contains(&num), "parser code out of range: {}", s);
    }
    Ok(())
}

#[test]
fn strict_warnings_codes_are_in_100_199_range() -> Result<(), Box<dyn std::error::Error>> {
    let codes = [
        DiagnosticCode::MissingStrict,
        DiagnosticCode::MissingWarnings,
        DiagnosticCode::UnusedVariable,
        DiagnosticCode::UndefinedVariable,
    ];
    for code in &codes {
        let s = code.as_str();
        let num: u32 = s[2..]
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        assert!(
            (100..200).contains(&num),
            "strict/warnings code out of range: {}",
            s
        );
    }
    Ok(())
}

#[test]
fn package_module_codes_are_in_200_299_range() -> Result<(), Box<dyn std::error::Error>> {
    let codes = [
        DiagnosticCode::MissingPackageDeclaration,
        DiagnosticCode::DuplicatePackage,
    ];
    for code in &codes {
        let s = code.as_str();
        let num: u32 = s[2..]
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        assert!(
            (200..300).contains(&num),
            "package/module code out of range: {}",
            s
        );
    }
    Ok(())
}

#[test]
fn subroutine_codes_are_in_300_399_range() -> Result<(), Box<dyn std::error::Error>> {
    let codes = [
        DiagnosticCode::DuplicateSubroutine,
        DiagnosticCode::MissingReturn,
        DiagnosticCode::RoleConflict,
    ];
    for code in &codes {
        let s = code.as_str();
        let num: u32 = s[2..]
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        assert!(
            (300..400).contains(&num),
            "subroutine code out of range: {}",
            s
        );
    }
    Ok(())
}

#[test]
fn best_practices_codes_are_in_400_499_range() -> Result<(), Box<dyn std::error::Error>> {
    let codes = [
        DiagnosticCode::BarewordFilehandle,
        DiagnosticCode::TwoArgOpen,
        DiagnosticCode::ImplicitReturn,
    ];
    for code in &codes {
        let s = code.as_str();
        let num: u32 = s[2..]
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        assert!(
            (400..500).contains(&num),
            "best practices code out of range: {}",
            s
        );
    }
    Ok(())
}

#[test]
fn critic_codes_start_with_pc_prefix() {
    let critic_codes = [
        DiagnosticCode::CriticSeverity1,
        DiagnosticCode::CriticSeverity2,
        DiagnosticCode::CriticSeverity3,
        DiagnosticCode::CriticSeverity4,
        DiagnosticCode::CriticSeverity5,
    ];
    for code in &critic_codes {
        assert!(
            code.as_str().starts_with("PC"),
            "critic code should start with PC: {}",
            code.as_str()
        );
    }
}

// --- DiagnosticCode: parse_code edge cases ---

#[test]
fn parse_code_empty_string_returns_none() {
    assert!(DiagnosticCode::parse_code("").is_none());
}

#[test]
fn parse_code_lowercase_returns_none() {
    assert!(DiagnosticCode::parse_code("pl001").is_none());
    assert!(DiagnosticCode::parse_code("pc001").is_none());
}

#[test]
fn parse_code_with_whitespace_returns_none() {
    assert!(DiagnosticCode::parse_code(" PL001").is_none());
    assert!(DiagnosticCode::parse_code("PL001 ").is_none());
    assert!(DiagnosticCode::parse_code("PL 001").is_none());
}

#[test]
fn parse_code_partial_prefix_returns_none() {
    assert!(DiagnosticCode::parse_code("PL").is_none());
    assert!(DiagnosticCode::parse_code("PC").is_none());
    assert!(DiagnosticCode::parse_code("PL0").is_none());
}

#[test]
fn parse_code_out_of_range_returns_none() {
    assert!(DiagnosticCode::parse_code("PL000").is_none());
    assert!(DiagnosticCode::parse_code("PL999").is_none());
    assert!(DiagnosticCode::parse_code("PC000").is_none());
    assert!(DiagnosticCode::parse_code("PC999").is_none());
}

#[test]
fn parse_code_wrong_prefix_returns_none() {
    assert!(DiagnosticCode::parse_code("XX001").is_none());
    assert!(DiagnosticCode::parse_code("AB100").is_none());
}

// --- DiagnosticCode: from_message edge cases ---

#[test]
fn from_message_case_insensitive() {
    assert_eq!(
        DiagnosticCode::from_message("USE STRICT is missing"),
        Some(DiagnosticCode::MissingStrict)
    );
    assert_eq!(
        DiagnosticCode::from_message("Use Warnings required"),
        Some(DiagnosticCode::MissingWarnings)
    );
}

#[test]
fn from_message_empty_string_returns_none() {
    assert!(DiagnosticCode::from_message("").is_none());
}

#[test]
fn from_message_never_used_variant() {
    assert_eq!(
        DiagnosticCode::from_message("variable $x is never used"),
        Some(DiagnosticCode::UnusedVariable)
    );
}

#[test]
fn from_message_not_declared_variant() {
    assert_eq!(
        DiagnosticCode::from_message("symbol not declared in scope"),
        Some(DiagnosticCode::UndefinedVariable)
    );
}

#[test]
fn from_message_2_arg_variant() {
    assert_eq!(
        DiagnosticCode::from_message("found a 2-arg open call"),
        Some(DiagnosticCode::TwoArgOpen)
    );
}

#[test]
fn from_message_syntax_error_matches_parse_error() {
    // "syntax error" in message triggers ParseError code
    assert_eq!(
        DiagnosticCode::from_message("syntax error near token"),
        Some(DiagnosticCode::ParseError)
    );
}

#[test]
fn from_message_unrelated_text_returns_none() {
    assert!(DiagnosticCode::from_message("everything looks fine").is_none());
    assert!(DiagnosticCode::from_message("refactor this method").is_none());
}

// --- DiagnosticCode: documentation_url coverage ---

#[test]
fn documentation_url_format_consistency() {
    for code in ALL_CODES {
        if let Some(url) = code.documentation_url() {
            assert!(
                url.starts_with("https://docs.perl-lsp.org/errors/"),
                "unexpected url prefix for {}: {}",
                code.as_str(),
                url
            );
            assert!(
                url.ends_with(code.as_str()),
                "url should end with code string for {}: {}",
                code.as_str(),
                url
            );
        }
    }
}

#[test]
fn documentation_url_all_pl_codes_have_urls() {
    for code in ALL_CODES {
        if code.as_str().starts_with("PL") {
            assert!(
                code.documentation_url().is_some(),
                "PL code {} should have a documentation URL",
                code.as_str()
            );
        }
    }
}

#[test]
fn documentation_url_all_pc_codes_have_no_urls() {
    for code in ALL_CODES {
        if code.as_str().starts_with("PC") {
            assert!(
                code.documentation_url().is_none(),
                "PC code {} should not have a documentation URL",
                code.as_str()
            );
        }
    }
}

// --- DiagnosticCode: category-severity cross-checks ---

#[test]
fn parser_category_codes_are_all_errors() {
    for code in ALL_CODES {
        if code.category() == DiagnosticCategory::Parser {
            assert_eq!(
                code.severity(),
                DiagnosticSeverity::Error,
                "parser code {} should be Error severity",
                code.as_str()
            );
        }
    }
}

#[test]
fn information_severity_codes_are_explicitly_tracked() {
    let info_codes: Vec<_> = ALL_CODES
        .iter()
        .filter(|code| code.severity() == DiagnosticSeverity::Information)
        .collect();
    assert_eq!(
        info_codes,
        vec![&DiagnosticCode::CaptureVarWithoutRegexMatch]
    );
}

#[test]
fn hint_codes_are_only_critic_3_4_5() {
    let hint_codes: Vec<&DiagnosticCode> = ALL_CODES
        .iter()
        .filter(|c| c.severity() == DiagnosticSeverity::Hint)
        .collect();
    assert_eq!(hint_codes.len(), 3);
    for code in &hint_codes {
        assert_eq!(code.category(), DiagnosticCategory::PerlCritic);
    }
}

// --- DiagnosticCode: Display trait ---

#[test]
fn display_all_codes_matches_as_str() {
    for code in ALL_CODES {
        assert_eq!(format!("{}", code), code.as_str());
    }
}

#[test]
fn display_used_in_format_string() {
    let code = DiagnosticCode::ParseError;
    let msg = format!("[{}] something went wrong", code);
    assert_eq!(msg, "[PL001] something went wrong");
}

// --- DiagnosticCategory: equality and inequality ---

#[test]
fn category_equality() {
    assert_eq!(DiagnosticCategory::Parser, DiagnosticCategory::Parser);
    assert_eq!(
        DiagnosticCategory::StrictWarnings,
        DiagnosticCategory::StrictWarnings
    );
    assert_eq!(
        DiagnosticCategory::PackageModule,
        DiagnosticCategory::PackageModule
    );
    assert_eq!(
        DiagnosticCategory::Subroutine,
        DiagnosticCategory::Subroutine
    );
    assert_eq!(
        DiagnosticCategory::BestPractices,
        DiagnosticCategory::BestPractices
    );
    assert_eq!(
        DiagnosticCategory::PerlCritic,
        DiagnosticCategory::PerlCritic
    );
}

#[test]
fn category_inequality_across_variants() {
    assert_ne!(
        DiagnosticCategory::Parser,
        DiagnosticCategory::StrictWarnings
    );
    assert_ne!(
        DiagnosticCategory::PackageModule,
        DiagnosticCategory::Subroutine
    );
    assert_ne!(
        DiagnosticCategory::BestPractices,
        DiagnosticCategory::PerlCritic
    );
}

// --- DiagnosticCategory: coverage of all codes ---

#[test]
fn all_seven_categories_are_represented() {
    use std::collections::HashSet;
    let categories: HashSet<DiagnosticCategory> = ALL_CODES.iter().map(|c| c.category()).collect();
    assert!(categories.contains(&DiagnosticCategory::Parser));
    assert!(categories.contains(&DiagnosticCategory::StrictWarnings));
    assert!(categories.contains(&DiagnosticCategory::PackageModule));
    assert!(categories.contains(&DiagnosticCategory::Subroutine));
    assert!(categories.contains(&DiagnosticCategory::BestPractices));
    assert!(categories.contains(&DiagnosticCategory::Security));
    assert!(categories.contains(&DiagnosticCategory::PerlCritic));
    assert_eq!(categories.len(), 7);
}

// --- DiagnosticCode: tags exhaustive check ---

#[test]
fn only_unused_variable_has_non_empty_tags() {
    let codes_with_tags: Vec<&DiagnosticCode> =
        ALL_CODES.iter().filter(|c| !c.tags().is_empty()).collect();
    assert_eq!(codes_with_tags.len(), 1);
    assert_eq!(*codes_with_tags[0], DiagnosticCode::UnusedVariable);
}

#[test]
fn unused_variable_tag_is_exactly_unnecessary() {
    let tags = DiagnosticCode::UnusedVariable.tags();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0], DiagnosticTag::Unnecessary);
}

// --- Cross-cutting: ALL_CODES count ---

#[test]
fn all_codes_count_is_24() {
    assert_eq!(ALL_CODES.len(), 26, "expected 26 diagnostic codes total");
}

// --- DiagnosticCode: parse_code boundary values ---

#[test]
fn parse_code_all_valid_pl_codes() {
    let valid_pl = [
        "PL001", "PL002", "PL003", "PL100", "PL101", "PL102", "PL103", "PL200", "PL201", "PL300",
        "PL301", "PL302", "PL303", "PL400", "PL401", "PL402", "PL602", "PL502", "PL503",
    ];
    for s in &valid_pl {
        assert!(
            DiagnosticCode::parse_code(s).is_some(),
            "expected valid parse for {}",
            s
        );
    }
}

#[test]
fn parse_code_all_valid_pc_codes() {
    let valid_pc = ["PC001", "PC002", "PC003", "PC004", "PC005"];
    for s in &valid_pc {
        assert!(
            DiagnosticCode::parse_code(s).is_some(),
            "expected valid parse for {}",
            s
        );
    }
}

#[test]
fn parse_code_gaps_return_none() {
    // Codes that fall in valid ranges but aren't assigned.
    // Note: PL104-PL111, PL303, PL403-PL404, PL500-PL503, PL600-PL602, PL700, PL800-PL806
    // are now assigned; only truly unassigned gaps are listed here.
    let gaps = [
        "PL004", "PL050", "PL099", "PL150", "PL199", "PL202", "PL250", "PL399", "PL499", "PL599",
        "PL699", "PL702", "PL799", "PL807", "PL899", "PC006", "PC010",
    ];
    for s in &gaps {
        assert!(
            DiagnosticCode::parse_code(s).is_none(),
            "expected None for unassigned code {}",
            s
        );
    }
}

// --- from_message: priority / first-match behavior ---

#[test]
fn from_message_prefers_use_strict_over_generic() {
    // "use strict" should match MissingStrict even if other keywords present
    let result = DiagnosticCode::from_message("use strict is required, parse error possible");
    assert_eq!(result, Some(DiagnosticCode::MissingStrict));
}

#[test]
fn from_message_embedded_in_longer_text() {
    let result =
        DiagnosticCode::from_message("Warning: the file does not contain use warnings at line 42");
    assert_eq!(result, Some(DiagnosticCode::MissingWarnings));
}

// --- Severity: no codes map to unexpected values ---

#[test]
fn all_lsp_severity_values_are_valid() {
    for code in ALL_CODES {
        let val = code.severity().to_lsp_value();
        assert!(
            (1..=4).contains(&val),
            "severity LSP value out of range for {}: {}",
            code.as_str(),
            val
        );
    }
}
