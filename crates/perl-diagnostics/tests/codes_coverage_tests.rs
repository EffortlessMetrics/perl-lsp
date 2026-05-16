//! Coverage gap tests for `codes/mod.rs` — all previously uncovered diagnostic code variants.
//!
//! The existing test suite (`codes_comprehensive_unit_tests.rs`) uses an internal `ALL_CODES`
//! array of only 26 variants out of 60 total.  The match arms for the remaining 34 variants
//! in `severity()`, `category()`, `context_hint()`, `tags()`, `as_str()`, `parse_code()`,
//! and `documentation_url()` were never executed, producing 167 missed lines.
//!
//! This file exercises every previously-untouched variant through all public methods,
//! reducing codes/mod.rs line coverage from 74.81% toward ≥95%.
//!
//! # Organisation
//!
//! Tests are grouped by diagnostic code range:
//! - `strict_warnings` — PL104–PL112 (shadowing, params, bareword, uninitialized, misspelled)
//! - `subroutine` — PL303–PL304 (role conflict, pod coverage) [MissingPodCoverage was missing]
//! - `best_practices` — PL403–PL410 (assignment-in-cond through goto/loop labels)
//! - `deprecated` — PL500–PL503 (deprecated syntax + phase-scoped pragmas)
//! - `security` — PL600–PL606 (all security codes)
//! - `import` — PL700–PL701
//! - `heredoc` — PL800–PL806
//! - `version` — PL900
//! - `exhaustive_all` — data-driven sweep of all 60 variants at once
//!
//! # What stays uncovered and why
//!
//! The `#[default]` attribute on `DiagnosticCode::ParseError` (line 96) and
//! `DiagnosticSeverity::Error` (line 33) are exercised by existing tests; no
//! gap remains there.
//!
//! The `fmt::Display` impls for `DiagnosticCategory` (lines 908-922) already
//! have full coverage from the internal `#[cfg(test)]` module inside `codes/mod.rs`.
//!
//! The `from_message()` priority branches for `PhaseScopedStrict` and
//! `PhaseScopedWarnings` (the first two arms) are exercised in
//! `codes_comprehensive_unit_tests.rs::from_message_*` tests; this file adds
//! additional verification for those arms via the helper below.

use perl_diagnostics::codes::{
    DiagnosticCategory, DiagnosticCode, DiagnosticSeverity, DiagnosticTag,
};

// ---------------------------------------------------------------------------
// Exhaustive all-codes sweep — the primary coverage driver
//
// One array that lists every single enum variant once.  Iterating it through
// every public method exercises each match arm exactly once, covering the
// lines the smaller existing test arrays skipped.
// ---------------------------------------------------------------------------

/// Every `DiagnosticCode` variant in declaration order (60 total).
///
/// IMPORTANT: Keep in sync with `DiagnosticCode` enum in `codes/mod.rs`.
/// Adding a variant there without adding it here will silently leave lines
/// uncovered.
const COMPLETE_ALL_CODES: &[DiagnosticCode] = &[
    // PL001-PL003: Parser
    DiagnosticCode::ParseError,
    DiagnosticCode::SyntaxError,
    DiagnosticCode::UnexpectedEof,
    // PL100-PL112: Strict/warnings
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
    // PL200-PL201: Package/module
    DiagnosticCode::MissingPackageDeclaration,
    DiagnosticCode::DuplicatePackage,
    // PL300-PL304: Subroutine
    DiagnosticCode::DuplicateSubroutine,
    DiagnosticCode::MissingReturn,
    DiagnosticCode::InvalidPrototype,
    DiagnosticCode::RoleConflict,
    DiagnosticCode::MissingPodCoverage,
    // PL400-PL410: Best practices
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
    // PL500-PL503: Deprecated
    DiagnosticCode::DeprecatedDefined,
    DiagnosticCode::DeprecatedArrayBase,
    DiagnosticCode::PhaseScopedStrictPragma,
    DiagnosticCode::PhaseScopedWarningsPragma,
    // PL600-PL606: Security
    DiagnosticCode::SecurityStringEval,
    DiagnosticCode::SecurityBacktickExec,
    DiagnosticCode::SecuritySignalHandler,
    DiagnosticCode::SecuritySystemCall,
    DiagnosticCode::SecurityExecCall,
    DiagnosticCode::SecurityPipeOpen,
    DiagnosticCode::SecurityReadpipe,
    // PL700-PL701: Import
    DiagnosticCode::UnusedImport,
    DiagnosticCode::ModuleNotFound,
    // PL800-PL806: Heredoc
    DiagnosticCode::HeredocInFormat,
    DiagnosticCode::HeredocInBegin,
    DiagnosticCode::HeredocDynamicDelimiter,
    DiagnosticCode::HeredocInSourceFilter,
    DiagnosticCode::HeredocInRegexCode,
    DiagnosticCode::HeredocInEval,
    DiagnosticCode::HeredocTiedHandle,
    // PL900: Version compatibility
    DiagnosticCode::VersionIncompatFeature,
    // PC001-PC005: Perl::Critic
    DiagnosticCode::CriticSeverity1,
    DiagnosticCode::CriticSeverity2,
    DiagnosticCode::CriticSeverity3,
    DiagnosticCode::CriticSeverity4,
    DiagnosticCode::CriticSeverity5,
];

