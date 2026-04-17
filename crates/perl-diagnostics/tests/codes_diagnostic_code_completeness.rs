//! Tests verifying completeness and correctness of the DiagnosticCode registry.
//!
//! These tests check that every diagnostic path in `perl-lsp-diagnostics`
//! has a corresponding stable code in `perl-diagnostics-codes`, and that
//! all code strings follow the expected PL/PC naming convention.

use perl_diagnostics::codes::DiagnosticCode;

// ---------------------------------------------------------------------------
// Scope diagnostic codes (PL104-PL110)
// ---------------------------------------------------------------------------

#[test]
fn scope_variable_shadowing_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::VariableShadowing;
    assert_eq!(code.as_str(), "PL104", "VariableShadowing should have stable code PL104");
    assert_eq!(DiagnosticCode::parse_code("PL104"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn scope_variable_redeclaration_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::VariableRedeclaration;
    assert_eq!(code.as_str(), "PL105", "VariableRedeclaration should have stable code PL105");
    assert_eq!(DiagnosticCode::parse_code("PL105"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn scope_duplicate_parameter_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::DuplicateParameter;
    assert_eq!(code.as_str(), "PL106", "DuplicateParameter should have stable code PL106");
    assert_eq!(DiagnosticCode::parse_code("PL106"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn scope_parameter_shadows_global_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::ParameterShadowsGlobal;
    assert_eq!(code.as_str(), "PL107", "ParameterShadowsGlobal should have stable code PL107");
    assert_eq!(DiagnosticCode::parse_code("PL107"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn scope_unused_parameter_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::UnusedParameter;
    assert_eq!(code.as_str(), "PL108", "UnusedParameter should have stable code PL108");
    assert_eq!(DiagnosticCode::parse_code("PL108"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn scope_unquoted_bareword_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::UnquotedBareword;
    assert_eq!(code.as_str(), "PL109", "UnquotedBareword should have stable code PL109");
    assert_eq!(DiagnosticCode::parse_code("PL109"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn scope_uninitialized_variable_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::UninitializedVariable;
    assert_eq!(code.as_str(), "PL110", "UninitializedVariable should have stable code PL110");
    assert_eq!(DiagnosticCode::parse_code("PL110"), Some(code), "parse_code round-trip failed");
    Ok(())
}

// ---------------------------------------------------------------------------
// Pragma diagnostic codes (PL102, PL103 already exist; PL111 for misspelled)
// ---------------------------------------------------------------------------

#[test]
fn misspelled_pragma_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::MisspelledPragma;
    assert_eq!(code.as_str(), "PL111", "MisspelledPragma should have stable code PL111");
    assert_eq!(DiagnosticCode::parse_code("PL111"), Some(code), "parse_code round-trip failed");
    Ok(())
}

// ---------------------------------------------------------------------------
// Lint diagnostic codes — common mistakes (PL4xx range)
// ---------------------------------------------------------------------------

#[test]
fn assignment_in_condition_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::AssignmentInCondition;
    assert_eq!(code.as_str(), "PL403", "AssignmentInCondition should have stable code PL403");
    assert_eq!(DiagnosticCode::parse_code("PL403"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn numeric_comparison_with_undef_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::NumericComparisonWithUndef;
    assert_eq!(code.as_str(), "PL404", "NumericComparisonWithUndef should have stable code PL404");
    assert_eq!(DiagnosticCode::parse_code("PL404"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn duplicate_hash_key_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::DuplicateHashKey;
    assert_eq!(code.as_str(), "PL408", "DuplicateHashKey should have code PL408");
    assert_eq!(
        code.severity(),
        perl_diagnostics::codes::DiagnosticSeverity::Warning,
        "DuplicateHashKey should have Warning severity"
    );
    assert!(code.context_hint().is_some(), "DuplicateHashKey should have a context hint");
    assert_eq!(
        DiagnosticCode::parse_code("PL408"),
        Some(DiagnosticCode::DuplicateHashKey),
        "parse_code('PL408') should return DuplicateHashKey"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Deprecated syntax codes (PL5xx range)
// ---------------------------------------------------------------------------

#[test]
fn deprecated_defined_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::DeprecatedDefined;
    assert_eq!(code.as_str(), "PL500", "DeprecatedDefined should have stable code PL500");
    assert_eq!(DiagnosticCode::parse_code("PL500"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn deprecated_array_base_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::DeprecatedArrayBase;
    assert_eq!(code.as_str(), "PL501", "DeprecatedArrayBase should have stable code PL501");
    assert_eq!(DiagnosticCode::parse_code("PL501"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn phase_scoped_strict_pragma_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::PhaseScopedStrictPragma;
    assert_eq!(code.as_str(), "PL502", "PhaseScopedStrictPragma should have code PL502");
    assert_eq!(
        code.severity(),
        perl_diagnostics::codes::DiagnosticSeverity::Warning,
        "PhaseScopedStrictPragma should have Warning severity"
    );
    assert!(code.context_hint().is_some(), "PhaseScopedStrictPragma should have a context hint");
    assert_eq!(
        DiagnosticCode::parse_code("PL502"),
        Some(DiagnosticCode::PhaseScopedStrictPragma),
        "parse_code('PL502') should return PhaseScopedStrictPragma"
    );
    Ok(())
}

#[test]
fn phase_scoped_warnings_pragma_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::PhaseScopedWarningsPragma;
    assert_eq!(code.as_str(), "PL503", "PhaseScopedWarningsPragma should have code PL503");
    assert_eq!(
        code.severity(),
        perl_diagnostics::codes::DiagnosticSeverity::Warning,
        "PhaseScopedWarningsPragma should have Warning severity"
    );
    assert!(code.context_hint().is_some(), "PhaseScopedWarningsPragma should have a context hint");
    assert_eq!(
        DiagnosticCode::parse_code("PL503"),
        Some(DiagnosticCode::PhaseScopedWarningsPragma),
        "parse_code('PL503') should return PhaseScopedWarningsPragma"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Security diagnostic codes (PL6xx range)
// ---------------------------------------------------------------------------

#[test]
fn security_string_eval_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::SecurityStringEval;
    assert_eq!(code.as_str(), "PL600", "SecurityStringEval should have stable code PL600");
    assert_eq!(DiagnosticCode::parse_code("PL600"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn security_backtick_exec_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::SecurityBacktickExec;
    assert_eq!(code.as_str(), "PL601", "SecurityBacktickExec should have stable code PL601");
    assert_eq!(DiagnosticCode::parse_code("PL601"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn security_signal_handler_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::SecuritySignalHandler;
    assert_eq!(code.as_str(), "PL602", "SecuritySignalHandler should have code PL602");
    assert_eq!(
        code.severity(),
        perl_diagnostics::codes::DiagnosticSeverity::Warning,
        "SecuritySignalHandler should have Warning severity"
    );
    assert!(code.context_hint().is_some(), "SecuritySignalHandler should have a context hint");
    assert_eq!(
        DiagnosticCode::parse_code("PL602"),
        Some(DiagnosticCode::SecuritySignalHandler),
        "parse_code('PL602') should return SecuritySignalHandler"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Import diagnostic codes
// ---------------------------------------------------------------------------

#[test]
fn unused_import_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::UnusedImport;
    assert_eq!(code.as_str(), "PL700", "UnusedImport should have stable code PL700");
    assert_eq!(DiagnosticCode::parse_code("PL700"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn module_not_found_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::ModuleNotFound;
    assert_eq!(code.as_str(), "PL701", "ModuleNotFound should have code PL701");
    assert_eq!(
        code.severity(),
        perl_diagnostics::codes::DiagnosticSeverity::Warning,
        "ModuleNotFound should have Warning severity"
    );
    assert!(code.context_hint().is_some(), "ModuleNotFound should have a context hint");
    assert_eq!(
        DiagnosticCode::parse_code("PL701"),
        Some(DiagnosticCode::ModuleNotFound),
        "parse_code('PL701') should return ModuleNotFound"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Heredoc anti-pattern codes
// ---------------------------------------------------------------------------

#[test]
fn heredoc_in_format_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocInFormat;
    assert_eq!(code.as_str(), "PL800", "HeredocInFormat should have stable code PL800");
    assert_eq!(DiagnosticCode::parse_code("PL800"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn heredoc_in_begin_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocInBegin;
    assert_eq!(code.as_str(), "PL801", "HeredocInBegin should have stable code PL801");
    assert_eq!(DiagnosticCode::parse_code("PL801"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn heredoc_dynamic_delimiter_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocDynamicDelimiter;
    assert_eq!(code.as_str(), "PL802", "HeredocDynamicDelimiter should have stable code PL802");
    assert_eq!(DiagnosticCode::parse_code("PL802"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn heredoc_in_source_filter_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocInSourceFilter;
    assert_eq!(code.as_str(), "PL803", "HeredocInSourceFilter should have stable code PL803");
    assert_eq!(DiagnosticCode::parse_code("PL803"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn heredoc_in_regex_code_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocInRegexCode;
    assert_eq!(code.as_str(), "PL804", "HeredocInRegexCode should have stable code PL804");
    assert_eq!(DiagnosticCode::parse_code("PL804"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn heredoc_in_eval_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocInEval;
    assert_eq!(code.as_str(), "PL805", "HeredocInEval should have stable code PL805");
    assert_eq!(DiagnosticCode::parse_code("PL805"), Some(code), "parse_code round-trip failed");
    Ok(())
}

#[test]
fn heredoc_tied_handle_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocTiedHandle;
    assert_eq!(code.as_str(), "PL806", "HeredocTiedHandle should have stable code PL806");
    assert_eq!(DiagnosticCode::parse_code("PL806"), Some(code), "parse_code round-trip failed");
    Ok(())
}

// ---------------------------------------------------------------------------
// Code string format invariants
// ---------------------------------------------------------------------------

/// All codes must start with PL or PC prefix.
#[test]
fn all_codes_have_valid_prefix() -> Result<(), Box<dyn std::error::Error>> {
    // Existing codes that are already stable
    let existing_codes = [
        DiagnosticCode::ParseError,
        DiagnosticCode::SyntaxError,
        DiagnosticCode::UnexpectedEof,
        DiagnosticCode::MissingStrict,
        DiagnosticCode::MissingWarnings,
        DiagnosticCode::UnusedVariable,
        DiagnosticCode::UndefinedVariable,
        DiagnosticCode::MissingPackageDeclaration,
        DiagnosticCode::DuplicatePackage,
        DiagnosticCode::DuplicateSubroutine,
        DiagnosticCode::MissingReturn,
        DiagnosticCode::BarewordFilehandle,
        DiagnosticCode::TwoArgOpen,
        DiagnosticCode::ImplicitReturn,
        DiagnosticCode::CriticSeverity1,
        DiagnosticCode::CriticSeverity2,
        DiagnosticCode::CriticSeverity3,
        DiagnosticCode::CriticSeverity4,
        DiagnosticCode::CriticSeverity5,
    ];

    for code in &existing_codes {
        let s = code.as_str();
        assert!(
            s.starts_with("PL") || s.starts_with("PC"),
            "Code {s} does not start with PL or PC"
        );
    }
    Ok(())
}

/// parse_code round-trip: as_str() output must be parseable back.
#[test]
fn existing_codes_round_trip_through_parse_code() -> Result<(), Box<dyn std::error::Error>> {
    let codes = [
        DiagnosticCode::ParseError,
        DiagnosticCode::MissingStrict,
        DiagnosticCode::TwoArgOpen,
        DiagnosticCode::CriticSeverity3,
    ];
    for code in codes {
        let s = code.as_str();
        let parsed = DiagnosticCode::parse_code(s);
        assert_eq!(parsed, Some(code), "parse_code({s}) should return {code:?}");
    }
    Ok(())
}
