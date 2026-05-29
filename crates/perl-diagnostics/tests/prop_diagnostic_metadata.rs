//! Property coverage for the diagnostic code registry and LSP metadata bridge.
//!
//! These tests exercise every known diagnostic variant through generated case
//! selection so newly added codes must keep parse, URL, hint, and tag metadata
//! internally consistent.

use perl_diagnostics::catalog::diagnostic_meta;
use perl_diagnostics::codes::{DiagnosticCategory, DiagnosticCode, DiagnosticTag};
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};

const DIAGNOSTIC_CODES: &[DiagnosticCode] = &[
    DiagnosticCode::ParseError,
    DiagnosticCode::SyntaxError,
    DiagnosticCode::UnexpectedEof,
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
    DiagnosticCode::MissingPackageDeclaration,
    DiagnosticCode::DuplicatePackage,
    DiagnosticCode::DuplicateSubroutine,
    DiagnosticCode::MissingReturn,
    DiagnosticCode::InvalidPrototype,
    DiagnosticCode::RoleConflict,
    DiagnosticCode::MissingPodCoverage,
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
    DiagnosticCode::LoopControlUndefinedLabel,
    DiagnosticCode::DeprecatedDefined,
    DiagnosticCode::DeprecatedArrayBase,
    DiagnosticCode::PhaseScopedStrictPragma,
    DiagnosticCode::PhaseScopedWarningsPragma,
    DiagnosticCode::SecurityStringEval,
    DiagnosticCode::SecurityBacktickExec,
    DiagnosticCode::SecuritySignalHandler,
    DiagnosticCode::SecuritySystemCall,
    DiagnosticCode::SecurityExecCall,
    DiagnosticCode::SecurityPipeOpen,
    DiagnosticCode::SecurityReadpipe,
    DiagnosticCode::UnusedImport,
    DiagnosticCode::ModuleNotFound,
    DiagnosticCode::HeredocInFormat,
    DiagnosticCode::HeredocInBegin,
    DiagnosticCode::HeredocDynamicDelimiter,
    DiagnosticCode::HeredocInSourceFilter,
    DiagnosticCode::HeredocInRegexCode,
    DiagnosticCode::HeredocInEval,
    DiagnosticCode::HeredocTiedHandle,
    DiagnosticCode::VersionIncompatFeature,
    DiagnosticCode::CriticSeverity1,
    DiagnosticCode::CriticSeverity2,
    DiagnosticCode::CriticSeverity3,
    DiagnosticCode::CriticSeverity4,
    DiagnosticCode::CriticSeverity5,
];

fn diagnostic_code_strategy() -> impl Strategy<Value = DiagnosticCode> {
    prop::sample::select(DIAGNOSTIC_CODES)
}

fn known_code_string(code: &str) -> bool {
    DiagnosticCode::parse_code(code).is_some()
}

#[test]
fn prop_diagnostic_code_registry_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let mut runner = TestRunner::new(Config::with_cases(256));

    runner.run(&diagnostic_code_strategy(), |code| {
        let code_string = code.as_str();

        prop_assert_eq!(DiagnosticCode::parse_code(code_string), Some(code));
        prop_assert!(code_string.len() == 5);
        prop_assert!(code_string.starts_with("PL") || code_string.starts_with("PC"));
        prop_assert_eq!(
            code_string.starts_with("PC"),
            code.category() == DiagnosticCategory::PerlCritic
        );

        let meta = diagnostic_meta(code);
        prop_assert_eq!(meta.code, serde_json::json!(code_string));

        if code_string.starts_with("PL") {
            let expected_url = format!("https://docs.perl-lsp.org/errors/{code_string}");
            prop_assert_eq!(code.documentation_url(), Some(expected_url.as_str()));
            prop_assert_eq!(meta.desc, Some(serde_json::json!({ "href": expected_url })));
            prop_assert!(meta.hint.is_some());
        } else {
            prop_assert_eq!(code.documentation_url(), None);
            prop_assert_eq!(meta.desc, None);
            prop_assert_eq!(meta.hint, None);
        }

        Ok(())
    })?;

    Ok(())
}

#[test]
fn prop_diagnostic_tags_are_lsp_safe_and_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let mut runner = TestRunner::new(Config::with_cases(256));

    runner.run(&diagnostic_code_strategy(), |code| {
        let tags = code.tags();
        prop_assert!(tags.len() <= 1);

        for tag in tags {
            prop_assert!(matches!(tag.to_lsp_value(), 1 | 2));
        }

        prop_assert_eq!(
            tags.contains(&DiagnosticTag::Deprecated),
            matches!(code, DiagnosticCode::DeprecatedDefined | DiagnosticCode::DeprecatedArrayBase)
        );
        prop_assert_eq!(
            tags.contains(&DiagnosticTag::Unnecessary),
            matches!(
                code,
                DiagnosticCode::UnusedVariable
                    | DiagnosticCode::UnusedParameter
                    | DiagnosticCode::UnusedImport
                    | DiagnosticCode::UnreachableCode
            )
        );

        Ok(())
    })?;

    Ok(())
}

#[test]
fn prop_unknown_formatted_code_strings_do_not_parse() -> Result<(), Box<dyn std::error::Error>> {
    let candidate = (prop_oneof![Just("PL"), Just("PC"), Just("PX")], 0_u16..1000)
        .prop_map(|(prefix, number)| format!("{prefix}{number:03}"));
    let mut runner = TestRunner::new(Config::with_cases(512));

    runner.run(&candidate, |code_string| {
        prop_assume!(!known_code_string(&code_string));
        prop_assert_eq!(DiagnosticCode::parse_code(&code_string), None);
        Ok(())
    })?;

    Ok(())
}