/// All 60 variants are present in COMPLETE_ALL_CODES.
#[test]
fn complete_all_codes_has_60_variants() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        COMPLETE_ALL_CODES.len(),
        60,
        "expected 60 DiagnosticCode variants; update COMPLETE_ALL_CODES if the enum changed"
    );
    Ok(())
}

/// Every variant produces a non-empty code string with PL or PC prefix.
#[test]
fn all_variants_as_str_valid_prefix() -> Result<(), Box<dyn std::error::Error>> {
    for code in COMPLETE_ALL_CODES {
        let s = code.as_str();
        assert!(
            s.starts_with("PL") || s.starts_with("PC"),
            "{code:?}.as_str() = {s:?} — expected PL or PC prefix"
        );
        assert!(s.len() == 5, "{code:?}.as_str() = {s:?} — expected exactly 5 chars");
    }
    Ok(())
}

/// parse_code(as_str()) round-trip for every variant.
#[test]
fn all_variants_parse_code_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    for code in COMPLETE_ALL_CODES {
        let s = code.as_str();
        let parsed = DiagnosticCode::parse_code(s);
        assert_eq!(parsed, Some(*code), "parse_code({s:?}) should return Some({code:?})");
    }
    Ok(())
}

/// Every variant has a severity in the valid LSP range 1–4.
#[test]
fn all_variants_severity_in_valid_range() -> Result<(), Box<dyn std::error::Error>> {
    for code in COMPLETE_ALL_CODES {
        let sev = code.severity();
        let v = sev.to_lsp_value();
        assert!(
            (1..=4).contains(&v),
            "{code:?}.severity() = {sev:?} ({v}) — outside LSP range 1..=4"
        );
    }
    Ok(())
}

/// Every variant returns a valid category.
#[test]
fn all_variants_category_is_reachable() -> Result<(), Box<dyn std::error::Error>> {
    for code in COMPLETE_ALL_CODES {
        // Just call it — any panics or unreachable!() would surface here.
        let _cat = code.category();
    }
    Ok(())
}

/// PL codes have documentation URLs; PC codes do not.
#[test]
fn all_variants_documentation_url_pl_some_pc_none() -> Result<(), Box<dyn std::error::Error>> {
    for code in COMPLETE_ALL_CODES {
        let url = code.documentation_url();
        if code.as_str().starts_with("PL") {
            let url_str =
                url.ok_or_else(|| format!("{code:?} (PL code) should have a documentation URL"))?;
            assert!(
                url_str.starts_with("https://docs.perl-lsp.org/errors/"),
                "{code:?} URL has unexpected base: {url_str}"
            );
            assert!(
                url_str.ends_with(code.as_str()),
                "{code:?} URL should end with its code string: {url_str}"
            );
        } else {
            assert!(
                url.is_none(),
                "{code:?} (PC code) should return None for documentation_url(), got {url:?}"
            );
        }
    }
    Ok(())
}

