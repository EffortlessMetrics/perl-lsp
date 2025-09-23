//! Code actions and quick fixes for Perl
//!
//! This module provides automated fixes for common issues and refactoring actions.
//!
//! # Related Modules
//!
//! See also [`crate::diagnostics`] for issue detection and [`crate::import_optimizer`]
//! for import-related code actions.

use crate::ast::{Node, NodeKind, SourceLocation};
use crate::diagnostics::Diagnostic;
use crate::rename::TextEdit;

/// A code action that can be applied to fix an issue
///
/// Code actions represent automated fixes for common issues and refactoring
/// operations that can be applied to Perl source code.
#[derive(Debug, Clone)]
pub struct CodeAction {
    /// Human-readable title describing the action
    pub title: String,
    /// The kind/category of code action
    pub kind: CodeActionKind,
    /// Diagnostic codes this action fixes
    pub diagnostics: Vec<String>,
    /// The edit operations to apply
    pub edit: CodeActionEdit,
    /// Whether this action is the preferred choice
    pub is_preferred: bool,
}

/// Kind of code action
///
/// Categorizes the type of code action to help editors organize and present
/// actions to users appropriately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeActionKind {
    /// Quick fix for a diagnostic issue
    QuickFix,
    /// General refactoring operation
    Refactor,
    /// Extract code into a new construct
    RefactorExtract,
    /// Inline a construct into its usage sites
    RefactorInline,
    /// Rewrite code using a different pattern
    RefactorRewrite,
    /// Source code organization action
    Source,
    /// Organize imports action
    SourceOrganizeImports,
    /// Fix all issues action
    SourceFixAll,
}

