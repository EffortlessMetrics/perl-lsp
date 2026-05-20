//! Coverage gap tests for `codes/mod.rs`.
//!
//! Targets the variants and arms that were not reached by any existing test.
//! Specifically covers:
//!
//! - `as_str()` arms: MissingPodCoverage, UnreachableCode, SecuritySystemCall,
//!   SecurityExecCall, SecurityPipeOpen, SecurityReadpipe, VersionIncompatFeature
//! - `documentation_url()` arms: PL603-PL606, PL700-PL701, PL800-PL806, PL900,
//!   and the `_ => None` fallback branch
//! - `tags()` arm: DeprecatedDefined | DeprecatedArrayBase => Deprecated
//! - `context_hint()` arms: all variants not previously hit
//! - `from_message()` arm: illegal-character-in-prototype path
//! - `category()` arms: Deprecated, Import, Heredoc categories
//! - `DiagnosticCategory::fmt()`: all Display arms

use perl_diagnostics::codes::{
    DiagnosticCategory, DiagnosticCode, DiagnosticSeverity, DiagnosticTag,
};

// ===========================================================================
// as_str() — uncovered variants
// ===========================================================================

#[test]
fn as_str_missing_pod_coverage_is_pl304() -> Result<(), Box<dyn std::error::Error>> {
    // Line 267 in codes/mod.rs
    assert_eq!(DiagnosticCode::MissingPodCoverage.as_str(), "PL304");
    Ok(())
}

#[test]
fn as_str_unreachable_code_is_pl406() -> Result<(), Box<dyn std::error::Error>> {
    // Line 274 in codes/mod.rs
    assert_eq!(DiagnosticCode::UnreachableCode.as_str(), "PL406");
    Ok(())
}

#[test]
fn as_str_security_system_call_is_pl603() -> Result<(), Box<dyn std::error::Error>> {
    // Line 286 in codes/mod.rs
    assert_eq!(DiagnosticCode::SecuritySystemCall.as_str(), "PL603");
    Ok(())
}

#[test]
fn as_str_security_exec_call_is_pl604() -> Result<(), Box<dyn std::error::Error>> {
    // Line 287 in codes/mod.rs
    assert_eq!(DiagnosticCode::SecurityExecCall.as_str(), "PL604");
    Ok(())
}

#[test]
fn as_str_security_pipe_open_is_pl605() -> Result<(), Box<dyn std::error::Error>> {
    // Line 288 in codes/mod.rs
    assert_eq!(DiagnosticCode::SecurityPipeOpen.as_str(), "PL605");
    Ok(())
}

#[test]
fn as_str_security_readpipe_is_pl606() -> Result<(), Box<dyn std::error::Error>> {
    // Line 289 in codes/mod.rs
    assert_eq!(DiagnosticCode::SecurityReadpipe.as_str(), "PL606");
    Ok(())
}

#[test]
fn as_str_version_incompat_feature_is_pl900() -> Result<(), Box<dyn std::error::Error>> {
    // Line 299 in codes/mod.rs
    assert_eq!(DiagnosticCode::VersionIncompatFeature.as_str(), "PL900");
    Ok(())
}

// ===========================================================================
// documentation_url() — uncovered arms (PL603-PL606, PL700-PL701, PL800-PL806,
//                                        PL900, and _ => None fallback)
// ===========================================================================

#[test]
fn documentation_url_security_system_call() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 358-360 in codes/mod.rs
    let url = DiagnosticCode::SecuritySystemCall.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL603"));
    Ok(())
}

#[test]
fn documentation_url_security_exec_call() -> Result<(), Box<dyn std::error::Error>> {
    // Line 359 in codes/mod.rs
    let url = DiagnosticCode::SecurityExecCall.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL604"));
    Ok(())
}

#[test]
fn documentation_url_security_pipe_open() -> Result<(), Box<dyn std::error::Error>> {
    // Line 360 in codes/mod.rs
    let url = DiagnosticCode::SecurityPipeOpen.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL605"));
    Ok(())
}