/// context_hint() for every variant: PL codes return Some, PC codes return None.
#[test]
fn all_variants_context_hint_pl_some_pc_none() -> Result<(), Box<dyn std::error::Error>> {
    for code in COMPLETE_ALL_CODES {
        let hint = code.context_hint();
        if code.as_str().starts_with("PC") {
            assert!(
                hint.is_none(),
                "{code:?} (PC code) should return None for context_hint(), got {hint:?}"
            );
        } else {
            let text = hint.ok_or_else(|| {
                format!("{code:?} (PL code) should return Some for context_hint()")
            })?;
            assert!(!text.is_empty(), "{code:?}.context_hint() returned an empty string");
            assert!(
                text.len() >= 20,
                "{code:?}.context_hint() is too short ({} chars): {text:?}",
                text.len()
            );
        }
    }
    Ok(())
}

/// tags() returns a slice (possibly empty) — no panics for any variant.
#[test]
fn all_variants_tags_returns_slice() -> Result<(), Box<dyn std::error::Error>> {
    for code in COMPLETE_ALL_CODES {
        let _tags: &[DiagnosticTag] = code.tags();
    }
    Ok(())
}

/// Display impl: format!("{code}") == code.as_str() for every variant.
#[test]
fn all_variants_display_matches_as_str() -> Result<(), Box<dyn std::error::Error>> {
    for code in COMPLETE_ALL_CODES {
        assert_eq!(format!("{code}"), code.as_str(), "{code:?} Display mismatch");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// PL104–PL112: strict/warnings variants missing from prior ALL_CODES
// ---------------------------------------------------------------------------

mod strict_warnings_missing_variants {
    use super::*;

    #[test]
    fn variable_shadowing_pl104_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::VariableShadowing;
        assert_eq!(code.as_str(), "PL104");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::StrictWarnings);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL104"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL104"));
        Ok(())
    }

    #[test]
    fn variable_redeclaration_pl105_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::VariableRedeclaration;
        assert_eq!(code.as_str(), "PL105");
        assert_eq!(code.severity(), DiagnosticSeverity::Error);
        assert_eq!(code.category(), DiagnosticCategory::StrictWarnings);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL105"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL105"));
        Ok(())
    }

    #[test]
    fn duplicate_parameter_pl106_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::DuplicateParameter;
        assert_eq!(code.as_str(), "PL106");
        assert_eq!(code.severity(), DiagnosticSeverity::Error);
        assert_eq!(code.category(), DiagnosticCategory::StrictWarnings);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL106"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL106"));
        Ok(())
    }

    #[test]
    fn parameter_shadows_global_pl107_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::ParameterShadowsGlobal;
        assert_eq!(code.as_str(), "PL107");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::StrictWarnings);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL107"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL107"));
        Ok(())
    }

    #[test]
    fn unused_parameter_pl108_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::UnusedParameter;
        assert_eq!(code.as_str(), "PL108");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::StrictWarnings);
        assert!(code.context_hint().is_some());
        // UnusedParameter carries the Unnecessary tag
        assert_eq!(code.tags(), &[DiagnosticTag::Unnecessary]);
        assert_eq!(DiagnosticCode::parse_code("PL108"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL108"));
        Ok(())
    }

    #[test]
    fn unquoted_bareword_pl109_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::UnquotedBareword;
        assert_eq!(code.as_str(), "PL109");
        assert_eq!(code.severity(), DiagnosticSeverity::Error);
        assert_eq!(code.category(), DiagnosticCategory::StrictWarnings);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL109"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL109"));
        Ok(())
    }

    #[test]
    fn uninitialized_variable_pl110_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::UninitializedVariable;
        assert_eq!(code.as_str(), "PL110");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::StrictWarnings);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL110"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL110"));
        Ok(())
    }

    #[test]
    fn misspelled_pragma_pl111_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::MisspelledPragma;
        assert_eq!(code.as_str(), "PL111");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::StrictWarnings);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL111"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL111"));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PL303–PL304: subroutine variants not in prior comprehensive test