/// Edit to apply for a code action
///
/// Contains the specific text changes needed to apply a code action.
#[derive(Debug, Clone)]
pub struct CodeActionEdit {
    /// List of text edits to apply
    pub changes: Vec<TextEdit>,
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
            if let Some(code) = &diagnostic.code {
                match code.as_str() {
                    "undefined-variable" | "undeclared-variable" => {
                        actions.extend(self.fix_undefined_variable(diagnostic));
                    }
                    "unused-variable" => {
                        actions.extend(self.fix_unused_variable(diagnostic));
                    }
                    "assignment-in-condition" => {
                        actions.extend(self.fix_assignment_in_condition(diagnostic));
                    }
                    "missing-strict" => {
                        actions.extend(self.add_use_strict());
                    }
                    "missing-warnings" => {
                        actions.extend(self.add_use_warnings());
                    }
                    "deprecated-defined" => {
                        actions.extend(self.fix_deprecated_defined(diagnostic));
                    }
                    "numeric-undef" => {
                        actions.extend(self.fix_numeric_undef(diagnostic));
                    }
                    _ => {}
                }
            }
        }

        // Get refactoring actions for the selection
        actions.extend(self.get_refactoring_actions(ast, range));

        actions
    }

    /// Fix undefined variable by declaring it
    fn fix_undefined_variable(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Extract variable name from diagnostic message
        if let Some(var_name) = diagnostic.message.split('\'').nth(1) {
            // Find the best place to insert declaration
            let insert_pos = self.find_declaration_position(diagnostic.range.0);

            // Add 'my' declaration
            actions.push(CodeAction {
                title: format!("Declare '{}' with 'my'", var_name),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec!["undefined-variable".to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: insert_pos, end: insert_pos },
                        new_text: format!("my {};\n", var_name),
                    }],
                },
                is_preferred: true,
            });

            // Add 'our' declaration
            actions.push(CodeAction {
                title: format!("Declare '{}' with 'our'", var_name),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec!["undefined-variable".to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: insert_pos, end: insert_pos },
                        new_text: format!("our {};\n", var_name),
                    }],
                },
                is_preferred: false,
            });
        }

        actions
    }

    /// Fix unused variable by removing it
    fn fix_unused_variable(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Find the declaration line
        let line_start = self.source[..diagnostic.range.0].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line_end = self.source[diagnostic.range.1..]
            .find('\n')
            .map(|p| diagnostic.range.1 + p)
            .unwrap_or(self.source.len());

        actions.push(CodeAction {
            title: "Remove unused variable".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["unused-variable".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: line_start, end: line_end + 1 },
                    new_text: String::new(),
                }],
            },
            is_preferred: true,
        });

        // Add underscore prefix to mark as intentionally unused
        if let Some(var_name) = diagnostic.message.split('\'').nth(1) {
            actions.push(CodeAction {
                title: format!("Rename to '_{}'", var_name),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec!["unused-variable".to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.0,
                            end: diagnostic.range.1,
                        },
                        new_text: format!("_{}", var_name),
                    }],
                },
                is_preferred: false,
            });
        }

        actions
    }

    /// Fix assignment in condition
    fn fix_assignment_in_condition(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Change = to ==
        let assignment_pos = self.source[diagnostic.range.0..diagnostic.range.1]
            .find('=')
            .map(|p| diagnostic.range.0 + p);

        if let Some(pos) = assignment_pos {
            actions.push(CodeAction {
                title: "Change to comparison (==)".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec!["assignment-in-condition".to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: pos, end: pos + 1 },
                        new_text: "==".to_string(),
                    }],
                },
                is_preferred: true,
            });

            // Wrap in parentheses to make intention clear
            actions.push(CodeAction {
                title: "Keep assignment (add parentheses)".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec!["assignment-in-condition".to_string()],
                edit: CodeActionEdit {
                    changes: vec![
                        TextEdit {
                            location: SourceLocation {
                                start: diagnostic.range.0,
                                end: diagnostic.range.0,
                            },
                            new_text: "(".to_string(),
                        },
                        TextEdit {
                            location: SourceLocation {
                                start: diagnostic.range.1,
                                end: diagnostic.range.1,
                            },
                            new_text: ")".to_string(),
                        },
                    ],
                },
                is_preferred: false,
            });
        }

        actions
    }

    /// Add 'use strict' pragma
    fn add_use_strict(&self) -> Vec<CodeAction> {
        vec![CodeAction {
            title: "Add 'use strict'".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["missing-strict".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: 0, end: 0 },
                    new_text: "use strict;\n".to_string(),
                }],
            },
            is_preferred: true,
        }]
    }

    /// Add 'use warnings' pragma
    fn add_use_warnings(&self) -> Vec<CodeAction> {
        vec![CodeAction {
            title: "Add 'use warnings'".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["missing-warnings".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: 0, end: 0 },
                    new_text: "use warnings;\n".to_string(),
                }],
            },
            is_preferred: true,
        }]
    }

    /// Fix deprecated 'defined @array' or 'defined %hash'
    fn fix_deprecated_defined(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Extract the array/hash from the diagnostic
        if let Some(start) = self.source[diagnostic.range.0..diagnostic.range.1].find("defined") {
            let defined_start = diagnostic.range.0 + start;
            let arg_start = defined_start + 7; // "defined".len()

            // Find the argument
            let arg_text = &self.source[arg_start..diagnostic.range.1].trim();

            actions.push(CodeAction {
                title: format!("Replace with 'if ({})'", arg_text),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec!["deprecated-defined".to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: defined_start, end: diagnostic.range.1 },
                        new_text: arg_text.to_string(),
                    }],
                },
                is_preferred: true,
            });
        }

        actions
    }

    /// Fix numeric comparison with undef
    fn fix_numeric_undef(&self, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Add defined check
        actions.push(CodeAction {
            title: "Add defined check".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["numeric-undef".to_string()],
            edit: CodeActionEdit {
                changes: vec![
                    TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.0,
                            end: diagnostic.range.0,
                        },
                        new_text: "defined(".to_string(),
                    },
                    TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.1,
                            end: diagnostic.range.1,
                        },
                        new_text: ")".to_string(),
                    },
                ],
            },
            is_preferred: true,
        });

        // Use // operator
        if self.source[diagnostic.range.0..diagnostic.range.1].contains("==") {
            actions.push(CodeAction {
                title: "Use defined-or operator (//)".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec!["numeric-undef".to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.0,
                            end: diagnostic.range.1,
                        },
                        new_text: "// 0".to_string(), // Default to 0
                    }],
                },
                is_preferred: false,
            });
        }

        actions
    }

    /// Get refactoring actions for a selection
    fn get_refactoring_actions(&self, ast: &Node, range: (usize, usize)) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Use the enhanced provider for better refactorings
        let enhanced_provider =
            crate::code_actions_enhanced::EnhancedCodeActionsProvider::new(self.source.clone());
        actions.extend(enhanced_provider.get_enhanced_refactoring_actions(ast, range));

        // Keep basic refactorings as fallback
        if let Some(node) = self.find_node_at_range(ast, range) {
            match &node.kind {
                // Extract variable (basic version, enhanced version is better)
                NodeKind::FunctionCall { .. } | NodeKind::Binary { .. } if actions.is_empty() => {
                    actions.push(CodeAction {
                        title: "Extract to variable".to_string(),
                        kind: CodeActionKind::RefactorExtract,
                        diagnostics: Vec::new(),
                        edit: self.extract_variable(node, range),
                        is_preferred: false,
                    });
                }

                // Extract function (basic version)
                NodeKind::Block { .. } if actions.is_empty() => {
                    actions.push(CodeAction {
                        title: "Extract to function".to_string(),
                        kind: CodeActionKind::RefactorExtract,
                        diagnostics: Vec::new(),
                        edit: self.extract_function(node, range),
                        is_preferred: false,
                    });
                }

                _ => {}
            }
        }

        actions
    }

    /// Extract expression to variable
    fn extract_variable(&self, node: &Node, _range: (usize, usize)) -> CodeActionEdit {
        let expr_text = &self.source[node.location.start..node.location.end];
        let var_name = "$extracted_var";

        // Find statement start
        let stmt_start = self.find_statement_start(node.location.start);

        CodeActionEdit {
            changes: vec![
                // Insert variable declaration
                TextEdit {
                    location: SourceLocation { start: stmt_start, end: stmt_start },
                    new_text: format!("my {} = {};\n", var_name, expr_text),
                },
                // Replace expression with variable
                TextEdit { location: node.location, new_text: var_name.to_string() },
            ],
        }
    }

    /// Extract statements to function
    fn extract_function(&self, node: &Node, _range: (usize, usize)) -> CodeActionEdit {
        let body_text = &self.source[node.location.start..node.location.end];
        let func_name = "extracted_function";

        // Find a good place to insert the function
        let insert_pos = self.find_function_insert_position();

        CodeActionEdit {
            changes: vec![
                // Insert function definition
                TextEdit {
                    location: SourceLocation { start: insert_pos, end: insert_pos },
                    new_text: format!("\nsub {} {{\n{}\n}}\n", func_name, body_text),
                },
                // Replace statements with function call
                TextEdit { location: node.location, new_text: format!("{}();", func_name) },
            ],
        }
    }

    /// Find the best position to insert a declaration
    fn find_declaration_position(&self, error_pos: usize) -> usize {
        // Find the start of the current statement
        self.find_statement_start(error_pos)
    }

    /// Find the start of the current statement
    fn find_statement_start(&self, pos: usize) -> usize {
        // Look backwards for statement boundary
        let mut i = pos.saturating_sub(1);
        while i > 0 {
            if self.source.chars().nth(i) == Some(';') || self.source.chars().nth(i) == Some('\n') {
                return i + 1;
            }
            i = i.saturating_sub(1);
        }
        0
    }

    /// Find a good position to insert a function
    fn find_function_insert_position(&self) -> usize {
        // For now, insert at the end of the file
        self.source.len()
    }

    /// Find node at the given range
    #[allow(clippy::only_used_in_recursion)]
    fn find_node_at_range<'a>(&self, node: &'a Node, range: (usize, usize)) -> Option<&'a Node> {
        // Check if this node contains the range
        if node.location.start <= range.0 && node.location.end >= range.1 {
            // Check children for more specific match based on node kind
            match &node.kind {
                NodeKind::Program { statements } => {
                    for stmt in statements {
                        if let Some(result) = self.find_node_at_range(stmt, range) {
                            return Some(result);
                        }
                    }
                }
                NodeKind::Block { statements } => {
                    for stmt in statements {
                        if let Some(result) = self.find_node_at_range(stmt, range) {
                            return Some(result);
                        }
                    }
                }
                NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                    if let Some(result) = self.find_node_at_range(condition, range) {
                        return Some(result);
                    }
                    if let Some(result) = self.find_node_at_range(then_branch, range) {
                        return Some(result);
                    }
                    for (cond, branch) in elsif_branches {
                        if let Some(result) = self.find_node_at_range(cond, range) {
                            return Some(result);
                        }
                        if let Some(result) = self.find_node_at_range(branch, range) {
                            return Some(result);
                        }
                    }
                    if let Some(branch) = else_branch {
                        if let Some(result) = self.find_node_at_range(branch, range) {
                            return Some(result);
                        }
                    }
                }
                NodeKind::Binary { left, right, .. } => {
                    if let Some(result) = self.find_node_at_range(left, range) {
                        return Some(result);
                    }
                    if let Some(result) = self.find_node_at_range(right, range) {
                        return Some(result);
                    }
                }
                _ => {}
            }
            return Some(node);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    use crate::diagnostics::DiagnosticsProvider;

    #[test]
    fn test_undefined_variable_fix() {
        let source = "use strict;\nprint $undefined;";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

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
        let ast = parser.parse().unwrap();

        let diag_provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = diag_provider.get_diagnostics(&ast, &[], source);

        let provider = CodeActionsProvider::new(source.to_string());
        let actions = provider.get_code_actions(&ast, (0, source.len()), &diagnostics);

        assert!(actions.iter().any(|a| a.title.contains("==")));
    }
}
