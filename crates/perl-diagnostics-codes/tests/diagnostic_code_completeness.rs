//! Tests verifying completeness and correctness of the DiagnosticCode registry.
//!
//! These tests check that every diagnostic path in `perl-lsp-diagnostics`
//! has a corresponding stable code in `perl-diagnostics-codes`, and that
//! all code strings follow the expected PL/PC naming convention.

use perl_diagnostics_codes::DiagnosticCode;

// ---------------------------------------------------------------------------
// Scope diagnostic codes (PL104-PL110)
// ---------------------------------------------------------------------------

#[test]
fn scope_variable_shadowing_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::VariableShadowing;
    assert!(
        code.as_str().starts_with("PL"),
        "VariableShadowing should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn scope_variable_redeclaration_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::VariableRedeclaration;
    assert!(
        code.as_str().starts_with("PL"),
        "VariableRedeclaration should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn scope_duplicate_parameter_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::DuplicateParameter;
    assert!(
        code.as_str().starts_with("PL"),
        "DuplicateParameter should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn scope_parameter_shadows_global_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::ParameterShadowsGlobal;
    assert!(
        code.as_str().starts_with("PL"),
        "ParameterShadowsGlobal should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn scope_unused_parameter_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::UnusedParameter;
    assert!(
        code.as_str().starts_with("PL"),
        "UnusedParameter should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn scope_unquoted_bareword_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::UnquotedBareword;
    assert!(
        code.as_str().starts_with("PL"),
        "UnquotedBareword should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn scope_uninitialized_variable_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::UninitializedVariable;
    assert!(
        code.as_str().starts_with("PL"),
        "UninitializedVariable should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Pragma diagnostic codes (PL102, PL103 already exist; PL111 for misspelled)
// ---------------------------------------------------------------------------

#[test]
fn misspelled_pragma_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::MisspelledPragma;
    assert!(
        code.as_str().starts_with("PL"),
        "MisspelledPragma should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Lint diagnostic codes — common mistakes (PL4xx range)
// ---------------------------------------------------------------------------

#[test]
fn assignment_in_condition_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::AssignmentInCondition;
    assert!(
        code.as_str().starts_with("PL"),
        "AssignmentInCondition should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn numeric_comparison_with_undef_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::NumericComparisonWithUndef;
    assert!(
        code.as_str().starts_with("PL"),
        "NumericComparisonWithUndef should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Deprecated syntax codes (PL5xx range)
// ---------------------------------------------------------------------------

#[test]
fn deprecated_defined_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::DeprecatedDefined;
    assert!(
        code.as_str().starts_with("PL"),
        "DeprecatedDefined should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn deprecated_array_base_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::DeprecatedArrayBase;
    assert!(
        code.as_str().starts_with("PL"),
        "DeprecatedArrayBase should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Security diagnostic codes (PL6xx range)
// ---------------------------------------------------------------------------

#[test]
fn security_string_eval_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::SecurityStringEval;
    assert!(
        code.as_str().starts_with("PL"),
        "SecurityStringEval should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn security_backtick_exec_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::SecurityBacktickExec;
    assert!(
        code.as_str().starts_with("PL"),
        "SecurityBacktickExec should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Import diagnostic codes
// ---------------------------------------------------------------------------

#[test]
fn unused_import_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::UnusedImport;
    assert!(
        code.as_str().starts_with("PL"),
        "UnusedImport should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Heredoc anti-pattern codes
// ---------------------------------------------------------------------------

#[test]
fn heredoc_in_format_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocInFormat;
    assert!(
        code.as_str().starts_with("PL"),
        "HeredocInFormat should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn heredoc_in_begin_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocInBegin;
    assert!(
        code.as_str().starts_with("PL"),
        "HeredocInBegin should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn heredoc_dynamic_delimiter_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocDynamicDelimiter;
    assert!(
        code.as_str().starts_with("PL"),
        "HeredocDynamicDelimiter should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn heredoc_in_source_filter_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocInSourceFilter;
    assert!(
        code.as_str().starts_with("PL"),
        "HeredocInSourceFilter should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn heredoc_in_regex_code_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocInRegexCode;
    assert!(
        code.as_str().starts_with("PL"),
        "HeredocInRegexCode should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn heredoc_in_eval_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocInEval;
    assert!(
        code.as_str().starts_with("PL"),
        "HeredocInEval should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

#[test]
fn heredoc_tied_handle_code_exists() -> Result<(), Box<dyn std::error::Error>> {
    let code = DiagnosticCode::HeredocTiedHandle;
    assert!(
        code.as_str().starts_with("PL"),
        "HeredocTiedHandle should have a PL code, got: {}",
        code.as_str()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Code string format invariants
// ---------------------------------------------------------------------------

/// All codes must start with PL or PC prefix.
#[test]
fn all_codes_have_valid_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let all_codes = [
        // Parser
        DiagnosticCode::ParseError,
        DiagnosticCode::SyntaxError,
        DiagnosticCode::UnexpectedEof,
        // Strict/warnings/scope
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
        // Package/module
        DiagnosticCode::MissingPackageDeclaration,
        DiagnosticCode::DuplicatePackage,
        // Subroutine
        DiagnosticCode::DuplicateSubroutine,
        DiagnosticCode::MissingReturn,
        // Best practices
        DiagnosticCode::BarewordFilehandle,
        DiagnosticCode::TwoArgOpen,
        DiagnosticCode::ImplicitReturn,
        DiagnosticCode::AssignmentInCondition,
        DiagnosticCode::NumericComparisonWithUndef,
        // Deprecated
        DiagnosticCode::DeprecatedDefined,
        DiagnosticCode::DeprecatedArrayBase,
        // Security
        DiagnosticCode::SecurityStringEval,
        DiagnosticCode::SecurityBacktickExec,
        // Import
        DiagnosticCode::UnusedImport,
        // Heredoc
        DiagnosticCode::HeredocInFormat,
        DiagnosticCode::HeredocInBegin,
        DiagnosticCode::HeredocDynamicDelimiter,
        DiagnosticCode::HeredocInSourceFilter,
        DiagnosticCode::HeredocInRegexCode,
        DiagnosticCode::HeredocInEval,
        DiagnosticCode::HeredocTiedHandle,
        // Perl::Critic
        DiagnosticCode::CriticSeverity1,
        DiagnosticCode::CriticSeverity2,
        DiagnosticCode::CriticSeverity3,
        DiagnosticCode::CriticSeverity4,
        DiagnosticCode::CriticSeverity5,
    ];

    for code in &all_codes {
        let s = code.as_str();
        assert!(
            s.starts_with("PL") || s.starts_with("PC"),
            "Code {s} does not start with PL or PC"
        );
    }
    Ok(())
}

/// parse_code round-trip: as_str() output must be parseable back for all new codes.
#[test]
fn new_codes_round_trip_through_parse_code() -> Result<(), Box<dyn std::error::Error>> {
    let new_codes = [
        DiagnosticCode::VariableShadowing,
        DiagnosticCode::VariableRedeclaration,
        DiagnosticCode::DuplicateParameter,
        DiagnosticCode::ParameterShadowsGlobal,
        DiagnosticCode::UnusedParameter,
        DiagnosticCode::UnquotedBareword,
        DiagnosticCode::UninitializedVariable,
        DiagnosticCode::MisspelledPragma,
        DiagnosticCode::AssignmentInCondition,
        DiagnosticCode::NumericComparisonWithUndef,
        DiagnosticCode::DeprecatedDefined,
        DiagnosticCode::DeprecatedArrayBase,
        DiagnosticCode::SecurityStringEval,
        DiagnosticCode::SecurityBacktickExec,
        DiagnosticCode::UnusedImport,
        DiagnosticCode::HeredocInFormat,
        DiagnosticCode::HeredocInBegin,
        DiagnosticCode::HeredocDynamicDelimiter,
        DiagnosticCode::HeredocInSourceFilter,
        DiagnosticCode::HeredocInRegexCode,
        DiagnosticCode::HeredocInEval,
        DiagnosticCode::HeredocTiedHandle,
    ];
    for code in new_codes {
        let s = code.as_str();
        let parsed = DiagnosticCode::parse_code(s);
        assert_eq!(parsed, Some(code), "parse_code({s}) should return {code:?}");
    }
    Ok(())
}

/// parse_code round-trip: as_str() output must be parseable back for original codes.
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