// ---------------------------------------------------------------------------

mod subroutine_missing_variants {
    use super::*;

    #[test]
    fn missing_pod_coverage_pl304_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::MissingPodCoverage;
        assert_eq!(code.as_str(), "PL304");
        assert_eq!(code.severity(), DiagnosticSeverity::Hint);
        assert_eq!(code.category(), DiagnosticCategory::Subroutine);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL304"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL304"));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PL403–PL410: best-practices variants missing from prior comprehensive test
// ---------------------------------------------------------------------------

mod best_practices_missing_variants {
    use super::*;

    #[test]
    fn assignment_in_condition_pl403_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::AssignmentInCondition;
        assert_eq!(code.as_str(), "PL403");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::BestPractices);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL403"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL403"));
        Ok(())
    }

    #[test]
    fn numeric_comparison_with_undef_pl404_full_coverage() -> Result<(), Box<dyn std::error::Error>>
    {
        let code = DiagnosticCode::NumericComparisonWithUndef;
        assert_eq!(code.as_str(), "PL404");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::BestPractices);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL404"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL404"));
        Ok(())
    }

    #[test]
    fn unreachable_code_pl406_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::UnreachableCode;
        assert_eq!(code.as_str(), "PL406");
        assert_eq!(code.severity(), DiagnosticSeverity::Hint);
        assert_eq!(code.category(), DiagnosticCategory::BestPractices);
        assert!(code.context_hint().is_some());
        // UnreachableCode carries the Unnecessary tag
        assert_eq!(code.tags(), &[DiagnosticTag::Unnecessary]);
        assert_eq!(DiagnosticCode::parse_code("PL406"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL406"));
        Ok(())
    }

    #[test]
    fn eval_error_flow_pl407_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::EvalErrorFlow;
        assert_eq!(code.as_str(), "PL407");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::BestPractices);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL407"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL407"));
        Ok(())
    }

    #[test]
    fn duplicate_hash_key_pl408_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::DuplicateHashKey;
        assert_eq!(code.as_str(), "PL408");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::BestPractices);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL408"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL408"));
        Ok(())
    }

    #[test]
    fn goto_undefined_label_pl409_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::GotoUndefinedLabel;
        assert_eq!(code.as_str(), "PL409");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::BestPractices);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL409"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL409"));
        Ok(())
    }

    #[test]
    fn loop_control_undefined_label_pl410_full_coverage() -> Result<(), Box<dyn std::error::Error>>
    {
        let code = DiagnosticCode::LoopControlUndefinedLabel;
        assert_eq!(code.as_str(), "PL410");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::BestPractices);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL410"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL410"));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PL500–PL503: deprecated syntax + phase-scoped pragma variants
// ---------------------------------------------------------------------------

mod deprecated_missing_variants {
    use super::*;

    #[test]
    fn deprecated_defined_pl500_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::DeprecatedDefined;
        assert_eq!(code.as_str(), "PL500");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::Deprecated);
        assert!(code.context_hint().is_some());
        // DeprecatedDefined carries the Deprecated tag
        assert_eq!(code.tags(), &[DiagnosticTag::Deprecated]);
        assert_eq!(DiagnosticCode::parse_code("PL500"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL500"));
        Ok(())
    }

    #[test]
    fn deprecated_array_base_pl501_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::DeprecatedArrayBase;
        assert_eq!(code.as_str(), "PL501");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::Deprecated);
        assert!(code.context_hint().is_some());
        // DeprecatedArrayBase carries the Deprecated tag
        assert_eq!(code.tags(), &[DiagnosticTag::Deprecated]);
        assert_eq!(DiagnosticCode::parse_code("PL501"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL501"));
        Ok(())
    }

    #[test]
    fn phase_scoped_strict_pragma_pl502_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::PhaseScopedStrictPragma;
        assert_eq!(code.as_str(), "PL502");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        // Phase-scoped pragmas are categorised as StrictWarnings (not Deprecated)
        assert_eq!(code.category(), DiagnosticCategory::StrictWarnings);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL502"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL502"));
        Ok(())
    }

    #[test]
    fn phase_scoped_warnings_pragma_pl503_full_coverage() -> Result<(), Box<dyn std::error::Error>>
    {
        let code = DiagnosticCode::PhaseScopedWarningsPragma;
        assert_eq!(code.as_str(), "PL503");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::StrictWarnings);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL503"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL503"));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PL600–PL606: security variants (only PL602 was in existing suite)