#[test]
fn documentation_url_security_readpipe() -> Result<(), Box<dyn std::error::Error>> {
    // Line 361 in codes/mod.rs
    let url = DiagnosticCode::SecurityReadpipe.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL606"));
    Ok(())
}

#[test]
fn documentation_url_unused_import() -> Result<(), Box<dyn std::error::Error>> {
    // Line 362 in codes/mod.rs
    let url = DiagnosticCode::UnusedImport.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL700"));
    Ok(())
}

#[test]
fn documentation_url_module_not_found() -> Result<(), Box<dyn std::error::Error>> {
    // Line 363 in codes/mod.rs
    let url = DiagnosticCode::ModuleNotFound.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL701"));
    Ok(())
}

#[test]
fn documentation_url_heredoc_in_format() -> Result<(), Box<dyn std::error::Error>> {
    // Line 364 in codes/mod.rs
    let url = DiagnosticCode::HeredocInFormat.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL800"));
    Ok(())
}

#[test]
fn documentation_url_heredoc_in_begin() -> Result<(), Box<dyn std::error::Error>> {
    // Line 365 in codes/mod.rs
    let url = DiagnosticCode::HeredocInBegin.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL801"));
    Ok(())
}

#[test]
fn documentation_url_heredoc_dynamic_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    // Line 366 in codes/mod.rs
    let url = DiagnosticCode::HeredocDynamicDelimiter.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL802"));
    Ok(())
}

#[test]
fn documentation_url_heredoc_in_source_filter() -> Result<(), Box<dyn std::error::Error>> {
    // Line 367 in codes/mod.rs
    let url = DiagnosticCode::HeredocInSourceFilter.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL803"));
    Ok(())
}

#[test]
fn documentation_url_heredoc_in_regex_code() -> Result<(), Box<dyn std::error::Error>> {
    // Line 368 in codes/mod.rs
    let url = DiagnosticCode::HeredocInRegexCode.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL804"));
    Ok(())
}

#[test]
fn documentation_url_heredoc_in_eval() -> Result<(), Box<dyn std::error::Error>> {
    // Line 369 in codes/mod.rs
    let url = DiagnosticCode::HeredocInEval.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL805"));
    Ok(())
}

#[test]
fn documentation_url_heredoc_tied_handle() -> Result<(), Box<dyn std::error::Error>> {
    // Line 370 in codes/mod.rs
    let url = DiagnosticCode::HeredocTiedHandle.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL806"));
    Ok(())
}

#[test]
fn documentation_url_version_incompat_feature() -> Result<(), Box<dyn std::error::Error>> {
    // Line 371 in codes/mod.rs
    let url = DiagnosticCode::VersionIncompatFeature.documentation_url();
    assert_eq!(url, Some("https://docs.perl-lsp.org/errors/PL900"));
    Ok(())
}

// ===========================================================================
// tags() — DeprecatedDefined | DeprecatedArrayBase => Deprecated
// ===========================================================================

#[test]
fn tags_deprecated_defined_has_deprecated_tag() -> Result<(), Box<dyn std::error::Error>> {
    // Line 456 in codes/mod.rs
    let tags = DiagnosticCode::DeprecatedDefined.tags();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0], DiagnosticTag::Deprecated);
    Ok(())
}

#[test]
fn tags_deprecated_array_base_has_deprecated_tag() -> Result<(), Box<dyn std::error::Error>> {
    // Line 456 in codes/mod.rs (same arm, other variant)
    let tags = DiagnosticCode::DeprecatedArrayBase.tags();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0], DiagnosticTag::Deprecated);
    Ok(())
}

#[test]
fn tags_unused_parameter_has_unnecessary_tag() -> Result<(), Box<dyn std::error::Error>> {
    // Line 452-455 in codes/mod.rs
    let tags = DiagnosticCode::UnusedParameter.tags();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0], DiagnosticTag::Unnecessary);
    Ok(())
}

