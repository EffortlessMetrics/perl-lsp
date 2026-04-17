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
use std::collections::HashSet;

mod error_checking;
mod extract_subroutine;
mod extract_variable;
mod helpers;
mod import_management;
mod inline_variable;
mod loop_conversion;
mod postfix;
mod signature_actions;

use helpers::Helpers;

/// Enhanced code actions provider with additional refactorings
pub struct EnhancedCodeActionsProvider {
    source: String,
    lines: Vec<String>,
    ast_root: Option<Node>,
}

impl EnhancedCodeActionsProvider {
    /// Create a new enhanced code actions provider
    pub fn new(source: String) -> Self {
        let lines = source.lines().map(|s| s.to_string()).collect();
        Self { source, lines, ast_root: None }
    }

    /// Get additional refactoring actions
    pub fn get_enhanced_refactoring_actions(
        &mut self,
        ast: &Node,
        range: (usize, usize),
    ) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        // Track (stmt_start, var_name) pairs already emitted to prevent duplicate
        // extract-variable actions when both a parent and child node overlap the range.
        let mut extract_var_seen: HashSet<(usize, String)> = HashSet::new();

        // Store ast_root for use in inline variable action
        self.ast_root = Some(ast.clone());

        // Find all nodes that overlap the range and collect actions
        self.collect_actions_for_range(ast, range, false, &mut actions, &mut extract_var_seen);

        // Signature refactoring: collect add-parameter actions for any subroutine
        // node whose span overlaps the requested range.
        self.collect_signature_actions(ast, ast, range, &mut actions);

        // Global actions (not node-specific)
        actions.extend(self.get_global_refactorings(ast));