// ---------------------------------------------------------------------------

mod security_missing_variants {
    use super::*;

    #[test]
    fn security_string_eval_pl600_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::SecurityStringEval;
        assert_eq!(code.as_str(), "PL600");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::Security);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL600"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL600"));
        Ok(())
    }

    #[test]
    fn security_backtick_exec_pl601_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::SecurityBacktickExec;
        assert_eq!(code.as_str(), "PL601");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::Security);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL601"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL601"));
        Ok(())
    }

    #[test]
    fn security_system_call_pl603_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::SecuritySystemCall;
        assert_eq!(code.as_str(), "PL603");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::Security);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL603"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL603"));
        Ok(())
    }

    #[test]
    fn security_exec_call_pl604_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::SecurityExecCall;
        assert_eq!(code.as_str(), "PL604");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::Security);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL604"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL604"));
        Ok(())
    }

    #[test]
    fn security_pipe_open_pl605_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::SecurityPipeOpen;
        assert_eq!(code.as_str(), "PL605");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::Security);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL605"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL605"));
        Ok(())
    }

    #[test]
    fn security_readpipe_pl606_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::SecurityReadpipe;
        assert_eq!(code.as_str(), "PL606");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::Security);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL606"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL606"));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PL700–PL701: import variants
// ---------------------------------------------------------------------------

mod import_missing_variants {
    use super::*;

    #[test]
    fn unused_import_pl700_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::UnusedImport;
        assert_eq!(code.as_str(), "PL700");
        assert_eq!(code.severity(), DiagnosticSeverity::Hint);
        assert_eq!(code.category(), DiagnosticCategory::Import);
        assert!(code.context_hint().is_some());
        // UnusedImport carries the Unnecessary tag
        assert_eq!(code.tags(), &[DiagnosticTag::Unnecessary]);
        assert_eq!(DiagnosticCode::parse_code("PL700"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL700"));
        Ok(())
    }

    #[test]
    fn module_not_found_pl701_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::ModuleNotFound;
        assert_eq!(code.as_str(), "PL701");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::Import);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL701"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL701"));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PL800–PL806: heredoc anti-pattern variants
// ---------------------------------------------------------------------------

mod heredoc_missing_variants {
    use super::*;

    #[test]
    fn heredoc_in_format_pl800_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::HeredocInFormat;
        assert_eq!(code.as_str(), "PL800");
        assert_eq!(code.severity(), DiagnosticSeverity::Information);
        assert_eq!(code.category(), DiagnosticCategory::Heredoc);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL800"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL800"));
        Ok(())
    }

    #[test]
    fn heredoc_in_begin_pl801_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::HeredocInBegin;
        assert_eq!(code.as_str(), "PL801");
        assert_eq!(code.severity(), DiagnosticSeverity::Information);
        assert_eq!(code.category(), DiagnosticCategory::Heredoc);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL801"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL801"));
        Ok(())
    }

    #[test]
    fn heredoc_dynamic_delimiter_pl802_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::HeredocDynamicDelimiter;
        assert_eq!(code.as_str(), "PL802");
        assert_eq!(code.severity(), DiagnosticSeverity::Information);
        assert_eq!(code.category(), DiagnosticCategory::Heredoc);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL802"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL802"));
        Ok(())
    }

    #[test]
    fn heredoc_in_source_filter_pl803_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::HeredocInSourceFilter;
        assert_eq!(code.as_str(), "PL803");
        assert_eq!(code.severity(), DiagnosticSeverity::Information);
        assert_eq!(code.category(), DiagnosticCategory::Heredoc);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL803"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL803"));
        Ok(())
    }

    #[test]
    fn heredoc_in_regex_code_pl804_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::HeredocInRegexCode;
        assert_eq!(code.as_str(), "PL804");
        assert_eq!(code.severity(), DiagnosticSeverity::Information);
        assert_eq!(code.category(), DiagnosticCategory::Heredoc);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL804"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL804"));
        Ok(())
    }

    #[test]
    fn heredoc_in_eval_pl805_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::HeredocInEval;
        assert_eq!(code.as_str(), "PL805");
        assert_eq!(code.severity(), DiagnosticSeverity::Information);
        assert_eq!(code.category(), DiagnosticCategory::Heredoc);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL805"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL805"));
        Ok(())
    }

    #[test]
    fn heredoc_tied_handle_pl806_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::HeredocTiedHandle;
        assert_eq!(code.as_str(), "PL806");
        assert_eq!(code.severity(), DiagnosticSeverity::Information);
        assert_eq!(code.category(), DiagnosticCategory::Heredoc);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL806"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL806"));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PL900: version compatibility variant
