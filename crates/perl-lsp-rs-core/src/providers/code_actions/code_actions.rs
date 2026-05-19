//! Code actions and quick fixes for Perl
//!
//! This module provides automated fixes for common issues and refactoring actions.
//!
//! # LSP Workflow Integration
//!
//! Code actions integrate with the Parse → Index → Navigate → Complete → Analyze workflow:
//!
//! - **Parse**: AST analysis identifies code patterns requiring fixes or refactoring
//! - **Index**: Symbol tables provide context for variable and function renaming actions
//! - **Navigate**: Cross-file analysis enables workspace-wide refactoring operations
//! - **Complete**: Code action suggestions are refined based on completion context
//! - **Analyze**: Diagnostic analysis drives automated fix generation and prioritization
//!
//! This integration ensures code actions are contextually appropriate and maintain
//! code correctness across the entire Perl workspace.
//!
//! # LSP Client Capabilities
//!
//! Requires client support for `textDocument/codeAction` capabilities and
//! `workspace/workspaceEdit` to apply edits across files.
//!
//! # Protocol Compliance
//!
//! Implements LSP code action protocol semantics (LSP 3.17+) including
//! range-based requests, diagnostic filtering, and edit application rules.
//!
//! # Performance Characteristics
//!
//! - **Action generation**: <50ms for typical code action requests
//! - **Edit application**: <100ms for complex workspace refactoring
//! - **Memory usage**: <5MB for action metadata and edit operations
//! - **Incremental analysis**: Leverages ≤1ms parsing SLO for real-time suggestions
//!
//! # Related Modules
//!
//! This module integrates with diagnostics and import optimization modules
//! for import-related code actions.
//!
//! # See also
//!
//! - [`DiagnosticsProvider`](crate::ide::lsp_compat::diagnostics::DiagnosticsProvider)
//! - [`crate::ide::lsp_compat::references`]
//!
//! # Usage Examples
//!
//! ```ignore
//! use perl_lsp_providers::ide::lsp_compat::code_actions::{CodeActionsProvider, CodeActionKind};
//! use perl_lsp_providers::ide::lsp_compat::diagnostics::Diagnostic;
//! use perl_parser_core::Parser;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let code = "my $unused_var = 42;";
//! let provider = CodeActionsProvider::new(code.to_string());
//! let mut parser = Parser::new(code);
//! let ast = parser.parse()?;
//! let diagnostics = vec![]; // Would contain actual diagnostics
//!
//! // Generate code actions for diagnostics
//! let actions = provider.get_code_actions(&ast, (0, code.len()), &diagnostics);
//! for action in actions {
//!     println!("Available action: {} ({:?})", action.title, action.kind);
//! }
//! # Ok(())
//! # }
//! ```

use super::modernize;
use super::quick_fixes;
use super::refactors;
use super::types::QuickFixDiagnostic;

pub use super::types::{CodeAction, CodeActionKind};

use crate::providers::diagnostics::Diagnostic;
use perl_diagnostics::codes::DiagnosticCode;
use perl_parser_core::Node;

/// Convert Diagnostic to QuickFixDiagnostic
///
/// Since Diagnostic already uses byte offsets, this is a simple copy.
fn to_quick_fix_diagnostic(diag: &Diagnostic) -> QuickFixDiagnostic {
    QuickFixDiagnostic { range: diag.range, message: diag.message.clone(), code: diag.code.clone() }
}

/// Code actions provider
///
/// Analyzes Perl source code and provides automated fixes and refactoring
/// actions for common issues and improvement opportunities.
pub struct CodeActionsProvider {
    source: String,
}

impl CodeActionsProvider {
    /// Create a new code actions provider
    pub fn new(source: String) -> Self {
        Self { source }
    }