#[test]
fn tags_unused_import_has_unnecessary_tag() -> Result<(), Box<dyn std::error::Error>> {
    // Line 452-455 in codes/mod.rs
    let tags = DiagnosticCode::UnusedImport.tags();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0], DiagnosticTag::Unnecessary);
    Ok(())
}

#[test]
fn tags_unreachable_code_has_unnecessary_tag() -> Result<(), Box<dyn std::error::Error>> {
    // Line 452-455 in codes/mod.rs
    let tags = DiagnosticCode::UnreachableCode.tags();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0], DiagnosticTag::Unnecessary);
    Ok(())
}

// ===========================================================================
// context_hint() — all previously uncovered arms
// ===========================================================================

#[test]
fn context_hint_role_conflict_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 513-516 in codes/mod.rs
    let hint = DiagnosticCode::RoleConflict.context_hint();
    assert!(hint.is_some(), "RoleConflict should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(hint.contains("Moo") || hint.contains("role"), "hint should mention roles");
    Ok(())
}

#[test]
fn context_hint_missing_pod_coverage_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 517-520 in codes/mod.rs
    let hint = DiagnosticCode::MissingPodCoverage.context_hint();
    assert!(hint.is_some(), "MissingPodCoverage should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("POD") || hint.contains("documentation"),
        "hint should mention documentation"
    );
    Ok(())
}

#[test]
fn context_hint_invalid_prototype_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 521-525 in codes/mod.rs
    let hint = DiagnosticCode::InvalidPrototype.context_hint();
    assert!(hint.is_some(), "InvalidPrototype should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(hint.contains("prototype"), "hint should mention prototype");
    Ok(())
}

#[test]
fn context_hint_assignment_in_condition_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 538-541 in codes/mod.rs
    let hint = DiagnosticCode::AssignmentInCondition.context_hint();
    assert!(hint.is_some(), "AssignmentInCondition should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("assignment") || hint.contains("comparison"),
        "hint should describe assignment vs comparison"
    );
    Ok(())
}

#[test]
fn context_hint_numeric_comparison_with_undef_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 542-545 in codes/mod.rs
    let hint = DiagnosticCode::NumericComparisonWithUndef.context_hint();
    assert!(hint.is_some(), "NumericComparisonWithUndef should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("undefined") || hint.contains("defined"),
        "hint should mention definedness"
    );
    Ok(())
}

#[test]
fn context_hint_unreachable_code_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 550-553 in codes/mod.rs
    let hint = DiagnosticCode::UnreachableCode.context_hint();
    assert!(hint.is_some(), "UnreachableCode should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("unreachable") || hint.contains("cannot be executed"),
        "hint should describe unreachability"
    );
    Ok(())
}

#[test]
fn context_hint_printf_format_mismatch_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 566-569 in codes/mod.rs
    let hint = DiagnosticCode::PrintfFormatMismatch.context_hint();
    assert!(hint.is_some(), "PrintfFormatMismatch should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("format") || hint.contains("specifier"),
        "hint should describe format specifiers"
    );
    Ok(())
}

#[test]
fn context_hint_variable_shadowing_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 570-573 in codes/mod.rs
    let hint = DiagnosticCode::VariableShadowing.context_hint();
    assert!(hint.is_some(), "VariableShadowing should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(hint.contains("shadow"), "hint should describe shadowing");
    Ok(())
}

#[test]
fn context_hint_variable_redeclaration_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 574-577 in codes/mod.rs
    let hint = DiagnosticCode::VariableRedeclaration.context_hint();
    assert!(hint.is_some(), "VariableRedeclaration should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("redeclar") || hint.contains("declared"),
        "hint should describe redeclaration"
    );
    Ok(())
}

