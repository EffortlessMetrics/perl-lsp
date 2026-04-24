//! Snapshot tests for perl-diagnostics-codes output surface.
//!
//! These tests capture the baseline output of key functions to detect any
//! unintended changes. Snapshot tests are particularly important for semver
//! hygiene because the string codes (e.g., "PL001") are part of the public
//! API surface that external tools may depend on.
//!
//! ## What's Snapshot Tested
//!
//! - `DiagnosticCode::as_str()` for all 58 variants — string codes like "PL001", "PC005"
//! - `DiagnosticSeverity::to_lsp_value()` — LSP numeric severity values
//! - `DiagnosticCode::documentation_url()` — URL patterns for docs
//!
//! ## Why Snapshots?
//!
//! Property tests verify CORRECTNESS (e.g., "all codes roundtrip"). Snapshot tests
//! verify STABILITY (e.g., "PL001 today equals PL001 yesterday"). Both are needed
//! for comprehensive semver hygiene.

use insta::assert_snapshot;
use perl_diagnostics_codes::{DiagnosticCode, DiagnosticSeverity};

/// Snapshot the string representation of ALL DiagnosticCode variants.
/// This captures the complete public API surface for diagnostic codes.
///
/// Any change to these strings is a semver-breaking change for external consumers.
#[test]
fn diagnostic_code_as_str_all_variants() {
    let codes: Vec<(&str, &str)> =
        ALL_CODES.iter().map(|code| (stringify!(code), code.as_str())).collect();

    // Create a multi-line snapshot showing all codes
    let snapshot = codes
        .iter()
        .map(|(name, code)| format!("{:<35} => \"{}\"", name, code))
        .collect::<Vec<_>>()
        .join("\n");

    assert_snapshot!("diagnostic_code_as_str_all_variants", snapshot);
}

/// Snapshot test for LSP severity values.
/// These numeric values map to LSP DiagnosticSeverity.
#[test]
fn diagnostic_severity_to_lsp_value() {
    let severities = [
        ("Error", DiagnosticSeverity::Error),
        ("Warning", DiagnosticSeverity::Warning),
        ("Information", DiagnosticSeverity::Information),
        ("Hint", DiagnosticSeverity::Hint),
    ];

    let snapshot = severities
        .iter()
        .map(|(name, sev)| format!("{:<15} => {}", name, sev.to_lsp_value()))
        .collect::<Vec<_>>()
        .join("\n");

    assert_snapshot!("diagnostic_severity_to_lsp_value", snapshot);
}

/// Snapshot test for documentation URLs.
/// This captures the URL pattern for all PL* codes.
#[test]
fn diagnostic_code_documentation_url_known_codes() {
    // Test a representative sample of codes across different ranges
    let sample_codes = [
        DiagnosticCode::ParseError,                // PL001
        DiagnosticCode::UnusedVariable,            // PL102
        DiagnosticCode::MissingPackageDeclaration, // PL200
        DiagnosticCode::DuplicateSubroutine,       // PL300
        DiagnosticCode::UnreachableCode,           // PL406
        DiagnosticCode::SecurityStringEval,        // PL600
        DiagnosticCode::UnusedImport,              // PL700
        DiagnosticCode::HeredocInFormat,           // PL800
        DiagnosticCode::VersionIncompatFeature,    // PL900
        DiagnosticCode::CriticSeverity1,           // PC001
    ];

    let snapshot: Vec<String> = sample_codes
        .iter()
        .map(|code| {
            let str_code = code.as_str();
            let url = code.documentation_url().unwrap_or("(none)");
            format!("{:<8} => {}", str_code, url)
        })
        .collect();

    assert_snapshot!("diagnostic_code_documentation_url_known_codes", snapshot.join("\n"));
}

// ---------------------------------------------------------------------------
// Helper: all DiagnosticCode variants for exhaustive iteration
// ---------------------------------------------------------------------------

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
