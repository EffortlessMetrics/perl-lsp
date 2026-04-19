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

use crate::modernize;
use crate::quick_fixes;
use crate::refactors;
use crate::types::QuickFixDiagnostic;

pub use crate::types::{CodeAction, CodeActionKind};

use perl_diagnostics::codes::DiagnosticCode;
use perl_lsp_diagnostics::Diagnostic;
use perl_parser_core::Node;

/// Convert Diagnostic to QuickFixDiagnostic
///
/// Since Diagnostic already uses byte offsets, this is a simple copy.
fn to_quick_fix_diagnostic(diag: &Diagnostic) -> QuickFixDiagnostic {
    QuickFixDiagnostic {
        range: diag.range,
        message: diag.message.clone(),
        code: diag.code.clone(),
    }
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
                match code.as_str() {
                    // PL103: Undefined/undeclared variable
                    c if c == DiagnosticCode::UndefinedVariable.as_str() => {
                        actions.extend(quick_fixes::fix_undefined_variable(&self.source, &qf_diag));
                    }
                    // PL102: Unused variable
                    c if c == DiagnosticCode::UnusedVariable.as_str() => {
                        actions.extend(quick_fixes::fix_unused_variable(&self.source, &qf_diag));
                    }
                    // PL403: Assignment in condition
                    c if c == DiagnosticCode::AssignmentInCondition.as_str() => {
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
                    c if c == DiagnosticCode::DeprecatedDefined.as_str() => {
                        actions.extend(quick_fixes::fix_deprecated_defined(&self.source, &qf_diag));
                    }
                    // PL404: Numeric comparison with undef
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
                    code if code.starts_with("parse-error-") => {
                        actions.extend(quick_fixes::fix_parse_error(&self.source, &qf_diag, code));
                    }
                    // PL108: Unused parameter
                    c if c == DiagnosticCode::UnusedParameter.as_str() => {
                        actions.extend(quick_fixes::fix_unused_parameter(&qf_diag));
                    }
                    // PL104: Variable shadowing
                    c if c == DiagnosticCode::VariableShadowing.as_str() => {
                        actions.extend(quick_fixes::fix_variable_shadowing(&qf_diag));
                    }
                    // PL400: Bareword filehandle
                    c if c == DiagnosticCode::BarewordFilehandle.as_str() => {
                        actions.extend(quick_fixes::fix_bareword_filehandle(&qf_diag));
                    }
                    // Perl::Critic policy alias for bareword filehandle.
                    "InputOutput::ProhibitBarewordFileHandles" => {
                        actions.extend(quick_fixes::fix_bareword_filehandle(&qf_diag));
                    }
                    // PL401: Two-arg open
                    c if c == DiagnosticCode::TwoArgOpen.as_str() => {
                        actions.extend(quick_fixes::fix_two_arg_open(&qf_diag));
                    }
                    // Perl::Critic policy aliases for two-arg open.
                    "InputOutput::RequireBriefOpen" | "InputOutput::RequireThreeArgOpen" => {
                        actions.extend(quick_fixes::fix_two_arg_open(&qf_diag));
                    }
                    // Perl::Critic policies for missing strict/warnings.
                    "TestingAndDebugging::RequireUseStrict" => {
                        actions.extend(quick_fixes::add_use_strict());
                    }
                    "TestingAndDebugging::RequireUseWarnings" => {
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
                    c if c == DiagnosticCode::VariableRedeclaration.as_str() => {
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
                    c if c == DiagnosticCode::UnreachableCode.as_str() => {
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
                    _ => {}
                }
            }
        }

        // Source-level lints (not diagnostic-driven)
        // Only suggest shebang fix when the range includes the first line
        if range.0 == 0 || self.source[..range.0].lines().count() <= 1 {
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
mod tests {
    use super::*;
    use perl_lsp_diagnostics::DiagnosticSeverity;
    use perl_parser_core::Parser;
    use perl_tdd_support::must;

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
            actions
                .iter()
                .any(|a| a.title.contains("Declare") || a.title.contains("my")),
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
    fn test_hardcoded_shebang_suggests_portable() {
        let source = "#!/usr/bin/perl\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> = actions
            .iter()
            .filter(|a| a.title.contains("portable shebang"))
            .collect();

        assert_eq!(shebang_actions.len(), 1, "Expected one shebang action");
        assert_eq!(
            shebang_actions[0].edit.changes[0].new_text,
            "#!/usr/bin/env perl"
        );
    }

    #[test]
    fn test_hardcoded_shebang_preserves_flags() {
        let source = "#!/usr/bin/perl -w\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> = actions
            .iter()
            .filter(|a| a.title.contains("portable shebang"))
            .collect();

        assert_eq!(shebang_actions.len(), 1);
        assert_eq!(
            shebang_actions[0].edit.changes[0].new_text,
            "#!/usr/bin/env perl -w"
        );
    }

    #[test]
    fn test_env_perl_shebang_not_flagged() {
        let source = "#!/usr/bin/env perl\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> = actions
            .iter()
            .filter(|a| a.title.contains("portable shebang"))
            .collect();

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

        let shebang_actions: Vec<_> = actions
            .iter()
            .filter(|a| a.title.contains("portable shebang"))
            .collect();

        assert!(
            shebang_actions.is_empty(),
            "No shebang should not be flagged"
        );
    }

    #[test]
    fn test_local_bin_perl_shebang() {
        let source = "#!/usr/local/bin/perl\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> = actions
            .iter()
            .filter(|a| a.title.contains("portable shebang"))
            .collect();

        assert_eq!(shebang_actions.len(), 1, "Local bin perl should be flagged");
        assert_eq!(
            shebang_actions[0].edit.changes[0].new_text,
            "#!/usr/bin/env perl"
        );
    }

    #[test]
    fn test_shebang_with_taint_flag() {
        let source = "#!/usr/bin/perl -T\nuse strict;\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> = actions
            .iter()
            .filter(|a| a.title.contains("portable shebang"))
            .collect();

        assert_eq!(shebang_actions.len(), 1);
        assert_eq!(
            shebang_actions[0].edit.changes[0].new_text,
            "#!/usr/bin/env perl -T"
        );
    }

    #[test]
    fn test_bash_shebang_not_flagged() {
        let source = "#!/bin/bash\necho hello\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let diagnostics = vec![];

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        let shebang_actions: Vec<_> = actions
            .iter()
            .filter(|a| a.title.contains("portable shebang"))
            .collect();

        assert!(
            shebang_actions.is_empty(),
            "Non-perl shebang should not be flagged"
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
        assert!(
            actions
                .iter()
                .any(|a| a.title.contains("bareword filehandle"))
        );
        assert!(
            actions
                .iter()
                .any(|a| a.title.contains("three-argument open() for safety"))
        );
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
        assert!(
            actions
                .iter()
                .any(|a| a.title.contains("Remove unused variable"))
        );
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
}
