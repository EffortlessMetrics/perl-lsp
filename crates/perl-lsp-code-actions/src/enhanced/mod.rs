//! Enhanced code actions with additional refactorings
//!
//! This module extends the base code actions with more sophisticated refactorings,
//! including extract variable, extract subroutine, loop conversion, and import management.
//!
//! # Architecture
//!
//! Enhanced actions are organized into focused submodules:
//!
//! - **extract_variable**: Extract selected expression into a named variable
//! - **extract_subroutine**: Extract code block into a new subroutine
//! - **loop_conversion**: Convert between loop styles (for/foreach/while)
//! - **import_management**: Organize and add/remove use statements
//! - **postfix**: Postfix completion-style actions (e.g., `.if`, `.unless`)
//! - **error_checking**: Add error handling around expressions
//! - **helpers**: Shared utilities for text manipulation and position mapping
//!
//! # Refactoring Categories
//!
//! Actions are categorized following LSP CodeActionKind:
//!
//! - **refactor.extract**: Extract variable, extract subroutine
//! - **refactor.rewrite**: Loop conversion, error wrapping
//! - **source.organizeImports**: Import management
//!
//! # Performance Characteristics
//!
//! - **Action generation**: <50ms for typical refactoring suggestions
//! - **Edit computation**: <100ms for complex multi-location edits
//! - **Incremental analysis**: Leverages parsed AST for efficient analysis

use crate::types::CodeAction;
use perl_parser_core::ast::{Node, NodeKind};

mod error_checking;
mod extract_subroutine;
mod extract_variable;
mod helpers;
mod import_management;
mod loop_conversion;
mod postfix;

use helpers::Helpers;

/// Enhanced code actions provider with additional refactorings
pub struct EnhancedCodeActionsProvider {
    source: String,
    lines: Vec<String>,
}

impl EnhancedCodeActionsProvider {
    /// Create a new enhanced code actions provider
    pub fn new(source: String) -> Self {
        let lines = source.lines().map(|s| s.to_string()).collect();
        Self { source, lines }
    }

    /// Get additional refactoring actions
    pub fn get_enhanced_refactoring_actions(
        &self,
        ast: &Node,
        range: (usize, usize),
    ) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Find all nodes that overlap the range and collect actions
        self.collect_actions_for_range(ast, range, &mut actions);

        // Global actions (not node-specific)
        actions.extend(self.get_global_refactorings(ast));