// ---------------------------------------------------------------------------

mod version_compat_variant {
    use super::*;

    #[test]
    fn version_incompat_feature_pl900_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let code = DiagnosticCode::VersionIncompatFeature;
        assert_eq!(code.as_str(), "PL900");
        assert_eq!(code.severity(), DiagnosticSeverity::Warning);
        assert_eq!(code.category(), DiagnosticCategory::BestPractices);
        assert!(code.context_hint().is_some());
        assert!(code.tags().is_empty());
        assert_eq!(DiagnosticCode::parse_code("PL900"), Some(code));
        assert_eq!(code.documentation_url(), Some("https://docs.perl-lsp.org/errors/PL900"));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tags: exhaustive tag validation for every variant
// ---------------------------------------------------------------------------

mod tags_exhaustive {
    use super::*;

    /// Variants that carry the Unnecessary tag.
    #[test]
    fn unnecessary_tag_variants() -> Result<(), Box<dyn std::error::Error>> {
        let unnecessary = [
            DiagnosticCode::UnusedVariable,
            DiagnosticCode::UnusedParameter,
            DiagnosticCode::UnusedImport,
            DiagnosticCode::UnreachableCode,
        ];
        for code in &unnecessary {
            let tags = code.tags();
            assert_eq!(
                tags,
                &[DiagnosticTag::Unnecessary],
                "{code:?} should carry exactly the Unnecessary tag"
            );
        }
        Ok(())
    }

    /// Variants that carry the Deprecated tag.
    #[test]
    fn deprecated_tag_variants() -> Result<(), Box<dyn std::error::Error>> {
        let deprecated_codes =
            [DiagnosticCode::DeprecatedDefined, DiagnosticCode::DeprecatedArrayBase];
        for code in &deprecated_codes {
            let tags = code.tags();
            assert_eq!(
                tags,
                &[DiagnosticTag::Deprecated],
                "{code:?} should carry exactly the Deprecated tag"
            );
        }
        Ok(())
    }