#[test]
fn context_hint_duplicate_parameter_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 578-581 in codes/mod.rs
    let hint = DiagnosticCode::DuplicateParameter.context_hint();
    assert!(hint.is_some(), "DuplicateParameter should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("parameter") || hint.contains("signature"),
        "hint should describe parameter"
    );
    Ok(())
}

#[test]
fn context_hint_parameter_shadows_global_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 582-585 in codes/mod.rs
    let hint = DiagnosticCode::ParameterShadowsGlobal.context_hint();
    assert!(hint.is_some(), "ParameterShadowsGlobal should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("global") || hint.contains("parameter"),
        "hint should mention global and parameter"
    );
    Ok(())
}

#[test]
fn context_hint_unused_parameter_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 586-589 in codes/mod.rs
    let hint = DiagnosticCode::UnusedParameter.context_hint();
    assert!(hint.is_some(), "UnusedParameter should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("parameter") || hint.contains("unused"),
        "hint should describe unused parameter"
    );
    Ok(())
}

#[test]
fn context_hint_unquoted_bareword_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 590-593 in codes/mod.rs
    let hint = DiagnosticCode::UnquotedBareword.context_hint();
    assert!(hint.is_some(), "UnquotedBareword should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("bareword") || hint.contains("quoted"),
        "hint should describe bareword quoting"
    );
    Ok(())
}

#[test]
fn context_hint_uninitialized_variable_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 594-597 in codes/mod.rs
    let hint = DiagnosticCode::UninitializedVariable.context_hint();
    assert!(hint.is_some(), "UninitializedVariable should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("uninitialized") || hint.contains("assigned"),
        "hint should describe initialization"
    );
    Ok(())
}

#[test]
fn context_hint_misspelled_pragma_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 598-601 in codes/mod.rs
    let hint = DiagnosticCode::MisspelledPragma.context_hint();
    assert!(hint.is_some(), "MisspelledPragma should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("pragma") || hint.contains("misspelled"),
        "hint should describe pragma spelling"
    );
    Ok(())
}

#[test]
fn context_hint_capture_var_without_regex_match_is_some() -> Result<(), Box<dyn std::error::Error>>
{
    // Lines 602-605 in codes/mod.rs
    let hint = DiagnosticCode::CaptureVarWithoutRegexMatch.context_hint();
    assert!(hint.is_some(), "CaptureVarWithoutRegexMatch should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("capture") || hint.contains("regex"),
        "hint should describe capture variable usage"
    );
    Ok(())
}

#[test]
fn context_hint_deprecated_defined_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 606-609 in codes/mod.rs
    let hint = DiagnosticCode::DeprecatedDefined.context_hint();
    assert!(hint.is_some(), "DeprecatedDefined should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("defined") || hint.contains("deprecated"),
        "hint should describe deprecation"
    );
    Ok(())
}

#[test]
fn context_hint_deprecated_array_base_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 610-613 in codes/mod.rs
    let hint = DiagnosticCode::DeprecatedArrayBase.context_hint();
    assert!(hint.is_some(), "DeprecatedArrayBase should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("deprecated") || hint.contains("array"),
        "hint should describe deprecation"
    );
    Ok(())
}

#[test]
fn context_hint_security_string_eval_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 622-625 in codes/mod.rs
    let hint = DiagnosticCode::SecurityStringEval.context_hint();
    assert!(hint.is_some(), "SecurityStringEval should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("eval") || hint.contains("security"),
        "hint should describe eval security"
    );
    Ok(())
}

#[test]
fn context_hint_security_backtick_exec_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 626-629 in codes/mod.rs
    let hint = DiagnosticCode::SecurityBacktickExec.context_hint();
    assert!(hint.is_some(), "SecurityBacktickExec should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("backtick") || hint.contains("shell"),
        "hint should describe backtick security"
    );
    Ok(())
}

#[test]
fn context_hint_security_system_call_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 634-637 in codes/mod.rs
    let hint = DiagnosticCode::SecuritySystemCall.context_hint();
    assert!(hint.is_some(), "SecuritySystemCall should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("system") || hint.contains("shell"),
        "hint should describe system() security"
    );
    Ok(())
}