        actions
    }

    /// Walk the AST and emit signature refactoring actions for subroutine nodes
    /// that overlap `range`.  `ast_root` is always the full program AST so that
    /// call-site collection can search the entire file.
    fn collect_signature_actions(
        &self,
        node: &Node,
        ast_root: &Node,
        range: (usize, usize),
        actions: &mut Vec<CodeAction>,
    ) {
        // Prune subtrees that cannot overlap the range.
        if node.location.end < range.0 || node.location.start > range.1 {
            return;
        }

        if let Some(action) = signature_actions::add_parameter_action(&self.source, node, ast_root)
        {
            actions.push(action);
        }

        // Recurse into children
        match &node.kind {
            NodeKind::Program { statements } => {
                for s in statements {
                    self.collect_signature_actions(s, ast_root, range, actions);
                }
            }
            NodeKind::Block { statements } => {
                for s in statements {
                    self.collect_signature_actions(s, ast_root, range, actions);
                }
            }
            NodeKind::ExpressionStatement { expression } => {
                self.collect_signature_actions(expression, ast_root, range, actions);
            }
            NodeKind::VariableDeclaration { initializer: Some(init), .. } => {
                self.collect_signature_actions(init, ast_root, range, actions);
            }
            NodeKind::Subroutine { body, .. } => {
                self.collect_signature_actions(body, ast_root, range, actions);
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.collect_signature_actions(condition, ast_root, range, actions);
                self.collect_signature_actions(then_branch, ast_root, range, actions);
                for (cond, branch) in elsif_branches {
                    self.collect_signature_actions(cond, ast_root, range, actions);
                    self.collect_signature_actions(branch, ast_root, range, actions);
                }
                if let Some(b) = else_branch {
                    self.collect_signature_actions(b, ast_root, range, actions);
                }
            }
            _ => {}
        }
    }

    /// Recursively collect actions for all nodes in range.
    ///
    /// `is_control_body` is `true` when the current node is the body block of a
    /// control-flow construct (`If`, `While`, `For`, `Foreach`, `Subroutine`).
    /// In that case the node is not offered as "Extract to subroutine" — only
    /// standalone bare blocks are extractable.
    ///
    /// # Range bounding
    ///
    /// This function returns immediately when the node's span does not overlap the
    /// requested range.  Because AST children are always contained within their
    /// parent's span, a non-overlapping parent implies none of its descendants can
    /// overlap either — so the entire subtree is pruned in one check.  This keeps
    /// code-action collection O(nodes in range) rather than O(total AST nodes),
    /// which is critical for responsiveness on large files (>5000 lines).
    fn collect_actions_for_range(
        &self,
        ast_root: &Node,
        node: &Node,
        range: (usize, usize),
        is_control_body: bool,
        actions: &mut Vec<CodeAction>,
        extract_var_seen: &mut HashSet<(usize, String)>,
    ) {
        // Prune entire subtree when the node is completely outside the range.
        // Children are always within the parent span, so if the parent doesn't
        // overlap the range neither can any child.
        if node.location.end < range.0 || node.location.start > range.1 {
            return;
        }

        // The node overlaps the range — collect applicable actions.
        let helpers = Helpers::new(&self.source, &self.lines);

        // Extract variable — only emit when the node's end reaches or exceeds the
        // selection's end. This prevents duplicate actions for nested expressions:
        // when both a Binary(8..25) and its inner FunctionCall(8..20) overlap a
        // selection (8..25), the FunctionCall's end (20) is before the selection's
        // end (25) and is skipped; only the outermost matching node emits an action.
        // Partial-left overlap (cursor inside expression) is still supported.
        let node_reaches_selection_end = node.location.end >= range.1;
        if node_reaches_selection_end && self.is_extractable_expression(node) {
            let action =
                extract_variable::create_extract_variable_action(node, &self.source, &helpers);
            if let Some(decl) = action.edit.changes.first() {
                let key = (decl.location.start, decl.new_text.clone());
                if extract_var_seen.insert(key) {
                    actions.push(action);
                }
            } else {
                actions.push(action);
            }
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

        // Extract subroutine — only for standalone blocks, not control-flow bodies
        if !is_control_body && self.is_extractable_block(node) {
            actions.push(extract_subroutine::create_extract_subroutine_action(
                node,
                &self.source,
                &helpers,
            ));
        }

        // Inline variable — only emit when the node is a VariableDeclaration
        // that overlaps the selection. Uses extract_var_seen to avoid duplicate
        // inline actions when multiple nodes overlap the range.
        if let NodeKind::VariableDeclaration { .. } = &node.kind {
            if node.location.start <= range.1 && node.location.end >= range.0 {
                if let Some(action) = inline_variable::create_inline_variable_action(
                    &self.source,
                    ast,
                    node,
                ) {
                    // Use the declaration start and variable name as the dedup key
                    let var_name = action.title.split('\'').nth(1).unwrap_or("").to_string();
                    let key = (node.location.start, var_name);
                    if extract_var_seen.insert(key) {
                        actions.push(action);
                    }
                }
            }
        }

        // Recursively check children, flagging control-flow body blocks
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.collect_actions_for_range(stmt, range, false, actions, extract_var_seen);
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.collect_actions_for_range(stmt, range, false, actions, extract_var_seen);
                }
            }
            NodeKind::ExpressionStatement { expression } => {
                self.collect_actions_for_range(expression, range, false, actions, extract_var_seen);
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.collect_actions_for_range(condition, range, false, actions, extract_var_seen);
                self.collect_actions_for_range(
                    then_branch,
                    range,
                    true, // then-body is a control-flow block
                    actions,
                    extract_var_seen,
                );
                for (cond, branch) in elsif_branches {
                    self.collect_actions_for_range(cond, range, false, actions, extract_var_seen);
                    self.collect_actions_for_range(branch, range, true, actions, extract_var_seen);
                }
                if let Some(branch) = else_branch {
                    self.collect_actions_for_range(branch, range, true, actions, extract_var_seen);
                }
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    self.collect_actions_for_range(arg, range, false, actions, extract_var_seen);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                self.collect_actions_for_range(left, range, false, actions, extract_var_seen);
                self.collect_actions_for_range(right, range, false, actions, extract_var_seen);
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                self.collect_actions_for_range(lhs, range, false, actions, extract_var_seen);
                self.collect_actions_for_range(rhs, range, false, actions, extract_var_seen);
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                self.collect_actions_for_range(variable, range, false, actions, extract_var_seen);
                if let Some(init) = initializer {
                    self.collect_actions_for_range(init, range, false, actions, extract_var_seen);
                }
            }
            NodeKind::For { init, condition, update, body, .. } => {
                if let Some(init) = init {
                    self.collect_actions_for_range(init, range, false, actions, extract_var_seen);
                }
                if let Some(condition) = condition {
                    self.collect_actions_for_range(
                        condition,
                        range,
                        false,
                        actions,
                        extract_var_seen,
                    );
                }
                if let Some(update) = update {
                    self.collect_actions_for_range(update, range, false, actions, extract_var_seen);
                }
                self.collect_actions_for_range(
                    body,
                    range,
                    true, // loop body is a control-flow block
                    actions,
                    extract_var_seen,
                );
            }
            NodeKind::Foreach { variable, list, body, continue_block } => {
                self.collect_actions_for_range(variable, range, false, actions, extract_var_seen);
                self.collect_actions_for_range(list, range, false, actions, extract_var_seen);
                self.collect_actions_for_range(body, range, true, actions, extract_var_seen);
                if let Some(cb) = continue_block {
                    self.collect_actions_for_range(cb, range, false, actions, extract_var_seen);
                }
            }
            NodeKind::While { condition, body, .. } => {
                self.collect_actions_for_range(condition, range, false, actions, extract_var_seen);
                self.collect_actions_for_range(
                    body,
                    range,
                    true, // loop body is a control-flow block
                    actions,
                    extract_var_seen,
                );
            }
            NodeKind::MethodCall { object, args, .. } => {
                self.collect_actions_for_range(object, range, false, actions, extract_var_seen);
                for arg in args {
                    self.collect_actions_for_range(arg, range, false, actions, extract_var_seen);
                }
            }
            NodeKind::Subroutine { body, prototype, signature, .. } => {
                self.collect_actions_for_range(
                    body,
                    range,
                    true, // subroutine body block is not a standalone block
                    actions,
                    extract_var_seen,
                );
                if let Some(proto) = prototype {
                    self.collect_actions_for_range(proto, range, false, actions, extract_var_seen);
                }
                if let Some(sig) = signature {
                    self.collect_actions_for_range(sig, range, false, actions, extract_var_seen);
                }
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
                title: format!("Add missing pragmas ({})", pragmas.join(", ")),
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

#[cfg(test)]
mod extract_variable_tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_tdd_support::must;

    #[test]
    fn test_extract_hash_access_to_variable() {
        // Use assignment so hash access is in the RHS, not a print argument
        let source = "my $x = $hash{$key};";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        // Select the range covering $hash{$key} (bytes 8..19)
        let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 19));

        let extract_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("Extract")).collect();

        assert!(
            !extract_actions.is_empty(),
            "Expected an Extract action for hash access, got: {:?}",
            actions.iter().map(|a| &a.title).collect::<Vec<_>>()
        );

        // Verify the action produces a declaration with `my $val`
        let action = &extract_actions[0];
        let decl_edit = &action.edit.changes[0];
        assert!(
            decl_edit.new_text.contains("my $val"),
            "Expected variable name '$val' for hash access, got: {}",
            decl_edit.new_text
        );
    }

    #[test]
    fn test_extract_method_call_to_variable() {
        let source = "print $obj->method();";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        // Select the range covering $obj->method()
        let actions = provider.get_enhanced_refactoring_actions(&ast, (6, 20));

        let extract_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("Extract")).collect();

        assert!(
            !extract_actions.is_empty(),
            "Expected an Extract action for method call, got: {:?}",
            actions.iter().map(|a| &a.title).collect::<Vec<_>>()
        );

        // Verify the action produces a declaration with `my $result`
        let action = &extract_actions[0];
        let decl_edit = &action.edit.changes[0];
        assert!(
            decl_edit.new_text.contains("my $result"),
            "Expected variable name '$result' for method call, got: {}",
            decl_edit.new_text
        );

        // Verify the replacement edit uses $result
        let replace_edit = &action.edit.changes[1];
        assert!(
            replace_edit.new_text.contains("$result"),
            "Expected replacement with '$result', got: {}",
            replace_edit.new_text
        );
    }

    #[test]
    fn test_extract_method_call_new_suggests_instance() {
        let source = "my $x = Foo->new();";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 18));

        let extract_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("Extract")).collect();

        assert!(
            !extract_actions.is_empty(),
            "Expected an Extract action for constructor call, got: {:?}",
            actions.iter().map(|a| &a.title).collect::<Vec<_>>()
        );

        // Constructor call ->new() should suggest $instance
        let action = &extract_actions[0];
        let decl_edit = &action.edit.changes[0];
        assert!(
            decl_edit.new_text.contains("my $instance"),
            "Expected variable name '$instance' for ->new(), got: {}",
            decl_edit.new_text
        );
    }

    #[test]
    fn test_extract_variable_edit_structure() {
        let source = "my $x = $obj->get();";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 19));

        let extract_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("Extract")).collect();

        assert!(!extract_actions.is_empty(), "Expected at least one extract action");

        let action = &extract_actions[0];
        assert_eq!(action.edit.changes.len(), 2, "Expected exactly 2 edits (insert + replace)");

        // First edit: insertion of variable declaration
        let insert_edit = &action.edit.changes[0];
        assert!(
            insert_edit.new_text.starts_with("my $"),
            "First edit should be a variable declaration"
        );
        assert!(insert_edit.new_text.ends_with(";\n"), "Declaration should end with semicolon");

        // Second edit: replacement of expression with variable reference
        let replace_edit = &action.edit.changes[1];
        assert!(
            replace_edit.new_text.starts_with('$'),
            "Second edit should be a variable reference"
        );
    }
}