        actions
    }

    /// Recursively collect actions for all nodes in range
    fn collect_actions_for_range(
        &self,
        node: &Node,
        range: (usize, usize),
        actions: &mut Vec<CodeAction>,
    ) {
        // Check if this node overlaps the range
        if node.location.start <= range.1 && node.location.end >= range.0 {
            let helpers = Helpers::new(&self.source, &self.lines);

            // Extract variable (enhanced version)
            if self.is_extractable_expression(node) {
                actions.push(extract_variable::create_extract_variable_action(
                    node,
                    &self.source,
                    &helpers,
                ));
            }

            // Convert old-style loops
            if let Some(action) = loop_conversion::convert_loop_style(node, &self.source) {
                actions.push(action);
            }

            // Add error checking
            if let Some(action) = error_checking::add_error_checking(node, &self.source) {
                actions.push(action);
            }

            // Convert to postfix
            if let Some(action) = postfix::convert_to_postfix(node, &self.source) {
                actions.push(action);
            }

            // Extract subroutine
            if self.is_extractable_block(node) {
                actions.push(extract_subroutine::create_extract_subroutine_action(
                    node,
                    &self.source,
                    &helpers,
                ));
            }
        }

        // Recursively check children
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.collect_actions_for_range(stmt, range, actions);
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.collect_actions_for_range(stmt, range, actions);
                }
            }
            NodeKind::ExpressionStatement { expression } => {
                self.collect_actions_for_range(expression, range, actions);
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.collect_actions_for_range(condition, range, actions);
                self.collect_actions_for_range(then_branch, range, actions);
                for (cond, branch) in elsif_branches {
                    self.collect_actions_for_range(cond, range, actions);
                    self.collect_actions_for_range(branch, range, actions);
                }
                if let Some(branch) = else_branch {
                    self.collect_actions_for_range(branch, range, actions);
                }
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    self.collect_actions_for_range(arg, range, actions);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                self.collect_actions_for_range(left, range, actions);
                self.collect_actions_for_range(right, range, actions);
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                self.collect_actions_for_range(lhs, range, actions);
                self.collect_actions_for_range(rhs, range, actions);
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                self.collect_actions_for_range(variable, range, actions);
                if let Some(init) = initializer {
                    self.collect_actions_for_range(init, range, actions);
                }
            }
            NodeKind::For { init, condition, update, body, .. } => {
                if let Some(init) = init {
                    self.collect_actions_for_range(init, range, actions);
                }
                if let Some(condition) = condition {
                    self.collect_actions_for_range(condition, range, actions);
                }
                if let Some(update) = update {
                    self.collect_actions_for_range(update, range, actions);
                }
                self.collect_actions_for_range(body, range, actions);
            }
            NodeKind::Foreach { variable, list, body, continue_block } => {
                self.collect_actions_for_range(variable, range, actions);
                self.collect_actions_for_range(list, range, actions);
                self.collect_actions_for_range(body, range, actions);
                if let Some(cb) = continue_block {
                    self.collect_actions_for_range(cb, range, actions);
                }
            }
            NodeKind::While { condition, body, .. } => {
                self.collect_actions_for_range(condition, range, actions);
                self.collect_actions_for_range(body, range, actions);
            }
            _ => {}
        }
    }

    /// Check if expression is extractable
    fn is_extractable_expression(&self, node: &Node) -> bool {
        matches!(
            &node.kind,
            NodeKind::FunctionCall { .. }
                | NodeKind::Binary { .. }
                | NodeKind::Unary { .. }
                | NodeKind::MethodCall { .. }
                | NodeKind::Ternary { .. }
        )
    }

    /// Check if block is extractable
    fn is_extractable_block(&self, node: &Node) -> bool {
        matches!(&node.kind, NodeKind::Block { .. })
    }

    /// Get global refactoring actions
    fn get_global_refactorings(&self, ast: &Node) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        let helpers = Helpers::new(&self.source, &self.lines);

        // Add missing imports
        if let Some(action) = import_management::add_missing_imports(ast, &self.source, &helpers) {
            actions.push(action);
        }

        // Organize imports
        if let Some(action) = import_management::organize_imports(ast, &self.source, &helpers) {
            actions.push(action);
        }

        // Add pragmas
        actions.extend(self.add_recommended_pragmas(&helpers));

        actions
    }

    /// Add recommended pragmas
    fn add_recommended_pragmas(&self, helpers: &Helpers<'_>) -> Vec<CodeAction> {
        use crate::types::{CodeAction, CodeActionEdit, CodeActionKind};
        use perl_lsp_rename::TextEdit;
        use perl_parser_core::ast::SourceLocation;

        let mut actions = Vec::new();

        // Check for missing strict and warnings
        let has_strict = self.source.contains("use strict");
        let has_warnings = self.source.contains("use warnings");

        if !has_strict || !has_warnings {
            let mut pragmas = Vec::new();
            if !has_strict {
                pragmas.push("use strict;");
            }
            if !has_warnings {
                pragmas.push("use warnings;");
            }

            let insert_pos = helpers.find_pragma_insert_position();

            actions.push(CodeAction {
                title: "Add recommended pragmas".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: Vec::new(),
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: insert_pos, end: insert_pos },
                        new_text: format!("{}\n", pragmas.join("\n")),
                    }],
                },
                is_preferred: true,
            });
        }

        // Add utf8 support if missing
        if !self.source.contains("use utf8") && helpers.has_non_ascii_content() {
            let insert_pos = helpers.find_pragma_insert_position();

            actions.push(CodeAction {
                title: "Add UTF-8 support".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: Vec::new(),
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: insert_pos, end: insert_pos },
                        new_text: "use utf8;\nuse open qw(:std :utf8);\n".to_string(),
                    }],
                },
                is_preferred: false,
            });
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_tdd_support::must;

    #[test]
    fn test_extract_variable() {
        let source = "my $x = length($string) + 10;";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 23)); // Select "length($string)"

        // Debug: print all actions
        for action in &actions {
            eprintln!("Action: {}", action.title);
        }

        assert!(!actions.is_empty(), "Expected at least one action");
        assert!(
            actions.iter().any(|a| a.title.contains("Extract")),
            "Expected an Extract action, got: {:?}",
            actions.iter().map(|a| &a.title).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_add_error_checking() {
        let source = "open my $fh, '<', 'file.txt';";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 30));

        assert!(actions.iter().any(|a| a.title.contains("error checking")));
    }

    #[test]
    fn test_convert_to_postfix() {
        let source = "if ($debug) { print \"Debug\\n\"; }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_enhanced_refactoring_actions(&ast, (0, source.len()));

        assert!(actions.iter().any(|a| a.title.contains("postfix")));
    }
}