    /// Get code actions for a range
    pub fn get_code_actions(
        &self,
        ast: &Node,
        range: (usize, usize),
        diagnostics: &[Diagnostic],
    ) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Get quick fixes for diagnostics
        for diagnostic in diagnostics {
            let qf_diag = to_quick_fix_diagnostic(diagnostic);
            if let Some(code) = &diagnostic.code {
                let policy_code =
                    code.strip_prefix("Perl::Critic::Policy::").unwrap_or(code.as_str());

                match policy_code {
                    // PL103: Undefined/undeclared variable
                    c if c == DiagnosticCode::UndefinedVariable.as_str() => {
                        actions.extend(quick_fixes::fix_undefined_variable(&self.source, &qf_diag));
                    }
                    // PL102: Unused variable
                    c if c == DiagnosticCode::UnusedVariable.as_str() => {
                        actions.extend(quick_fixes::fix_unused_variable(&self.source, &qf_diag));
                    }
                    "native.variables.unused_lexical" => {
                        actions.extend(quick_fixes::fix_unused_variable(&self.source, &qf_diag));
                    }
                    // PL403: Assignment in condition
                    c if c == DiagnosticCode::AssignmentInCondition.as_str()
                        || c == "native.common.assignment_in_condition" =>
                    {
                        actions.extend(quick_fixes::fix_assignment_in_condition(
                            &self.source,
                            &qf_diag,
                        ));
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
                        actions.extend(quick_fixes::move_use_strict_to_file_scope(
                            &self.source,
                            &qf_diag,
                        ));
                    }
                    // PL503: Phase-scoped use warnings misconception
                    c if c == DiagnosticCode::PhaseScopedWarningsPragma.as_str() => {
                        actions.extend(quick_fixes::move_use_warnings_to_file_scope(
                            &self.source,
                            &qf_diag,
                        ));
                    }
                    // PL500: Deprecated defined()
                    c if c == DiagnosticCode::DeprecatedDefined.as_str()
                        || c == "native.common.deprecated_defined" =>
                    {
                        actions.extend(quick_fixes::fix_deprecated_defined(&self.source, &qf_diag));
                    }
                    // PL404: Numeric comparison with undef
                    "native.common.undef_comparison" => {
                        actions.extend(quick_fixes::fix_native_undef_comparison(
                            &self.source,
                            &qf_diag,
                        ));
                    }
                    c if c == DiagnosticCode::NumericComparisonWithUndef.as_str() => {
                        actions.extend(quick_fixes::fix_numeric_undef(&self.source, &qf_diag));
                    }
                    // PL109: Unquoted bareword
                    c if c == DiagnosticCode::UnquotedBareword.as_str() => {
                        actions.extend(quick_fixes::fix_bareword(&self.source, &qf_diag));
                    }
                    // PL001: General parse error (stable code)
                    // PL002: Syntax error — same quick-fix routing as PL001
                    c if c == DiagnosticCode::ParseError.as_str()
                        || c == DiagnosticCode::SyntaxError.as_str() =>
                    {
                        actions.extend(quick_fixes::fix_parse_error(&self.source, &qf_diag, c));
                    }
                    // parse-error-* subcodes (legacy subtype codes from error classifier)
                    c if c.starts_with("parse-error-") => {
                        actions.extend(quick_fixes::fix_parse_error(&self.source, &qf_diag, c));
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
                    c if c == DiagnosticCode::TwoArgOpen.as_str()
                        || c == "native.io.two_arg_open" =>
                    {
                        actions.extend(quick_fixes::fix_two_arg_open(&self.source, &qf_diag));
                    }
                    // Perl::Critic policy aliases for two-arg open.
                    "InputOutput::ProhibitTwoArgOpen"
                    | "InputOutput::RequireBriefOpen"
                    | "InputOutput::RequireThreeArgOpen" => {
                        actions.extend(quick_fixes::fix_two_arg_open(&self.source, &qf_diag));
                    }
                    // Perl::Critic/native critic policies for missing strict/warnings.
                    "TestingAndDebugging::RequireUseStrict"
                    | "native.testing.require_use_strict" => {
                        actions.extend(quick_fixes::add_use_strict());
                    }
                    "TestingAndDebugging::RequireUseWarnings"
                    | "native.testing.require_use_warnings" => {
                        actions.extend(quick_fixes::add_use_warnings());
                    }
                    // Perl::Critic policy alias for unused variables.
                    "Variables::ProhibitUnusedVariables" => {
                        actions.extend(quick_fixes::fix_unused_variable(&self.source, &qf_diag));
                    }
                    // PL200: Missing package declaration
                    c if c == DiagnosticCode::MissingPackageDeclaration.as_str() => {
                        actions.extend(quick_fixes::fix_missing_package_declaration(&self.source));
                    }
                    // PL105: Variable redeclaration (duplicate my)
                    c if c == DiagnosticCode::VariableRedeclaration.as_str()
                        || c == "native.variables.duplicate_lexical" =>
                    {
                        actions.extend(quick_fixes::fix_variable_redeclaration(
                            &self.source,
                            &qf_diag,
                        ));
                    }
                    // PL111: Misspelled pragma
                    c if c == DiagnosticCode::MisspelledPragma.as_str() => {
                        actions.extend(quick_fixes::fix_misspelled_pragma(&self.source, &qf_diag));
                    }
                    // PL406: Unreachable code
                    c if c == DiagnosticCode::UnreachableCode.as_str()
                        || c == "native.common.unreachable_code" =>
                    {
                        actions.extend(quick_fixes::fix_unreachable_code(&self.source, &qf_diag));
                    }
                    // PL300: Duplicate subroutine
                    c if c == DiagnosticCode::DuplicateSubroutine.as_str() => {
                        actions.extend(quick_fixes::fix_duplicate_subroutine(&qf_diag));
                    }
                    // PL301: Missing return statement
                    c if c == DiagnosticCode::MissingReturn.as_str() => {
                        actions.extend(quick_fixes::fix_missing_return(&self.source, &qf_diag));
                    }
                    // PL408: Duplicate hash key
                    c if c == DiagnosticCode::DuplicateHashKey.as_str() => {
                        actions
                            .extend(quick_fixes::fix_duplicate_hash_keys(&self.source, &qf_diag));
                    }
                    _ => {}
                }
            }
        }

        // Source-level lints (not diagnostic-driven)
        // Only suggest shebang fix when the range includes the first line
        if range.0 == 0 || !self.source[..range.0].contains('\n') {
            actions.extend(quick_fixes::fix_hardcoded_shebang(&self.source));
        }

        // Get refactoring actions for selection
        actions.extend(refactors::get_refactoring_actions(&self.source, ast, range));

        // Get modernization suggestions
        actions.extend(modernize::get_modernize_actions(&self.source));

        actions
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::providers::diagnostics::DiagnosticSeverity;
    use perl_parser_core::Parser;
    use perl_tdd_support::{must, must_some};

    /// Create a diagnostic with byte offsets
    fn make_diagnostic(start: usize, end: usize, code: &str, msg: &str) -> Diagnostic {
        Diagnostic {
            range: (start, end),
            severity: DiagnosticSeverity::Error,
            code: Some(code.to_string()),
            message: msg.to_string(),
            related_information: Vec::new(),
            tags: Vec::new(),
            suggestion: None,
        }
    }

    fn apply_action(source: &str, action: &CodeAction) -> String {
        let mut edits = action.edit.changes.clone();
        edits.sort_by(|a, b| b.location.start.cmp(&a.location.start));

        let mut output = source.to_string();
        for edit in edits {
            output.replace_range(edit.location.start..edit.location.end, &edit.new_text);
        }
        output
    }

    #[test]
    fn test_undefined_variable_fix() {
        let source = "use strict;\nprint $undefined;";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        // Create a synthetic diagnostic for undefined-variable (stable code PL103)
        // "$undefined" starts at byte offset 18 (after "use strict;\nprint ")
        let diagnostics = vec![make_diagnostic(
            18, // start of "$undefined"
            28, // end of "$undefined"
            "PL103",
            "Undefined variable '$undefined'",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title.contains("Declare") || a.title.contains("my")),
            "Expected action to declare variable, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_assignment_in_condition_fix() {
        let source = "if ($x = 5) { }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        // Create a synthetic diagnostic for assignment-in-condition (stable code PL403)
        // "$x = 5" is at bytes 4-10
        let diagnostics = vec![make_diagnostic(
            4,  // start of "$x = 5"
            10, // end of "$x = 5"
            "PL403",
            "Assignment in condition",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title.contains("==")),
            "Expected action to change to comparison, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_native_critic_policy_alias_for_assignment_in_condition() {
        let source = "if ($x = 5) { }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![make_diagnostic(
            4,
            10,
            "native.common.assignment_in_condition",
            "Assignment in condition - did you mean '=='?",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title == "Change to comparison (==)"
                && a.edit.changes.iter().any(|edit| edit.new_text == "==")),
            "Expected native critic alias to offer comparison fix, got: {:?}",
            actions
        );
        assert!(
            actions.iter().any(|a| a.title == "Keep assignment (add parentheses)"),
            "Expected native critic alias to offer intentional-assignment fix, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_native_unreachable_code_alias_produces_quick_fix() {
        let source = "sub f {\nreturn 1;\nmy $dead = 2;\n}\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = source.find("my $dead").expect("dead statement start");
        let end = start + "my $dead = 2;".len();
        let diagnostics = vec![make_diagnostic(
            start,
            end,
            "native.common.unreachable_code",
            "Unreachable code: this statement cannot be executed",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|action| action.title == "Remove unreachable code"
                && action.edit.changes.iter().any(|edit| edit.new_text.is_empty()
                    && &source[edit.location.start..edit.location.end] == "my $dead = 2;\n")),
            "Expected native unreachable-code alias to remove dead line, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_native_deprecated_defined_alias_produces_quick_fix() {
        let source = "if (defined @items) { print @items; }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![make_diagnostic(
            4,
            18,
            "native.common.deprecated_defined",
            "Use of 'defined @items' is deprecated",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title == "Replace with '@items'"
                && a.edit.changes.iter().any(|edit| edit.new_text == "@items")),
            "Expected native deprecated-defined alias to offer defined() removal, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_native_deprecated_defined_alias_normalizes_parenthesized_quick_fix() {
        let source = "if (defined(%seen)) { print keys %seen; }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![make_diagnostic(
            4,
            18,
            "native.common.deprecated_defined",
            "Use of 'defined %seen' is deprecated",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title == "Replace with '%seen'"
                && a.edit.changes.iter().any(|edit| edit.new_text == "%seen")),
            "Expected native deprecated-defined alias to normalize parenthesized defined() removal, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_native_undef_comparison_alias_produces_defined_quick_fix() {
        let source = "if ($value == undef) { print $value; }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![make_diagnostic(
            4,
            19,
            "native.common.undef_comparison",
            "Using '==' with undef -- use defined() to check first",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            actions.iter().any(|a| a.title == "Use defined() check"
                && a.edit.changes.iter().any(|edit| edit.new_text == "!defined($value)")),
            "Expected native undef-comparison alias to offer defined() fix, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_hardcoded_shebang_suggests_portable() {
        let source = "#!/usr/bin/perl\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert_eq!(shebang_actions.len(), 1, "Expected one shebang action");
        assert_eq!(shebang_actions[0].edit.changes[0].new_text, "#!/usr/bin/env perl");
    }

    #[test]
    fn test_hardcoded_shebang_preserves_flags() {
        let source = "#!/usr/bin/perl -w\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert_eq!(shebang_actions.len(), 1);
        assert_eq!(shebang_actions[0].edit.changes[0].new_text, "#!/usr/bin/env perl -w");
    }

    #[test]
    fn test_env_perl_shebang_not_flagged() {
        let source = "#!/usr/bin/env perl\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert!(shebang_actions.is_empty(), "env perl should not be flagged");
    }

    #[test]
    fn test_no_shebang_not_flagged() {
        let source = "use strict;\nuse warnings;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert!(shebang_actions.is_empty(), "No shebang should not be flagged");
    }

    #[test]
    fn test_local_bin_perl_shebang() {
        let source = "#!/usr/local/bin/perl\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert_eq!(shebang_actions.len(), 1, "Local bin perl should be flagged");
        assert_eq!(shebang_actions[0].edit.changes[0].new_text, "#!/usr/bin/env perl");
    }

    #[test]
    fn test_shebang_with_taint_flag() {
        let source = "#!/usr/bin/perl -T\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert_eq!(shebang_actions.len(), 1);
        assert_eq!(shebang_actions[0].edit.changes[0].new_text, "#!/usr/bin/env perl -T");
    }

    #[test]
    fn test_bash_shebang_not_flagged() {
        let source = "#!/bin/bash\necho hello\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();

        assert!(shebang_actions.is_empty(), "Non-perl shebang should not be flagged");
    }

    #[test]
    fn test_shebang_fix_not_suggested_when_range_starts_after_first_line() {
        let source = "#!/usr/bin/perl\nmy $x = 1;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];
        let range_start = source.find("my $x").expect("line exists");

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (range_start, source.len()), &diagnostics);

        let shebang_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("portable shebang")).collect();
        assert!(
            shebang_actions.is_empty(),
            "Shebang fix should only appear when requested range includes line 1"
        );
    }

    #[test]
    fn test_perlcritic_policy_aliases_produce_quick_fixes() {
        let source = "open FH, $path;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![
            Diagnostic {
                range: (0, 4),
                severity: DiagnosticSeverity::Warning,
                code: Some("InputOutput::ProhibitBarewordFileHandles".to_string()),
                message: "Bareword filehandle 'FH'".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (0, 4),
                severity: DiagnosticSeverity::Warning,
                code: Some("InputOutput::RequireThreeArgOpen".to_string()),
                message: "Use 3-arg open".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
        ];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
        assert!(actions.iter().any(|a| a.title.contains("bareword filehandle")));
        assert!(actions.iter().any(|a| a.title.contains("three-argument open() for safety")));
    }

    #[test]
    fn test_fully_qualified_perlcritic_policy_aliases_produce_quick_fixes() {
        let source = "open FH, $path;
";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![
            Diagnostic {
                range: (0, 4),
                severity: DiagnosticSeverity::Warning,
                code: Some(
                    "Perl::Critic::Policy::InputOutput::ProhibitBarewordFileHandles".to_string(),
                ),
                message: "Bareword filehandle 'FH'".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (0, 4),
                severity: DiagnosticSeverity::Warning,
                code: Some("Perl::Critic::Policy::InputOutput::RequireThreeArgOpen".to_string()),
                message: "Use 3-arg open".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
        ];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
        assert!(actions.iter().any(|a| a.title.contains("bareword filehandle")));
        assert!(actions.iter().any(|a| a.title.contains("three-argument open() for safety")));
    }

    #[test]
    fn test_perlcritic_require_brief_open_alias_produces_quick_fix() {
        let source = "open FH, $path;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![Diagnostic {
            range: (0, 4),
            severity: DiagnosticSeverity::Warning,
            code: Some("InputOutput::RequireBriefOpen".to_string()),
            message: "Use 3-arg open".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|a| a.title.contains("three-argument open() for safety")));
    }

    #[test]
    fn test_native_bareword_filehandle_alias_produces_quick_fix() {
        let source = "open FH, $path;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![Diagnostic {
            range: (5, 7),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.io.bareword_filehandle".to_string()),
            message: "Bareword filehandle 'FH' should be lexical".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let fix = actions
            .iter()
            .find(|action| action.title.contains("bareword filehandle"))
            .expect("native bareword filehandle diagnostic should produce a quick fix");
        assert_eq!(fix.edit.changes[0].new_text, "my $fh_fh");
    }

    #[test]
    fn test_native_two_arg_open_alias_produces_quick_fix() {
        let source = "open(my $fh, $path);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![Diagnostic {
            range: (0, 19),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.io.two_arg_open".to_string()),
            message: "Two-argument open should use an explicit mode".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let fix = actions
            .iter()
            .find(|action| action.title.contains("three-argument open()"))
            .expect("native two-arg open diagnostic should produce a quick fix");
        assert_eq!(fix.edit.changes[0].new_text, "open(my $fh, '<', $path)");
    }

    #[test]
    fn test_legacy_two_arg_open_alias_range_only_open_edits_whole_call() {
        let source = "open FH, $path;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![Diagnostic {
            range: (0, 4),
            severity: DiagnosticSeverity::Warning,
            code: Some("InputOutput::RequireThreeArgOpen".to_string()),
            message: "Two-argument open should use an explicit mode".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let fix =
            must_some(actions.iter().find(|action| action.title.contains("three-argument open()")));
        assert_eq!(fix.edit.changes[0].location.start, 0);
        assert_eq!(fix.edit.changes[0].location.end, "open FH, $path".len());
        assert_eq!(fix.edit.changes[0].new_text, "open(FH, '<', $path)");
    }

    #[test]
    fn test_legacy_two_arg_open_alias_range_only_open_rejects_ambiguous_line_fallback() {
        for source in ["open FH, $path; # legacy\n", "open FH, $path; close FH;\n"] {
            let mut parser = Parser::new(source);
            let ast = must(parser.parse());
            let diagnostics = vec![Diagnostic {
                range: (0, 4),
                severity: DiagnosticSeverity::Warning,
                code: Some("InputOutput::RequireThreeArgOpen".to_string()),
                message: "Two-argument open should use an explicit mode".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            }];

            let provider = CodeActionsProvider::new(source.to_string());
            let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

            assert!(
                actions.iter().all(|action| !action.title.contains("three-argument open()")),
                "ambiguous fallback should not produce a two-arg open fix for {source:?}"
            );
        }
    }

    #[test]
    fn test_perlcritic_policy_aliases_for_strict_warnings_and_unused_variable() {
        let source = "print $unused;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![
            Diagnostic {
                range: (0, source.len()),
                severity: DiagnosticSeverity::Warning,
                code: Some("TestingAndDebugging::RequireUseStrict".to_string()),
                message: "Code does not use strict".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (0, source.len()),
                severity: DiagnosticSeverity::Warning,
                code: Some("TestingAndDebugging::RequireUseWarnings".to_string()),
                message: "Code does not use warnings".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (6, 13),
                severity: DiagnosticSeverity::Warning,
                code: Some("Variables::ProhibitUnusedVariables".to_string()),
                message: "Unused variable '$unused'".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
        ];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|a| a.title == "Add 'use strict'"));
        assert!(actions.iter().any(|a| a.title == "Add 'use warnings'"));
        assert!(actions.iter().any(|a| a.title.contains("Remove unused variable")));
    }

    #[test]
    fn test_native_critic_policy_aliases_for_strict_and_warnings() {
        let source = "print 'hello';\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![
            Diagnostic {
                range: (0, 0),
                severity: DiagnosticSeverity::Warning,
                code: Some("native.testing.require_use_strict".to_string()),
                message: "Code does not use strict".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (0, 0),
                severity: DiagnosticSeverity::Warning,
                code: Some("native.testing.require_use_warnings".to_string()),
                message: "Code does not use warnings".to_string(),
                suggestion: None,
                related_information: Vec::new(),
                tags: Vec::new(),
            },
        ];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|a| a.title == "Add 'use strict'"));
        assert!(actions.iter().any(|a| a.title == "Add 'use warnings'"));
    }

    #[test]
    fn test_native_critic_policy_alias_for_unused_lexical() {
        let source = "use strict;\nuse warnings;\nmy $unused = 1;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = source.find("$unused").unwrap();
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$unused".len()),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.variables.unused_lexical".to_string()),
            message: "Lexical variable '$unused' is declared but never used".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|a| a.title == "Remove unused variable"));
        assert!(actions.iter().any(|a| {
            a.title == "Rename to '$_unused'"
                && a.edit.changes.iter().any(|edit| edit.new_text == "$_unused")
        }));
    }

    #[test]
    fn test_native_critic_policy_alias_for_unused_parameter() {
        let source = "use strict;\nuse warnings;\nsub helper($used, $unused) { return $used; }\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = source.find("$unused").unwrap();
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$unused".len()),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.variables.unused_parameter".to_string()),
            message: "Parameter '$unused' is never used".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|action| {
            action.title == "Rename to '_$unused'"
                && action.edit.changes.iter().any(|edit| {
                    edit.location.start == start
                        && edit.location.end == start + "$unused".len()
                        && edit.new_text == "_$unused"
                })
        }));
    }

    #[test]
    fn test_native_critic_policy_alias_for_duplicate_parameter() {
        let source = "use strict;\nuse warnings;\nsub helper($arg, $arg) { return $arg; }\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = source.find(", $arg").unwrap() + ", ".len();
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$arg".len()),
            severity: DiagnosticSeverity::Error,
            code: Some("native.variables.duplicate_parameter".to_string()),
            message: "Parameter '$arg' appears more than once in this signature".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|action| {
            action.title == "Remove duplicate parameter '$arg'"
                && action.edit.changes.iter().any(|edit| {
                    edit.location.start == start
                        && edit.location.end == start + "$arg".len()
                        && edit.new_text.is_empty()
                })
        }));
        assert!(actions.iter().any(|action| {
            action.title == "Rename duplicate to '$arg_2'"
                && action.edit.changes.iter().any(|edit| edit.new_text == "$arg_2")
        }));
    }

    #[test]
    fn test_native_critic_policy_alias_for_parameter_shadows_global() {
        let source = "use strict;\nuse warnings;\nmy $name = 'outer';\nsub helper($name) { return $name; }\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = source.find("($name").unwrap() + 1;
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$name".len()),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.variables.parameter_shadows_global".to_string()),
            message: "Parameter '$name' shadows an outer declaration".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|action| {
            action.title == "Rename parameter to '$p_name'"
                && action.edit.changes.iter().any(|edit| {
                    edit.location.start == start
                        && edit.location.end == start + "$name".len()
                        && edit.new_text == "$p_name"
                })
        }));
        assert!(actions.iter().any(|action| {
            action.title == "Rename parameter to '$name_param'"
                && action.edit.changes.iter().any(|edit| edit.new_text == "$name_param")
        }));
    }

    #[test]
    fn test_native_critic_policy_alias_for_duplicate_lexical() {
        let source = "use strict;\nuse warnings;\nmy $dup = 1;\nmy $dup = 2;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = source.rfind("$dup").unwrap();
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$dup".len()),
            severity: DiagnosticSeverity::Error,
            code: Some("native.variables.duplicate_lexical".to_string()),
            message: "Lexical variable '$dup' is declared more than once in the same scope"
                .to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let action = actions
            .iter()
            .find(|action| action.title == "Remove duplicate 'my' declaration")
            .expect("native duplicate lexical should reuse duplicate-my fix");
        assert!(action.edit.changes.iter().any(|edit| {
            edit.location.start == source.rfind("my $dup").unwrap()
                && edit.location.end == start
                && edit.new_text.is_empty()
        }));
    }

    #[test]
    fn test_native_critic_policy_alias_for_shadowed_lexical() {
        let source = "use strict;\nuse warnings;\nmy $value = 1;\n{ my $value = 2; }\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = source.rfind("$value").unwrap();
        let diagnostics = vec![Diagnostic {
            range: (start, start + "$value".len()),
            severity: DiagnosticSeverity::Warning,
            code: Some("native.variables.shadowed_lexical".to_string()),
            message: "Lexical variable '$value' shadows an outer declaration".to_string(),
            suggestion: None,
            related_information: Vec::new(),
            tags: Vec::new(),
        }];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|action| {
            action.title == "Rename to '$value_inner'"
                && action.edit.changes.iter().any(|edit| edit.new_text == "$value_inner")
        }));
        assert!(actions.iter().any(|action| {
            action.title == "Rename to '$value_local'"
                && action.edit.changes.iter().any(|edit| edit.new_text == "$value_local")
        }));
    }

    #[test]
    fn test_phase_scoped_strict_quick_fix_moves_pragma_to_file_scope() {
        let source = "BEGIN { use strict; }\nmy $x = 1;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = source.find("use strict;").unwrap();
        let end = start + "use strict;".len();
        let diagnostics = vec![make_diagnostic(
            start,
            end,
            "PL502",
            "`use strict` inside a BEGIN block does not enable strict for the rest of the file",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
        let action = actions
            .iter()
            .find(|action| action.title == "Move 'use strict' to file scope")
            .expect("phase-scoped strict quick fix");

        let rewritten = apply_action(source, action);
        assert!(rewritten.starts_with("use strict;\nBEGIN { "));
        assert!(rewritten.contains("BEGIN {  }"));
    }

    #[test]
    fn test_phase_scoped_warnings_quick_fix_preserves_shebang() {
        let source = "#!/usr/bin/perl\nBEGIN { use warnings; }\nprint 1;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let start = source.find("use warnings;").unwrap();
        let end = start + "use warnings;".len();
        let diagnostics = vec![make_diagnostic(
            start,
            end,
            "PL503",
            "`use warnings` inside a BEGIN block does not enable warnings for the rest of the file",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
        let action = actions
            .iter()
            .find(|action| action.title == "Move 'use warnings' to file scope")
            .expect("phase-scoped warnings quick fix");

        let rewritten = apply_action(source, action);
        assert!(rewritten.starts_with("#!/usr/bin/perl\nuse warnings;\n"));
        assert!(rewritten.contains("BEGIN {  }"));
    }

    #[test]
    fn test_parse_error_code_variants_route_to_same_quick_fix() {
        let source = "my $x =\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        for code in ["PL001", "PL002", "parse-error-missing-expression"] {
            let diagnostics = vec![make_diagnostic(
                source.len() - 1,
                source.len(),
                code,
                "Parse error near newline",
            )];
            let provider = CodeActionsProvider::new(source.to_string());
            let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
            assert!(
                !actions.is_empty(),
                "Expected parse error code {code} to produce at least one quick fix"
            );
        }
    }

    #[test]
    fn test_pl408_duplicate_hash_key_rename_action() {
        // PL408: duplicate hash key 'host' on a multiline hash — offers rename and delete
        let source = "my %cfg = (\n    host => 'db1',\n    host => 'db2',\n);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        // Second 'host' key
        let dup_start = source.rfind("host").unwrap();
        let dup_end = dup_start + "host".len();
        let diagnostics = vec![make_diagnostic(
            dup_start,
            dup_end,
            "PL408",
            "Duplicate hash key 'host' -- only the last value will be used",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        // Must offer a rename action
        let rename = actions
            .iter()
            .find(|a| a.title.contains("Rename") && a.title.contains("host"))
            .expect("PL408 should produce a rename action for duplicate key");
        assert_eq!(rename.edit.changes[0].new_text, "host_2");
        assert_eq!(rename.edit.changes[0].location.start, dup_start);
        assert_eq!(rename.edit.changes[0].location.end, dup_end);
    }

    #[test]
    fn test_pl408_duplicate_hash_key_delete_preferred_for_multiline() {
        // PL408: delete action is preferred and removes only the duplicate line
        let source = "my %cfg = (\n    host => 'db1',\n    host => 'db2',\n);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let dup_start = source.rfind("host").unwrap();
        let dup_end = dup_start + "host".len();
        let diagnostics = vec![make_diagnostic(
            dup_start,
            dup_end,
            "PL408",
            "Duplicate hash key 'host' -- only the last value will be used",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let delete = actions
            .iter()
            .find(|a| a.title.contains("Remove") && a.title.contains("host"))
            .expect("PL408 should produce a remove action for multiline duplicate key");
        assert!(delete.is_preferred, "remove action should be preferred");

        let rewritten = apply_action(source, delete);
        assert_eq!(rewritten, "my %cfg = (\n    host => 'db1',\n);\n");
    }

    #[test]
    fn test_pl408_inline_hash_only_rename_no_delete() {
        // PL408: inline hash — delete is suppressed; only rename is offered
        let source = "my %h = (foo => 1, foo => 2);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let dup_start = source.rfind("foo").unwrap();
        let dup_end = dup_start + "foo".len();
        let diagnostics = vec![make_diagnostic(
            dup_start,
            dup_end,
            "PL408",
            "Duplicate hash key 'foo' -- only the last value will be used",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(
            !actions.iter().any(|a| a.title.contains("Remove")),
            "should not offer delete for inline hash"
        );
        assert!(
            actions.iter().any(|a| a.title.contains("Rename") && a.title.contains("foo")),
            "should still offer rename for inline hash"
        );
    }

    #[test]
    fn test_pl408_single_quoted_key_rename_preserves_quotes() {
        let source = "my %h = (\n    'key' => 1,\n    'key' => 2,\n);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let dup_start = source.rfind("'key'").unwrap();
        let dup_end = dup_start + "'key'".len();
        let diagnostics = vec![make_diagnostic(
            dup_start,
            dup_end,
            "PL408",
            "Duplicate hash key 'key' -- only the last value will be used",
        )];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let rename = must_some(actions.iter().find(|a| a.title.contains("Rename")));
        assert_eq!(rename.edit.changes[0].new_text, "'key_2'");
    }
}