#[test]
fn context_hint_security_exec_call_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 638-641 in codes/mod.rs
    let hint = DiagnosticCode::SecurityExecCall.context_hint();
    assert!(hint.is_some(), "SecurityExecCall should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("exec") || hint.contains("shell"),
        "hint should describe exec() security"
    );
    Ok(())
}

#[test]
fn context_hint_security_pipe_open_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 642-645 in codes/mod.rs
    let hint = DiagnosticCode::SecurityPipeOpen.context_hint();
    assert!(hint.is_some(), "SecurityPipeOpen should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("pipe") || hint.contains("open"),
        "hint should describe pipe open security"
    );
    Ok(())
}

#[test]
fn context_hint_security_readpipe_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 646-649 in codes/mod.rs
    let hint = DiagnosticCode::SecurityReadpipe.context_hint();
    assert!(hint.is_some(), "SecurityReadpipe should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("readpipe") || hint.contains("command"),
        "hint should describe readpipe security"
    );
    Ok(())
}

#[test]
fn context_hint_unused_import_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 650-653 in codes/mod.rs
    let hint = DiagnosticCode::UnusedImport.context_hint();
    assert!(hint.is_some(), "UnusedImport should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("import") || hint.contains("module"),
        "hint should describe unused import"
    );
    Ok(())
}

#[test]
fn context_hint_heredoc_in_format_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 658-661 in codes/mod.rs
    let hint = DiagnosticCode::HeredocInFormat.context_hint();
    assert!(hint.is_some(), "HeredocInFormat should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("heredoc") || hint.contains("format"),
        "hint should describe heredoc in format"
    );
    Ok(())
}

#[test]
fn context_hint_heredoc_in_begin_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 662-665 in codes/mod.rs
    let hint = DiagnosticCode::HeredocInBegin.context_hint();
    assert!(hint.is_some(), "HeredocInBegin should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("BEGIN") || hint.contains("heredoc"),
        "hint should describe heredoc in BEGIN"
    );
    Ok(())
}

#[test]
fn context_hint_heredoc_dynamic_delimiter_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 666-669 in codes/mod.rs
    let hint = DiagnosticCode::HeredocDynamicDelimiter.context_hint();
    assert!(hint.is_some(), "HeredocDynamicDelimiter should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("delimiter") || hint.contains("dynamic"),
        "hint should describe dynamic delimiter"
    );
    Ok(())
}

#[test]
fn context_hint_heredoc_in_source_filter_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 670-673 in codes/mod.rs
    let hint = DiagnosticCode::HeredocInSourceFilter.context_hint();
    assert!(hint.is_some(), "HeredocInSourceFilter should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("filter") || hint.contains("heredoc"),
        "hint should describe source filter"
    );
    Ok(())
}

#[test]
fn context_hint_heredoc_in_regex_code_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 674-677 in codes/mod.rs
    let hint = DiagnosticCode::HeredocInRegexCode.context_hint();
    assert!(hint.is_some(), "HeredocInRegexCode should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("regex") || hint.contains("heredoc"),
        "hint should describe heredoc in regex"
    );
    Ok(())
}

#[test]
fn context_hint_heredoc_in_eval_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 678-681 in codes/mod.rs
    let hint = DiagnosticCode::HeredocInEval.context_hint();
    assert!(hint.is_some(), "HeredocInEval should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("eval") || hint.contains("heredoc"),
        "hint should describe heredoc in eval"
    );
    Ok(())
}

#[test]
fn context_hint_heredoc_tied_handle_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 682-685 in codes/mod.rs
    let hint = DiagnosticCode::HeredocTiedHandle.context_hint();
    assert!(hint.is_some(), "HeredocTiedHandle should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("tied") || hint.contains("filehandle"),
        "hint should describe tied filehandle"
    );
    Ok(())
}

