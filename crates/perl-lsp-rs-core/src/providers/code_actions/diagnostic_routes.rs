//! Diagnostic-code routing for code-action quick fixes.

use super::quick_fixes;
use super::types::{CodeAction, QuickFixDiagnostic};
use crate::providers::diagnostics::Diagnostic;
use perl_diagnostics::codes::DiagnosticCode;

/// Convert Diagnostic to QuickFixDiagnostic.
///
/// Since Diagnostic already uses byte offsets, this is a simple copy.
fn to_quick_fix_diagnostic(diag: &Diagnostic) -> QuickFixDiagnostic {
    QuickFixDiagnostic { range: diag.range, message: diag.message.clone(), code: diag.code.clone() }
}

pub(super) fn quick_fixes_for_diagnostics(
    source: &str,
    diagnostics: &[Diagnostic],
) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    for diagnostic in diagnostics {
        actions.extend(quick_fixes_for_diagnostic(source, diagnostic));
    }

    actions
}

fn quick_fixes_for_diagnostic(source: &str, diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();
    let qf_diag = to_quick_fix_diagnostic(diagnostic);

    let Some(code) = &diagnostic.code else {
        return actions;
    };

    let policy_code = code.strip_prefix("Perl::Critic::Policy::").unwrap_or(code.as_str());

    match policy_code {
        // PL103: Undefined/undeclared variable
        c if c == DiagnosticCode::UndefinedVariable.as_str() => {
            actions.extend(quick_fixes::fix_undefined_variable(source, &qf_diag));
        }
        // PL102: Unused variable
        c if c == DiagnosticCode::UnusedVariable.as_str() => {
            actions.extend(quick_fixes::fix_unused_variable(source, &qf_diag));
        }
        "native.variables.unused_lexical" => {
            actions.extend(quick_fixes::fix_unused_variable(source, &qf_diag));
        }
        // PL403: Assignment in condition
        c if c == DiagnosticCode::AssignmentInCondition.as_str()
            || c == "native.common.assignment_in_condition" =>
        {
            actions.extend(quick_fixes::fix_assignment_in_condition(source, &qf_diag));
        }
        // PL100: Missing use strict
        c if c == DiagnosticCode::MissingStrict.as_str() => {
            actions.extend(quick_fixes::add_use_strict());
        }
        // PL101: Missing use warnings
        c if c == DiagnosticCode::MissingWarnings.as_str() => {
            actions.extend(quick_fixes::add_use_warnings());
        }
        // PL502: Phase-scoped use strict misconception
        c if c == DiagnosticCode::PhaseScopedStrictPragma.as_str() => {
            actions.extend(quick_fixes::move_use_strict_to_file_scope(source, &qf_diag));
        }
        // PL503: Phase-scoped use warnings misconception
        c if c == DiagnosticCode::PhaseScopedWarningsPragma.as_str() => {
            actions.extend(quick_fixes::move_use_warnings_to_file_scope(source, &qf_diag));
        }
        // PL500: Deprecated defined()
        c if c == DiagnosticCode::DeprecatedDefined.as_str()
            || c == "native.common.deprecated_defined" =>
        {
            actions.extend(quick_fixes::fix_deprecated_defined(source, &qf_diag));
        }
        // PL404: Numeric comparison with undef
        "native.common.undef_comparison" => {
            actions.extend(quick_fixes::fix_native_undef_comparison(source, &qf_diag));
        }
        c if c == DiagnosticCode::NumericComparisonWithUndef.as_str() => {
            actions.extend(quick_fixes::fix_numeric_undef(source, &qf_diag));
        }
        // PL109: Unquoted bareword
        c if c == DiagnosticCode::UnquotedBareword.as_str() => {
            actions.extend(quick_fixes::fix_bareword(source, &qf_diag));
            // Also offer an import action when the bareword resolves to a known module.
            actions.extend(quick_fixes::fix_import_for_bareword_function(source, &qf_diag));
        }
        // PL001: General parse error (stable code)
        // PL002: Syntax error — same quick-fix routing as PL001
        c if c == DiagnosticCode::ParseError.as_str()
            || c == DiagnosticCode::SyntaxError.as_str() =>
        {
            actions.extend(quick_fixes::fix_parse_error(source, &qf_diag, c));
        }
        // parse-error-* subcodes (legacy subtype codes from error classifier)
        c if c.starts_with("parse-error-") => {
            actions.extend(quick_fixes::fix_parse_error(source, &qf_diag, c));
        }
        // PL108: Unused parameter
        c if c == DiagnosticCode::UnusedParameter.as_str()
            || c == "native.variables.unused_parameter" =>
        {
            actions.extend(quick_fixes::fix_unused_parameter(&qf_diag));
        }
        // PL107: Duplicate parameter
        c if c == DiagnosticCode::DuplicateParameter.as_str()
            || c == "native.variables.duplicate_parameter" =>
        {
            actions.extend(quick_fixes::fix_duplicate_parameter(&qf_diag));
        }
        // PL110: Parameter shadows outer/global variable
        c if c == DiagnosticCode::ParameterShadowsGlobal.as_str()
            || c == "native.variables.parameter_shadows_global" =>
        {
            actions.extend(quick_fixes::fix_parameter_shadowing(&qf_diag));
        }
        // PL104: Variable shadowing
        c if c == DiagnosticCode::VariableShadowing.as_str()
            || c == "native.variables.shadowed_lexical" =>
        {
            actions.extend(quick_fixes::fix_variable_shadowing(&qf_diag));
        }
        // PL400: Bareword filehandle
        c if c == DiagnosticCode::BarewordFilehandle.as_str()
            || c == "native.io.bareword_filehandle" =>
        {
            actions.extend(quick_fixes::fix_bareword_filehandle(&qf_diag));
        }
        // Perl::Critic policy alias for bareword filehandle.
        "InputOutput::ProhibitBarewordFileHandles" => {
            actions.extend(quick_fixes::fix_bareword_filehandle(&qf_diag));
        }
        // PL401: Two-arg open
        c if c == DiagnosticCode::TwoArgOpen.as_str() || c == "native.io.two_arg_open" => {
            actions.extend(quick_fixes::fix_two_arg_open(source, &qf_diag));
        }
        // Perl::Critic policy aliases for two-arg open.
        "InputOutput::ProhibitTwoArgOpen"
        | "InputOutput::RequireBriefOpen"
        | "InputOutput::RequireThreeArgOpen" => {
            actions.extend(quick_fixes::fix_two_arg_open(source, &qf_diag));
        }
        // Perl::Critic/native critic policies for missing strict/warnings.
        "TestingAndDebugging::RequireUseStrict" | "native.testing.require_use_strict" => {
            actions.extend(quick_fixes::add_use_strict());
        }
        "TestingAndDebugging::RequireUseWarnings" | "native.testing.require_use_warnings" => {
            actions.extend(quick_fixes::add_use_warnings());
        }
        // Perl::Critic policy alias for unused variables.
        "Variables::ProhibitUnusedVariables" => {
            actions.extend(quick_fixes::fix_unused_variable(source, &qf_diag));
        }
        // PL200: Missing package declaration
        c if c == DiagnosticCode::MissingPackageDeclaration.as_str() => {
            actions.extend(quick_fixes::fix_missing_package_declaration(source));
        }
        // PL105: Variable redeclaration (duplicate my)
        c if c == DiagnosticCode::VariableRedeclaration.as_str()
            || c == "native.variables.duplicate_lexical" =>
        {
            actions.extend(quick_fixes::fix_variable_redeclaration(source, &qf_diag));
        }
        // PL111: Misspelled pragma
        c if c == DiagnosticCode::MisspelledPragma.as_str() => {
            actions.extend(quick_fixes::fix_misspelled_pragma(source, &qf_diag));
        }
        // PL406: Unreachable code
        c if c == DiagnosticCode::UnreachableCode.as_str()
            || c == "native.common.unreachable_code" =>
        {
            actions.extend(quick_fixes::fix_unreachable_code(source, &qf_diag));
        }
        // PL300: Duplicate subroutine
        c if c == DiagnosticCode::DuplicateSubroutine.as_str() => {
            actions.extend(quick_fixes::fix_duplicate_subroutine(&qf_diag));
        }
        // PL301: Missing return statement
        c if c == DiagnosticCode::MissingReturn.as_str() => {
            actions.extend(quick_fixes::fix_missing_return(source, &qf_diag));
        }
        // PL408: Duplicate hash key
        c if c == DiagnosticCode::DuplicateHashKey.as_str() => {
            actions.extend(quick_fixes::fix_duplicate_hash_keys(source, &qf_diag));
        }
        // PL700: Unused import
        c if c == DiagnosticCode::UnusedImport.as_str() => {
            actions.extend(quick_fixes::fix_unused_import(source, &qf_diag));
        }
        // PL501: Deprecated $[ array base variable
        c if c == DiagnosticCode::DeprecatedArrayBase.as_str() => {
            actions.extend(quick_fixes::fix_deprecated_array_base(source, &qf_diag));
        }
        // PL602: Global signal handler assignment
        c if c == DiagnosticCode::SecuritySignalHandler.as_str() => {
            actions.extend(quick_fixes::fix_security_signal_handler(source, &qf_diag));
        }
        // PL405: printf/sprintf format specifier count mismatch
        c if c == DiagnosticCode::PrintfFormatMismatch.as_str()
            || c == "native.common.printf_format_arity" =>
        {
            actions.extend(quick_fixes::fix_printf_format_arity(source, &qf_diag));
        }
        _ => {}
    }

    actions
}