    /// All other variants have empty tags slices.
    #[test]
    fn all_other_variants_have_empty_tags() -> Result<(), Box<dyn std::error::Error>> {
        let tagged: std::collections::HashSet<DiagnosticCode> = [
            DiagnosticCode::UnusedVariable,
            DiagnosticCode::UnusedParameter,
            DiagnosticCode::UnusedImport,
            DiagnosticCode::UnreachableCode,
            DiagnosticCode::DeprecatedDefined,
            DiagnosticCode::DeprecatedArrayBase,
        ]
        .into_iter()
        .collect();

        for code in COMPLETE_ALL_CODES {
            if !tagged.contains(code) {
                assert!(
                    code.tags().is_empty(),
                    "{code:?} should have empty tags but has {:?}",
                    code.tags()
                );
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Category display: exercise every DiagnosticCategory Display arm
// ---------------------------------------------------------------------------

mod category_display {
    use super::*;

    #[test]
    fn all_category_display_arms() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(format!("{}", DiagnosticCategory::Parser), "Parser");
        assert_eq!(format!("{}", DiagnosticCategory::StrictWarnings), "Strict/Warnings");
        assert_eq!(format!("{}", DiagnosticCategory::PackageModule), "Package/Module");
        assert_eq!(format!("{}", DiagnosticCategory::Subroutine), "Subroutine");
        assert_eq!(format!("{}", DiagnosticCategory::BestPractices), "Best Practices");
        assert_eq!(format!("{}", DiagnosticCategory::Deprecated), "Deprecated");
        assert_eq!(format!("{}", DiagnosticCategory::Security), "Security");
        assert_eq!(format!("{}", DiagnosticCategory::Import), "Import");
        assert_eq!(format!("{}", DiagnosticCategory::Heredoc), "Heredoc");
        assert_eq!(format!("{}", DiagnosticCategory::PerlCritic), "Perl::Critic");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// from_message: exercise the phase-scoped-pragma arms (highest priority arms)
// ---------------------------------------------------------------------------

mod from_message_phase_scoped {
    use super::*;

    #[test]
    fn from_message_inside_begin_block_does_not_enable_strict()
    -> Result<(), Box<dyn std::error::Error>> {
        let msg = "use strict inside a begin block does not enable strict for file scope";
        assert_eq!(
            DiagnosticCode::from_message(msg),
            Some(DiagnosticCode::PhaseScopedStrictPragma)
        );
        Ok(())
    }

    #[test]
    fn from_message_inside_phase_block_does_not_enable_strict()
    -> Result<(), Box<dyn std::error::Error>> {
        let msg = "pragma inside a phase block does not enable strict";
        assert_eq!(
            DiagnosticCode::from_message(msg),
            Some(DiagnosticCode::PhaseScopedStrictPragma)
        );
        Ok(())
    }

    #[test]
    fn from_message_inside_begin_block_does_not_enable_warnings()
    -> Result<(), Box<dyn std::error::Error>> {
        let msg = "use warnings inside a begin block does not enable warnings";
        assert_eq!(
            DiagnosticCode::from_message(msg),
            Some(DiagnosticCode::PhaseScopedWarningsPragma)
        );
        Ok(())
    }

    #[test]
    fn from_message_inside_phase_block_does_not_enable_warnings()
    -> Result<(), Box<dyn std::error::Error>> {
        let msg = "pragma inside a phase block does not enable warnings for this scope";
        assert_eq!(
            DiagnosticCode::from_message(msg),
            Some(DiagnosticCode::PhaseScopedWarningsPragma)
        );
        Ok(())
    }

    #[test]
    fn from_message_invalid_prototype_arm() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            DiagnosticCode::from_message("invalid prototype character found"),
            Some(DiagnosticCode::InvalidPrototype)
        );
        assert_eq!(
            DiagnosticCode::from_message("illegal character in prototype '$foo'"),
            Some(DiagnosticCode::InvalidPrototype)
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// DiagnosticCategory: all variants reachable via category() calls
// ---------------------------------------------------------------------------

mod category_completeness {
    use super::*;

    /// Every category value is produced by at least one code.
    #[test]
    fn all_ten_categories_produced_by_complete_codes() -> Result<(), Box<dyn std::error::Error>> {
        use std::collections::HashSet;
        let categories: HashSet<DiagnosticCategory> =
            COMPLETE_ALL_CODES.iter().map(|c| c.category()).collect();

        let expected = [
            DiagnosticCategory::Parser,
            DiagnosticCategory::StrictWarnings,
            DiagnosticCategory::PackageModule,
            DiagnosticCategory::Subroutine,
            DiagnosticCategory::BestPractices,
            DiagnosticCategory::Deprecated,
            DiagnosticCategory::Security,
            DiagnosticCategory::Import,
            DiagnosticCategory::Heredoc,
            DiagnosticCategory::PerlCritic,
        ];
        for cat in &expected {
            assert!(
                categories.contains(cat),
                "category {cat:?} should be produced by at least one code"
            );
        }
        assert_eq!(categories.len(), 10, "expected exactly 10 categories");
        Ok(())
    }
}
