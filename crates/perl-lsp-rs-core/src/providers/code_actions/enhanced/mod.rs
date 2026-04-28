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

use super::types::CodeAction;
use perl_parser_core::ast::{Node, NodeKind};
use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

mod error_checking;
mod extract_subroutine;
mod extract_variable;
mod helpers;
mod import_management;
mod loop_conversion;
mod postfix;
mod signature_actions;

use helpers::Helpers;

static UTF8_PRAGMA_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*use\s+utf8\b").ok());
static OPEN_UTF8_PRAGMA_RE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?mi)^\s*use\s+open\b[^\n;]*:(?:utf8|encoding\s*\(\s*utf-?8\s*\))").ok()
});

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
        let normalized_range = self.normalize_range_for_refactors(range);
        // Track (stmt_start, var_name) pairs already emitted to prevent duplicate
        // extract-variable actions when both a parent and child node overlap the range.
        let mut extract_var_seen: HashSet<(usize, String)> = HashSet::new();

        // Find all nodes that overlap the range and collect actions
        self.collect_actions_for_range(
            ast,
            normalized_range,
            false,
            &mut actions,
            &mut extract_var_seen,
        );

        // Signature refactoring: collect add-parameter actions for any subroutine
        // node whose span overlaps the requested range.
        self.collect_signature_actions(ast, ast, normalized_range, &mut actions);

        // Global actions (not node-specific)
        actions.extend(self.get_global_refactorings(ast));

        actions
    }

    /// Normalize a selected byte range so trailing statement punctuation does not
    /// block expression-oriented refactor actions.
    fn normalize_range_for_refactors(&self, range: (usize, usize)) -> (usize, usize) {
        if self.source.is_empty() {
            return (0, 0);
        }

        let start = range.0.min(self.source.len());
        let mut end = range.1.min(self.source.len());

        if start >= end {
            return (start, end);
        }

        while end > start {
            // Use .get(..end) to avoid panicking on a non-char-boundary `end` value
            // that a stale or externally-sourced byte range might supply.
            let Some(ch) = self.source.get(..end).and_then(|s| s.chars().next_back()) else {
                // `end` is mid-char — snap to the nearest lower char boundary by
                // decrementing one byte at a time until we land on a boundary.
                end -= 1;
                while end > start && !self.source.is_char_boundary(end) {
                    end -= 1;
                }
                continue;
            };

            if ch.is_whitespace() || ch == ';' {
                end -= ch.len_utf8();
            } else {
                break;
            }
        }

        (start, end.max(start))
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
        use super::types::{CodeAction, CodeActionEdit, CodeActionKind};
        use crate::providers::rename::TextEdit;
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

        // Add UTF-8 pragmas if missing
        let has_utf8 = UTF8_PRAGMA_RE.as_ref().is_some_and(|re| re.is_match(&self.source));
        let has_open_utf8 =
            OPEN_UTF8_PRAGMA_RE.as_ref().is_some_and(|re| re.is_match(&self.source));
        if helpers.has_non_ascii_content() && (!has_utf8 || !has_open_utf8) {
            let insert_pos = helpers.find_pragma_insert_position();
            let mut missing_pragmas = Vec::new();
            if !has_utf8 {
                missing_pragmas.push("use utf8;");
            }
            if !has_open_utf8 {
                missing_pragmas.push("use open qw(:std :utf8);");
            }

            actions.push(CodeAction {
                title: "Add UTF-8 support".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: Vec::new(),
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: insert_pos, end: insert_pos },
                        new_text: format!("{}\n", missing_pragmas.join("\n")),
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
    use perl_tdd_support::{must, must_some};

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
    fn test_utf8_action_adds_open_when_utf8_already_present() {
        let source = "use utf8;\nmy $msg = \"café\";\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_global_refactorings(&ast);
        let utf8_action = must_some(actions.iter().find(|a| a.title == "Add UTF-8 support"));

        assert_eq!(
            utf8_action.edit.changes[0].new_text, "use open qw(:std :utf8);\n",
            "Should only add missing open pragma when use utf8 already exists"
        );
    }

    #[test]
    fn test_utf8_action_ignores_comment_mentions_of_pragma() {
        let source = "# use utf8;\nmy $msg = \"café\";\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_global_refactorings(&ast);
        let utf8_action = must_some(actions.iter().find(|a| a.title == "Add UTF-8 support"));

        assert_eq!(
            utf8_action.edit.changes[0].new_text, "use utf8;\nuse open qw(:std :utf8);\n",
            "Comments should not suppress UTF-8 pragma suggestions"
        );
    }

    #[test]
    fn test_utf8_action_adds_utf8_when_open_already_present() {
        // Inverse regression: only `use open :utf8` is present, should only add `use utf8;`.
        let source = "use open qw(:std :utf8);\nmy $msg = \"café\";\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_global_refactorings(&ast);
        let utf8_action = must_some(actions.iter().find(|a| a.title == "Add UTF-8 support"));

        assert_eq!(
            utf8_action.edit.changes[0].new_text, "use utf8;\n",
            "Should only add missing utf8 pragma when use open :utf8 already exists"
        );
    }

    #[test]
    fn test_utf8_action_suppressed_when_both_pragmas_present() {
        // Both pragmas already present — no UTF-8 action should be generated.
        let source = "use utf8;\nuse open qw(:std :utf8);\nmy $msg = \"café\";\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_global_refactorings(&ast);

        assert!(
            !actions.iter().any(|a| a.title == "Add UTF-8 support"),
            "Should not suggest UTF-8 pragmas when both are already present"
        );
    }

    #[test]
    fn test_utf8_action_suppressed_for_ascii_only_source() {
        // No non-ASCII content — no UTF-8 action regardless of pragma presence.
        let source = "my $msg = \"hello\";\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_global_refactorings(&ast);

        assert!(
            !actions.iter().any(|a| a.title == "Add UTF-8 support"),
            "Should not suggest UTF-8 pragmas for ASCII-only source"
        );
    }

    #[test]
    fn test_utf8_action_recognizes_encoding_utf8_variant() {
        // `use open ... :encoding(UTF-8)` must also count as open-utf8 pragma present.
        let source = "use utf8;\nuse open IO => ':encoding(UTF-8)';\nmy $msg = \"café\";\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_global_refactorings(&ast);

        assert!(
            !actions.iter().any(|a| a.title == "Add UTF-8 support"),
            "encoding(UTF-8) variant should be recognized as the open :utf8 pragma"
        );
    }

    #[test]
    fn test_utf8_action_recognizes_indented_pragma() {
        // Leading whitespace on the pragma line should still be matched (anchored to ^\s*).
        let source = "    use utf8;\nmy $msg = \"café\";\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_global_refactorings(&ast);
        let utf8_action = must_some(actions.iter().find(|a| a.title == "Add UTF-8 support"));

        assert_eq!(
            utf8_action.edit.changes[0].new_text, "use open qw(:std :utf8);\n",
            "Indented 'use utf8;' should still count as present"
        );
    }

    #[test]
    fn test_utf8_action_does_not_match_utf8mode_lookalike() {
        // `use utf8mode` (hypothetical) is not `use utf8` — the \b word boundary must prevent a match.
        let source = "use utf8mode;\nmy $msg = \"café\";\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_global_refactorings(&ast);
        let utf8_action = must_some(actions.iter().find(|a| a.title == "Add UTF-8 support"));

        assert_eq!(
            utf8_action.edit.changes[0].new_text, "use utf8;\nuse open qw(:std :utf8);\n",
            "`use utf8mode;` should not be treated as the utf8 pragma"
        );
    }

    #[test]
    fn test_utf8_action_recognizes_string_after_pragma_line() {
        // Comment on same line after pragma should still match.
        let source = "use utf8; # enable unicode\nmy $msg = \"café\";\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_global_refactorings(&ast);
        let utf8_action = must_some(actions.iter().find(|a| a.title == "Add UTF-8 support"));

        assert_eq!(
            utf8_action.edit.changes[0].new_text, "use open qw(:std :utf8);\n",
            "Trailing same-line comment should not hide pragma"
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

    #[test]
    fn test_extract_variable_with_selection_including_semicolon() {
        let source = "my $x = length($string);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        // Range includes trailing ';' and newline, as editors often do.
        let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 24));

        assert!(
            actions.iter().any(|a| a.title.contains("Extract")),
            "Expected extract action even when selection includes trailing punctuation"
        );
    }

    /// Line-based selection (cursor-to-end-of-line) extends to the `\n` byte.
    /// Normalization must trim both the semicolon AND the trailing newline so
    /// the expression's node.end (>= range.1) comparison still succeeds.
    #[test]
    fn test_extract_variable_with_line_selection_including_newline() {
        let source = "my $x = length($string);\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        // Range (8, 25) covers `length($string);\n` — the full line tail that
        // triple-click or "select to EOL" keybinds produce.
        let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 25));

        assert!(
            actions.iter().any(|a| a.title.contains("Extract")),
            "Expected extract action when selection includes trailing `;` and newline"
        );
    }

    /// Windows editors emit CRLF. Both '\r' and '\n' are `is_whitespace()` so
    /// normalization should trim them both.
    #[test]
    fn test_extract_variable_with_crlf_line_ending() {
        let source = "my $x = length($string);\r\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        // Range (8, 26) covers `length($string);\r\n`.
        let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 26));

        assert!(
            actions.iter().any(|a| a.title.contains("Extract")),
            "Expected extract action when selection ends with CRLF"
        );
    }

    /// A selection consisting entirely of whitespace/semicolons normalizes to
    /// an empty range. This must not panic, hang, or return spurious actions.
    #[test]
    fn test_normalize_all_trimmable_does_not_panic() {
        let source = "my $x = 42;   \n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        // Range (10, 15) covers `;   \n` — only trimmable bytes.
        let actions = provider.get_enhanced_refactoring_actions(&ast, (10, 15));

        // No assertion on action presence — important thing is no panic and
        // no infinite loop in the trim-while.
        let _ = actions;
    }

    /// An out-of-bounds `range.1` past `source.len()` (e.g. stale editor
    /// range after a truncation) must be clamped, not cause index panic.
    #[test]
    fn test_normalize_range_clamps_out_of_bounds_end() {
        let source = "my $x = length($string);";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        // range.1 = 9999 is well past source.len() = 24.
        let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 9999));

        assert!(
            actions.iter().any(|a| a.title.contains("Extract")),
            "Expected extract action when range.1 exceeds source length"
        );
    }

    /// Regression guard: the normalizer must not panic when a multibyte UTF-8
    /// character borders the trim boundary. `source[..end]` splits at a byte
    /// offset so the while-loop must only decrement by `len_utf8` of the last
    /// char — which the current implementation does via `chars().next_back()`.
    #[test]
    fn test_normalize_range_respects_multibyte_boundary() {
        // "π" is 2 bytes (0xCF 0x80). Place it just before the trimmable tail.
        let source = "my $x = \"π\";\n";
        let provider = EnhancedCodeActionsProvider::new(source.to_string());

        // Full source len in bytes (Rust &str indexing is byte-based).
        let len = source.len();
        // Just verify normalization completes and returns a start <= end
        // range within bounds — no panic on UTF-8 boundary.
        let normalized = provider.normalize_range_for_refactors((0, len));
        assert!(normalized.0 <= normalized.1);
        assert!(normalized.1 <= len);
    }

    /// An externally-supplied `end` that bisects a multibyte UTF-8 character must
    /// not cause a panic. The normalizer must snap to the nearest lower char boundary.
    #[test]
    fn test_normalize_range_mid_char_boundary_does_not_panic() {
        // "π" is 2 bytes (0xCF 0x80). "my $x = " is 8 bytes, then "π" occupies
        // bytes 8..10. Passing end=9 bisects the π character.
        let source = "my $x = π;";
        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        // end=9 is mid-char (π spans bytes 8 and 9). Must not panic.
        let normalized = provider.normalize_range_for_refactors((0, 9));
        assert!(normalized.0 <= normalized.1);
        assert!(source.is_char_boundary(normalized.1), "result end must be a valid char boundary");
    }

    /// An empty source must not panic when any range is passed.
    #[test]
    fn test_normalize_range_empty_source() {
        let provider = EnhancedCodeActionsProvider::new(String::new());
        let normalized = provider.normalize_range_for_refactors((5, 10));
        assert_eq!(normalized, (0, 0));
    }

    /// An inverted range (start > end) must be returned as-is without trimming
    /// or panicking — downstream `collect_actions_for_range` already treats
    /// such ranges as out-of-overlap.
    #[test]
    fn test_normalize_range_inverted_is_inert() {
        let source = "my $x = 42;";
        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        // start > end
        let normalized = provider.normalize_range_for_refactors((8, 3));
        assert_eq!(normalized, (8, 3));
    }
}