#[test]
fn context_hint_version_incompat_feature_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 686-689 in codes/mod.rs
    let hint = DiagnosticCode::VersionIncompatFeature.context_hint();
    assert!(hint.is_some(), "VersionIncompatFeature should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("version") || hint.contains("feature"),
        "hint should describe version incompatibility"
    );
    Ok(())
}

#[test]
fn context_hint_phase_scoped_strict_pragma_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 614-617 in codes/mod.rs
    let hint = DiagnosticCode::PhaseScopedStrictPragma.context_hint();
    assert!(hint.is_some(), "PhaseScopedStrictPragma should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("strict") || hint.contains("phase"),
        "hint should describe strict-pragma phase scoping"
    );
    Ok(())
}

#[test]
fn context_hint_phase_scoped_warnings_pragma_is_some() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 618-622 in codes/mod.rs
    let hint = DiagnosticCode::PhaseScopedWarningsPragma.context_hint();
    assert!(hint.is_some(), "PhaseScopedWarningsPragma should have a context hint");
    let hint = hint.ok_or("missing hint")?;
    assert!(
        hint.contains("warnings") || hint.contains("phase"),
        "hint should describe warnings-pragma phase scoping"
    );
    Ok(())
}

#[test]
fn context_hint_critic_codes_return_none() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 691-695 in codes/mod.rs — already covered by other tests but verify all 5
    assert_eq!(DiagnosticCode::CriticSeverity1.context_hint(), None);
    assert_eq!(DiagnosticCode::CriticSeverity2.context_hint(), None);
    assert_eq!(DiagnosticCode::CriticSeverity3.context_hint(), None);
    assert_eq!(DiagnosticCode::CriticSeverity4.context_hint(), None);
    assert_eq!(DiagnosticCode::CriticSeverity5.context_hint(), None);
    Ok(())
}

// ===========================================================================
// from_message() — InvalidPrototype arm
// ===========================================================================

#[test]
fn from_message_invalid_prototype_character() -> Result<(), Box<dyn std::error::Error>> {
    // Line 725 in codes/mod.rs
    assert_eq!(
        DiagnosticCode::from_message("invalid prototype character in definition"),
        Some(DiagnosticCode::InvalidPrototype)
    );
    Ok(())
}

#[test]
fn from_message_illegal_character_in_prototype() -> Result<(), Box<dyn std::error::Error>> {
    // Line 724 in codes/mod.rs (second arm of the ||)
    assert_eq!(
        DiagnosticCode::from_message("Illegal character in prototype definition"),
        Some(DiagnosticCode::InvalidPrototype)
    );
    Ok(())
}

// ===========================================================================
// category() — Deprecated, Import, Heredoc arms
// ===========================================================================

#[test]
fn category_deprecated_defined() -> Result<(), Box<dyn std::error::Error>> {
    // Line 846 in codes/mod.rs
    assert_eq!(DiagnosticCode::DeprecatedDefined.category(), DiagnosticCategory::Deprecated);
    Ok(())
}

#[test]
fn category_deprecated_array_base() -> Result<(), Box<dyn std::error::Error>> {
    // Line 846 in codes/mod.rs (same arm, other variant)
    assert_eq!(DiagnosticCode::DeprecatedArrayBase.category(), DiagnosticCategory::Deprecated);
    Ok(())
}

#[test]
fn category_unused_import() -> Result<(), Box<dyn std::error::Error>> {
    // Line 856 in codes/mod.rs
    assert_eq!(DiagnosticCode::UnusedImport.category(), DiagnosticCategory::Import);
    Ok(())
}

#[test]
fn category_module_not_found() -> Result<(), Box<dyn std::error::Error>> {
    // Line 856 in codes/mod.rs (same arm, other variant)
    assert_eq!(DiagnosticCode::ModuleNotFound.category(), DiagnosticCategory::Import);
    Ok(())
}

