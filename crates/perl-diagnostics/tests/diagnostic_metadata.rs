//! Exhaustive coverage for the diagnostic code registry and LSP metadata bridge.
//!
//! These tests exercise every known diagnostic variant so newly added codes must
//! keep parse, URL, hint, and tag metadata internally consistent.

use perl_diagnostics::catalog::diagnostic_meta;
use perl_diagnostics::codes::{DiagnosticCategory, DiagnosticCode, DiagnosticTag};

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

fn known_code_string(code: &str) -> bool {
    DiagnosticCode::parse_code(code).is_some()
}

#[test]
fn diagnostic_code_registry_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    for code in DIAGNOSTIC_CODES {
        let code_string = code.as_str();

        assert_eq!(DiagnosticCode::parse_code(code_string), Some(*code));
        assert!(code_string.len() == 5);
        assert!(code_string.starts_with("PL") || code_string.starts_with("PC"));
        assert_eq!(
            code_string.starts_with("PC"),
            code.category() == DiagnosticCategory::PerlCritic
        );

        let meta = diagnostic_meta(*code);
        assert_eq!(meta.code, serde_json::json!(code_string));

        if code_string.starts_with("PL") {
            let expected_url = format!("https://docs.perl-lsp.org/errors/{code_string}");
            assert_eq!(code.documentation_url(), Some(expected_url.as_str()));
            assert_eq!(meta.desc, Some(serde_json::json!({ "href": expected_url })));
            assert!(meta.hint.is_some());
        } else {
            assert_eq!(code.documentation_url(), None);
            assert_eq!(meta.desc, None);
            assert_eq!(meta.hint, None);
        }
    }

    Ok(())
}

#[test]
fn diagnostic_tags_are_lsp_safe_and_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    for code in DIAGNOSTIC_CODES {
        let tags = code.tags();
        assert!(tags.len() <= 1);

        for tag in tags {
            assert!(matches!(tag.to_lsp_value(), 1 | 2));
        }

        assert_eq!(
            tags.contains(&DiagnosticTag::Deprecated),
            matches!(code, DiagnosticCode::DeprecatedDefined | DiagnosticCode::DeprecatedArrayBase)
        );
        assert_eq!(
            tags.contains(&DiagnosticTag::Unnecessary),
            matches!(
                code,
                DiagnosticCode::UnusedVariable
                    | DiagnosticCode::UnusedParameter
                    | DiagnosticCode::UnusedImport
                    | DiagnosticCode::UnreachableCode
            )
        );
    }

    Ok(())
}

#[test]
fn unknown_formatted_code_strings_do_not_parse() -> Result<(), Box<dyn std::error::Error>> {
    for prefix in ["PL", "PC", "PX"] {
        for number in 0_u16..1000 {
            let code_string = format!("{prefix}{number:03}");

            if known_code_string(&code_string) {
                continue;
            }

            assert_eq!(
                DiagnosticCode::parse_code(&code_string),
                None,
                "{code_string} should not parse as a known diagnostic code"
            );
        }
    }

    Ok(())
}
