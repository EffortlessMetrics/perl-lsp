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
//! ```no_run
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

use crate::quick_fixes;
use crate::refactors;
use crate::types::QuickFixDiagnostic;

pub use crate::types::{CodeAction, CodeActionKind};

use perl_lsp_diagnostics::Diagnostic;
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
                match code.as_str() {
                    "undefined-variable" | "undeclared-variable" => {
                        actions.extend(quick_fixes::fix_undefined_variable(&self.source, &qf_diag));
                    }
                    "unused-variable" => {
                        actions.extend(quick_fixes::fix_unused_variable(&self.source, &qf_diag));
                    }
                    "assignment-in-condition" => {
                        actions.extend(quick_fixes::fix_assignment_in_condition(
                            &self.source,
                            &qf_diag,
                        ));
                    }
                    "missing-strict" => {
                        actions.extend(quick_fixes::add_use_strict());
                    }
                    "missing-warnings" => {
                        actions.extend(quick_fixes::add_use_warnings());
                    }
                    "deprecated-defined" => {
                        actions.extend(quick_fixes::fix_deprecated_defined(&self.source, &qf_diag));
                    }
                    "numeric-undef" => {
                        actions.extend(quick_fixes::fix_numeric_undef(&self.source, &qf_diag));
                    }
                    "unquoted-bareword" => {
                        actions.extend(quick_fixes::fix_bareword(&self.source, &qf_diag));
                    }
                    code if code.starts_with("parse-error-") => {
                        actions.extend(quick_fixes::fix_parse_error(&self.source, &qf_diag, code));
                    }
                    "unused-parameter" => {
                        actions.extend(quick_fixes::fix_unused_parameter(&qf_diag));
                    }
                    "variable-shadowing" => {
                        actions.extend(quick_fixes::fix_variable_shadowing(&qf_diag));
                    }
                    _ => {}
                }
            }
        }

        // Get refactoring actions for selection
        actions.extend(refactors::get_refactoring_actions(&self.source, ast, range));

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
        }
    }

    #[test]
    fn test_undefined_variable_fix() {
        let source = "use strict;\nprint $undefined;";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        // Create a synthetic diagnostic for 'undefined-variable'
        // "$undefined" starts at byte offset 18 (after "use strict;\nprint ")
        let diagnostics = vec![make_diagnostic(
            18, // start of "$undefined"
            28, // end of "$undefined"
            "undefined-variable",
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

        // Create a synthetic diagnostic for 'assignment-in-condition'
        // "$x = 5" is at bytes 4-10
        let diagnostics = vec![make_diagnostic(
            4,  // start of "$x = 5"
            10, // end of "$x = 5"
            "assignment-in-condition",
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
}