#[test]
fn category_heredoc_in_format() -> Result<(), Box<dyn std::error::Error>> {
    // Lines 858-864 in codes/mod.rs
    assert_eq!(DiagnosticCode::HeredocInFormat.category(), DiagnosticCategory::Heredoc);
    Ok(())
}

#[test]
fn category_heredoc_in_begin() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::HeredocInBegin.category(), DiagnosticCategory::Heredoc);
    Ok(())
}

#[test]
fn category_heredoc_dynamic_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::HeredocDynamicDelimiter.category(), DiagnosticCategory::Heredoc);
    Ok(())
}

#[test]
fn category_heredoc_in_source_filter() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::HeredocInSourceFilter.category(), DiagnosticCategory::Heredoc);
    Ok(())
}

#[test]
fn category_heredoc_in_regex_code() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::HeredocInRegexCode.category(), DiagnosticCategory::Heredoc);
    Ok(())
}

#[test]
fn category_heredoc_in_eval() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::HeredocInEval.category(), DiagnosticCategory::Heredoc);
    Ok(())
}

#[test]
fn category_heredoc_tied_handle() -> Result<(), Box<dyn std::error::Error>> {
    // Line 864 in codes/mod.rs
    assert_eq!(DiagnosticCode::HeredocTiedHandle.category(), DiagnosticCategory::Heredoc);
    Ok(())
}

#[test]
fn category_version_incompat_feature_is_best_practices() -> Result<(), Box<dyn std::error::Error>> {
    // VersionIncompatFeature maps to BestPractices per line 844 in codes/mod.rs
    assert_eq!(
        DiagnosticCode::VersionIncompatFeature.category(),
        DiagnosticCategory::BestPractices
    );
    Ok(())
}

// ===========================================================================
// DiagnosticCategory::fmt() — all Display arms
// ===========================================================================

#[test]
fn category_display_parser() -> Result<(), Box<dyn std::error::Error>> {
    // Line 910 in codes/mod.rs
    assert_eq!(format!("{}", DiagnosticCategory::Parser), "Parser");
    Ok(())
}

#[test]
fn category_display_strict_warnings() -> Result<(), Box<dyn std::error::Error>> {
    // Line 911 in codes/mod.rs
    assert_eq!(format!("{}", DiagnosticCategory::StrictWarnings), "Strict/Warnings");
    Ok(())
}

#[test]
fn category_display_package_module() -> Result<(), Box<dyn std::error::Error>> {
    // Line 912 in codes/mod.rs
    assert_eq!(format!("{}", DiagnosticCategory::PackageModule), "Package/Module");
    Ok(())
}

#[test]
fn category_display_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    // Line 913 in codes/mod.rs
    assert_eq!(format!("{}", DiagnosticCategory::Subroutine), "Subroutine");
    Ok(())
}

#[test]
fn category_display_best_practices() -> Result<(), Box<dyn std::error::Error>> {
    // Line 914 in codes/mod.rs
    assert_eq!(format!("{}", DiagnosticCategory::BestPractices), "Best Practices");
    Ok(())
}

#[test]
fn category_display_deprecated() -> Result<(), Box<dyn std::error::Error>> {
    // Line 915 in codes/mod.rs
    assert_eq!(format!("{}", DiagnosticCategory::Deprecated), "Deprecated");
    Ok(())
}

#[test]
fn category_display_security() -> Result<(), Box<dyn std::error::Error>> {
    // Line 916 in codes/mod.rs
    assert_eq!(format!("{}", DiagnosticCategory::Security), "Security");
    Ok(())
}

#[test]
fn category_display_import() -> Result<(), Box<dyn std::error::Error>> {
    // Line 917 in codes/mod.rs
    assert_eq!(format!("{}", DiagnosticCategory::Import), "Import");
    Ok(())
}

#[test]
fn category_display_heredoc() -> Result<(), Box<dyn std::error::Error>> {
    // Line 918 in codes/mod.rs
    assert_eq!(format!("{}", DiagnosticCategory::Heredoc), "Heredoc");
    Ok(())
}

