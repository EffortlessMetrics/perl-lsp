//! Code actions and quick fixes for Perl
//!
//! This module provides automated fixes for common issues and refactoring actions.
//!
//! # PSTX Pipeline Integration
//!
//! Code actions integrate with the PSTX (Parse → Index → Navigate → Complete → Analyze) pipeline:
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

mod ast_utils;
mod enhanced;
mod quick_fixes;
mod refactors;
mod types;

pub use enhanced::EnhancedCodeActionsProvider;
pub use types::{CodeAction, CodeActionEdit, CodeActionKind};

use crate::ide::lsp_compat::diagnostics::Diagnostic;
use perl_parser_core::Node;

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
            if let Some(code) = &diagnostic.code {
                match code.as_str() {
                    "undefined-variable" | "undeclared-variable" => {
                        actions
                            .extend(quick_fixes::fix_undefined_variable(&self.source, diagnostic));
                    }
                    "unused-variable" => {
                        actions.extend(quick_fixes::fix_unused_variable(&self.source, diagnostic));
                    }
                    "assignment-in-condition" => {
                        actions.extend(quick_fixes::fix_assignment_in_condition(
                            &self.source,
                            diagnostic,
                        ));
                    }
                    "missing-strict" => {
                        actions.extend(quick_fixes::add_use_strict());
                    }
                    "missing-warnings" => {
                        actions.extend(quick_fixes::add_use_warnings());
                    }
                    "deprecated-defined" => {
                        actions
                            .extend(quick_fixes::fix_deprecated_defined(&self.source, diagnostic));
                    }
                    "numeric-undef" => {
                        actions.extend(quick_fixes::fix_numeric_undef(&self.source, diagnostic));
                    }
                    "unquoted-bareword" => {
                        actions.extend(quick_fixes::fix_bareword(&self.source, diagnostic));
                    }
                    code if code.starts_with("parse-error-") => {
                        actions.extend(quick_fixes::fix_parse_error(
                            &self.source,
                            diagnostic,
                            code,
                        ));
                    }
                    "unused-parameter" => {
                        actions.extend(quick_fixes::fix_unused_parameter(diagnostic));
                    }
                    "variable-shadowing" => {
                        actions.extend(quick_fixes::fix_variable_shadowing(diagnostic));
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
    use crate::ide::lsp_compat::diagnostics::DiagnosticsProvider;
    use perl_parser_core::Parser;
    use perl_tdd_support::must;

    #[test]
    fn test_undefined_variable_fix() {
        let source = "use strict;\nprint $undefined;";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let diag_provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = diag_provider.get_diagnostics(&ast, &[], source);

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        // Debug output
        eprintln!("Diagnostics: {:?}", diagnostics);
        eprintln!("Actions: {:?}", actions);

        assert!(!diagnostics.is_empty(), "Expected diagnostics for undefined variable");
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

        let diag_provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = diag_provider.get_diagnostics(&ast, &[], source);

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|a| a.title.contains("==")));
    }
}