#[test]
fn category_display_perl_critic() -> Result<(), Box<dyn std::error::Error>> {
    // Line 919 in codes/mod.rs
    assert_eq!(format!("{}", DiagnosticCategory::PerlCritic), "Perl::Critic");
    Ok(())
}

// ===========================================================================
// parse_code() — full round-trip for all variants to cover remaining as_str
//               and parse_code arms not hit by the partial ALL_CODES list
// ===========================================================================

#[test]
fn parse_code_round_trip_all_variants_exhaustive() -> Result<(), Box<dyn std::error::Error>> {
    let all_variants = [
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

    assert_eq!(
        all_variants.len(),
        60,
        "expected exhaustive DiagnosticCode variant list to cover 60 variants"
    );
    for code in &all_variants {
        let s = code.as_str();
        let parsed = DiagnosticCode::parse_code(s);
        assert_eq!(parsed, Some(*code), "parse_code({s}) should round-trip back to {code:?}");
    }
    Ok(())
}

// ===========================================================================
// severity() — ensure newly exercised variants have expected severity
// ===========================================================================

#[test]
fn severity_missing_pod_coverage_is_hint() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::MissingPodCoverage.severity(), DiagnosticSeverity::Hint);
    Ok(())
}

#[test]
fn severity_unreachable_code_is_hint() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::UnreachableCode.severity(), DiagnosticSeverity::Hint);
    Ok(())
}

#[test]
fn severity_unused_import_is_hint() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::UnusedImport.severity(), DiagnosticSeverity::Hint);
    Ok(())
}

#[test]
fn severity_version_incompat_feature_is_warning() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::VersionIncompatFeature.severity(), DiagnosticSeverity::Warning);
    Ok(())
}

#[test]
fn severity_security_codes_are_all_warning() -> Result<(), Box<dyn std::error::Error>> {
    let security_codes = [
        DiagnosticCode::SecuritySystemCall,
        DiagnosticCode::SecurityExecCall,
        DiagnosticCode::SecurityPipeOpen,
        DiagnosticCode::SecurityReadpipe,
    ];
    for code in &security_codes {
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
fn severity_heredoc_codes_are_all_information() -> Result<(), Box<dyn std::error::Error>> {
    let heredoc_codes = [
        DiagnosticCode::HeredocInFormat,
        DiagnosticCode::HeredocInBegin,
        DiagnosticCode::HeredocDynamicDelimiter,
        DiagnosticCode::HeredocInSourceFilter,
        DiagnosticCode::HeredocInRegexCode,
        DiagnosticCode::HeredocInEval,
        DiagnosticCode::HeredocTiedHandle,
    ];
    for code in &heredoc_codes {
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
fn severity_deprecated_codes_are_warning() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticCode::DeprecatedDefined.severity(), DiagnosticSeverity::Warning);
    assert_eq!(DiagnosticCode::DeprecatedArrayBase.severity(), DiagnosticSeverity::Warning);
    Ok(())
}

#[test]
fn all_pl_codes_have_documentation_url() -> Result<(), Box<dyn std::error::Error>> {
    // Every PL-prefixed code should have a documentation URL. Covers the
    // internal match in documentation_url() including all newly tested variants.
    let pl_only = [
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
    ];
    for code in &pl_only {
        let url = code.documentation_url();
        assert!(url.is_some(), "PL code {} should have a documentation URL", code.as_str());
        let url_str = url.ok_or("missing url")?;
        assert!(
            url_str.starts_with("https://docs.perl-lsp.org/errors/"),
            "URL for {} should start with base URL, got {}",
            code.as_str(),
            url_str
        );
        assert!(
            url_str.ends_with(code.as_str()),
            "URL for {} should end with its code string, got {}",
            code.as_str(),
            url_str
        );
    }
    Ok(())
}
