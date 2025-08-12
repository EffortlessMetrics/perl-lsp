//! Enhanced code actions with additional refactorings
//!
//! This module extends the base code actions with more sophisticated refactorings.

use crate::ast::{Node, NodeKind, SourceLocation};
use crate::rename::TextEdit;
use crate::code_actions::{CodeAction, CodeActionKind, CodeActionEdit};
use std::collections::HashSet;

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
    pub fn get_enhanced_refactoring_actions(&self, ast: &Node, range: (usize, usize)) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        
        // Find all nodes that overlap the range and collect actions
        self.collect_actions_for_range(ast, range, &mut actions);
        
        // Global actions (not node-specific)
        actions.extend(self.get_global_refactorings(ast));
        
        actions
    }
    
    /// Recursively collect actions for all nodes in range
    fn collect_actions_for_range(&self, node: &Node, range: (usize, usize), actions: &mut Vec<CodeAction>) {
        // Check if this node overlaps the range
        if node.location.start <= range.1 && node.location.end >= range.0 {
            // Extract variable (enhanced version)
            if self.is_extractable_expression(node) {
                actions.push(self.create_extract_variable_action(node));
            }
            
            // Convert old-style loops
            if let Some(action) = self.convert_loop_style(node) {
                actions.push(action);
            }
            
            // Add error checking
            if let Some(action) = self.add_error_checking(node) {
                actions.push(action);
            }
            
            // Convert to postfix
            if let Some(action) = self.convert_to_postfix(node) {
                actions.push(action);
            }
            
            // Extract subroutine
            if self.is_extractable_block(node) {
                actions.push(self.create_extract_subroutine_action(node));
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
    
    /// Create extract variable action with smart naming
    fn create_extract_variable_action(&self, node: &Node) -> CodeAction {
        let expr_text = &self.source[node.location.start..node.location.end];
        let var_name = self.suggest_variable_name(node);
        
        // Find the best insertion point
        let stmt_start = self.find_statement_start(node.location.start);
        let indent = self.get_indent_at(stmt_start);
        
        CodeAction {
            title: format!("Extract '{}' to variable", self.truncate_expr(expr_text, 30)),
            kind: CodeActionKind::RefactorExtract,
            diagnostics: Vec::new(),
            edit: CodeActionEdit {
                changes: vec![
                    // Insert variable declaration
                    TextEdit {
                        location: SourceLocation { start: stmt_start, end: stmt_start },
                        new_text: format!("{}my ${} = {};\n", indent, var_name, expr_text),
                    },
                    // Replace expression with variable
                    TextEdit {
                        location: node.location,
                        new_text: format!("${}", var_name),
                    },
                ],
            },
            is_preferred: false,
        }
    }
    
    /// Create extract subroutine action
    fn create_extract_subroutine_action(&self, node: &Node) -> CodeAction {
        let body_text = &self.source[node.location.start..node.location.end];
        let sub_name = self.suggest_subroutine_name(node);
        let params = self.detect_parameters(node);
        let returns = self.detect_return_values(node);
        
        // Generate function signature
        let signature = if params.is_empty() {
            format!("sub {} {{\n", sub_name)
        } else {
            format!("sub {} {{\n    my ({}) = @_;\n", sub_name, params.join(", "))
        };
        
        // Find insertion position (before current sub or at end)
        let insert_pos = self.find_subroutine_insert_position(node.location.start);
        
        // Generate function call
        let call = if returns.is_empty() {
            format!("{}({});", sub_name, params.join(", "))
        } else {
            format!("my {} = {}({});", returns.join(", "), sub_name, params.join(", "))
        };
        
        CodeAction {
            title: "Extract to subroutine".to_string(),
            kind: CodeActionKind::RefactorExtract,
            diagnostics: Vec::new(),
            edit: CodeActionEdit {
                changes: vec![
                    // Insert function definition
                    TextEdit {
                        location: SourceLocation { start: insert_pos, end: insert_pos },
                        new_text: format!("{}{}\n}}\n\n", signature, body_text),
                    },
                    // Replace block with function call
                    TextEdit {
                        location: node.location,
                        new_text: call,
                    },
                ],
            },
            is_preferred: false,
        }
    }
    
    /// Convert old-style for loops to modern foreach
    fn convert_loop_style(&self, node: &Node) -> Option<CodeAction> {
        if let NodeKind::For { init, condition, update, body, .. } = &node.kind {
            // Check if it's a C-style for loop that can be converted
            if let Some(converted) = self.try_convert_c_style_loop(init, condition, update, body) {
                return Some(CodeAction {
                    title: "Convert to foreach loop".to_string(),
                    kind: CodeActionKind::RefactorRewrite,
                    diagnostics: Vec::new(),
                    edit: CodeActionEdit {
                        changes: vec![TextEdit {
                            location: node.location,
                            new_text: converted,
                        }],
                    },
                    is_preferred: false,
                });
            }
        }
        
        // Check for foreach that could be improved  
        if let NodeKind::Foreach { variable, list, body } = &node.kind {
            // Check if using implicit $_
            if let NodeKind::Variable { name, sigil } = &variable.kind {
                if name == "_" && sigil == "$" {
                    let list_text = &self.source[list.location.start..list.location.end];
                    let body_text = &self.source[body.location.start..body.location.end];
                    
                    return Some(CodeAction {
                        title: "Use explicit loop variable instead of $_".to_string(),
                        kind: CodeActionKind::RefactorRewrite,
                        diagnostics: Vec::new(),
                        edit: CodeActionEdit {
                            changes: vec![TextEdit {
                                location: node.location,
                                new_text: format!("foreach my $item ({}) {}", list_text, body_text),
                            }],
                        },
                        is_preferred: false,
                    });
                }
            }
        }
        
        None
    }
    
    /// Add error checking to file operations
    fn add_error_checking(&self, node: &Node) -> Option<CodeAction> {
        if let NodeKind::FunctionCall { name, args: _ } = &node.kind {
            let func_name = name.as_str();
            
            // Check for file operations without error checking
            if matches!(func_name, "open" | "close" | "print" | "printf" | "write" | "read" | "seek" | "truncate") {
                // Check if already has error checking
                if !self.has_error_checking_nearby(node.location.end) {
                    let expr_text = &self.source[node.location.start..node.location.end];
                    
                    return Some(CodeAction {
                        title: format!("Add error checking to '{}'", func_name),
                        kind: CodeActionKind::RefactorRewrite,
                        diagnostics: Vec::new(),
                        edit: CodeActionEdit {
                            changes: vec![TextEdit {
                                location: node.location,
                                new_text: format!("{} or die \"Failed to {}: $!\"", expr_text, func_name),
                            }],
                        },
                        is_preferred: false,
                    });
                }
            }
        }
        
        None
    }
    
    /// Convert if statement to postfix form
    fn convert_to_postfix(&self, node: &Node) -> Option<CodeAction> {
        if let NodeKind::If { condition, then_branch, elsif_branches, else_branch } = &node.kind {
            // Only convert simple if statements with no elsif/else
            if elsif_branches.is_empty() && else_branch.is_none() {
                if let NodeKind::Block { statements } = &then_branch.kind {
                    if statements.len() == 1 {
                        let stmt = &statements[0];
                        let stmt_text = &self.source[stmt.location.start..stmt.location.end];
                        let cond_text = &self.source[condition.location.start..condition.location.end];
                        
                        // Check if statement is simple enough for postfix
                        if !stmt_text.contains('\n') && stmt_text.len() < 80 {
                            return Some(CodeAction {
                                title: "Convert to postfix if".to_string(),
                                kind: CodeActionKind::RefactorRewrite,
                                diagnostics: Vec::new(),
                                edit: CodeActionEdit {
                                    changes: vec![TextEdit {
                                        location: node.location,
                                        new_text: format!("{} if {}", stmt_text, cond_text),
                                    }],
                                },
                                is_preferred: false,
                            });
                        }
                    }
                }
            }
        }
        
        // Similarly for while, until
        // Note: Unless is not a separate node type in this AST
        
        None
    }
    
    /// Get global refactoring actions
    fn get_global_refactorings(&self, ast: &Node) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        
        // Add missing imports
        if let Some(action) = self.add_missing_imports(ast) {
            actions.push(action);
        }
        
        // Organize imports
        if let Some(action) = self.organize_imports(ast) {
            actions.push(action);
        }
        
        // Add pragmas
        actions.extend(self.add_recommended_pragmas(ast));
        
        actions
    }
    
    /// Add missing imports for undefined functions
    fn add_missing_imports(&self, ast: &Node) -> Option<CodeAction> {
        let undefined = self.find_undefined_functions(ast);
        if undefined.is_empty() {
            return None;
        }
        
        let mut imports = Vec::new();
        
        // Map common functions to their modules
        for func in &undefined {
            if let Some(module) = self.guess_module_for_function(func) {
                imports.push(format!("use {};", module));
            }
        }
        
        if imports.is_empty() {
            return None;
        }
        
        // Find insert position (after shebang and existing pragmas)
        let insert_pos = self.find_import_insert_position();
        
        Some(CodeAction {
            title: "Add missing imports".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: Vec::new(),
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: insert_pos, end: insert_pos },
                    new_text: format!("{}\n", imports.join("\n")),
                }],
            },
            is_preferred: false,
        })
    }
    
    /// Organize import statements
    fn organize_imports(&self, _ast: &Node) -> Option<CodeAction> {
        let imports = self.collect_imports();
        if imports.len() <= 1 {
            return None;
        }
        
        // Sort imports: pragmas first, then core, then CPAN, then local
        let organized = self.sort_imports(imports);
        
        // Find the range of existing imports
        if let Some((start, end)) = self.find_imports_range() {
            return Some(CodeAction {
                title: "Organize imports".to_string(),
                kind: CodeActionKind::SourceOrganizeImports,
                diagnostics: Vec::new(),
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start, end },
                        new_text: organized.join("\n") + "\n",
                    }],
                },
                is_preferred: false,
            });
        }
        
        None
    }
    
    /// Add recommended pragmas
    fn add_recommended_pragmas(&self, _ast: &Node) -> Vec<CodeAction> {
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
            
            let insert_pos = self.find_pragma_insert_position();
            
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
        if !self.source.contains("use utf8") && self.has_non_ascii_content() {
            let insert_pos = self.find_pragma_insert_position();
            
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
    
    // Helper methods
    
    /// Suggest a variable name based on the expression
    fn suggest_variable_name(&self, node: &Node) -> String {
        match &node.kind {
            NodeKind::FunctionCall { name, .. } => {
                let func_name = name.as_str();
                match func_name {
                    "length" | "size" => "len",
                    "split" => "parts",
                    "join" => "joined",
                    "sort" => "sorted",
                    "reverse" => "reversed",
                    "grep" | "filter" => "filtered",
                    "map" => "mapped",
                    _ => "result",
                }
            }
            NodeKind::Binary { op, .. } => {
                match op.as_str() {
                    "+" | "-" | "*" | "/" | "%" => "result",
                    "." | "x" => "str",
                    "&&" | "||" | "and" | "or" => "condition",
                    "==" | "!=" | "<" | ">" | "<=" | ">=" => "is_valid",
                    _ => "value",
                }
            }
            _ => "extracted",
        }.to_string()
    }
    
    /// Suggest a subroutine name
    fn suggest_subroutine_name(&self, _node: &Node) -> String {
        // Could analyze the code to suggest better names
        "process_data".to_string()
    }
    
    /// Detect parameters used in a block
    fn detect_parameters(&self, node: &Node) -> Vec<String> {
        let mut params = HashSet::new();
        self.collect_variables(node, &mut params);
        params.into_iter().collect()
    }
    
    /// Detect return values in a block
    fn detect_return_values(&self, _node: &Node) -> Vec<String> {
        // For now, return empty - could analyze return statements
        Vec::new()
    }
    
    /// Collect variables used in a node
    #[allow(clippy::only_used_in_recursion)]
    fn collect_variables(&self, node: &Node, vars: &mut HashSet<String>) {
        match &node.kind {
            NodeKind::Variable { name, .. } => {
                let var_name = name.as_str();
                vars.insert(var_name.to_string());
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.collect_variables(stmt, vars);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                self.collect_variables(left, vars);
                self.collect_variables(right, vars);
            }
            _ => {}
        }
    }
    
    /// Try to convert C-style for loop to foreach
    fn try_convert_c_style_loop(&self, init: &Option<Box<Node>>, condition: &Option<Box<Node>>, 
                                update: &Option<Box<Node>>, body: &Node) -> Option<String> {
        // Pattern: for (my $i = 0; $i < @array; $i++)
        // Convert to: foreach my $item (@array)
        
        // This is complex pattern matching - simplified version
        if init.is_some() && condition.is_some() && update.is_some() {
            let body_text = &self.source[body.location.start..body.location.end];
            // Check if it's array iteration pattern
            if self.source[init.as_ref().unwrap().location.start..init.as_ref().unwrap().location.end].contains("= 0") {
                return Some(format!("foreach my $item (@array) {}", body_text));
            }
        }
        
        None
    }
    
    /// Check if there's error checking nearby
    fn has_error_checking_nearby(&self, pos: usize) -> bool {
        // Check next 50 characters for "or", "||", "die", "warn"
        let check_text = &self.source[pos..std::cmp::min(pos + 50, self.source.len())];
        check_text.contains(" or ") || check_text.contains(" || ") || 
        check_text.contains("die") || check_text.contains("warn")
    }
    
    /// Find undefined functions in the AST
    fn find_undefined_functions(&self, _ast: &Node) -> Vec<String> {
        // This would require full semantic analysis
        // For now, return empty
        Vec::new()
    }
    
    /// Guess module for a function
    fn guess_module_for_function(&self, func: &str) -> Option<String> {
        match func {
            "dumper" => Some("Data::Dumper"),
            "encode" | "decode" => Some("Encode"),
            "basename" | "dirname" => Some("File::Basename"),
            "mkpath" | "rmtree" => Some("File::Path"),
            "slurp" => Some("File::Slurp"),
            "decode_json" | "encode_json" => Some("JSON"),
            _ => None,
        }.map(|s| s.to_string())
    }
    
    /// Check if content has non-ASCII characters
    fn has_non_ascii_content(&self) -> bool {
        self.source.chars().any(|c| c as u32 > 127)
    }
    
    /// Truncate expression for display
    fn truncate_expr(&self, expr: &str, max_len: usize) -> String {
        if expr.len() <= max_len {
            expr.to_string()
        } else {
            format!("{}...", &expr[..max_len-3])
        }
    }
    
    /// Get indentation at position
    fn get_indent_at(&self, pos: usize) -> String {
        let line_start = self.source[..pos]
            .rfind('\n')
            .map(|p| p + 1)
            .unwrap_or(0);
        
        let line = &self.source[line_start..];
        let mut indent = String::new();
        for ch in line.chars() {
            if ch == ' ' || ch == '\t' {
                indent.push(ch);
            } else {
                break;
            }
        }
        indent
    }
    
    /// Find statement start
    fn find_statement_start(&self, pos: usize) -> usize {
        let mut i = pos.saturating_sub(1);
        while i > 0 {
            if self.source.chars().nth(i) == Some(';') || self.source.chars().nth(i) == Some('\n') {
                return i + 1;
            }
            i = i.saturating_sub(1);
        }
        0
    }
    
    /// Find subroutine insertion position
    fn find_subroutine_insert_position(&self, current_pos: usize) -> usize {
        // Find the current subroutine
        let mut pos = current_pos;
        while pos > 0 {
            if self.source[pos.saturating_sub(4)..pos].starts_with("sub ") {
                // Found a sub, insert before it
                return pos.saturating_sub(4);
            }
            pos = pos.saturating_sub(1);
        }
        
        // No sub found, insert at end
        self.source.len()
    }
    
    /// Find pragma insertion position
    fn find_pragma_insert_position(&self) -> usize {
        // After shebang if present
        if self.source.starts_with("#!") {
            if let Some(pos) = self.source.find('\n') {
                return pos + 1;
            }
        }
        0
    }
    
    /// Find import insertion position
    fn find_import_insert_position(&self) -> usize {
        // After existing pragmas
        let mut pos = self.find_pragma_insert_position();
        
        for line in self.lines.iter() {
            if line.starts_with("use ") || line.starts_with("require ") {
                pos = self.source.find(line).unwrap_or(0) + line.len() + 1;
            } else if !line.is_empty() && !line.starts_with('#') {
                break;
            }
        }
        
        pos
    }
    
    /// Collect all import statements
    fn collect_imports(&self) -> Vec<String> {
        let mut imports = Vec::new();
        
        for line in &self.lines {
            let trimmed = line.trim();
            if trimmed.starts_with("use ") || trimmed.starts_with("require ") {
                imports.push(line.clone());
            }
        }
        
        imports
    }
    
    /// Sort imports by category
    fn sort_imports(&self, imports: Vec<String>) -> Vec<String> {
        let mut pragmas = Vec::new();
        let mut core = Vec::new();
        let mut cpan = Vec::new();
        let mut local = Vec::new();
        
        for import in imports {
            if import.contains("strict") || import.contains("warnings") || 
               import.contains("utf8") || import.contains("feature") {
                pragmas.push(import);
            } else if import.contains("::") {
                cpan.push(import);
            } else if import.starts_with("use lib") || import.contains("./") {
                local.push(import);
            } else {
                core.push(import);
            }
        }
        
        pragmas.sort();
        core.sort();
        cpan.sort();
        local.sort();
        
        let mut result = Vec::new();
        result.extend(pragmas);
        result.extend(core);
        result.extend(cpan);
        result.extend(local);
        
        result
    }
    
    /// Find the range of import statements
    fn find_imports_range(&self) -> Option<(usize, usize)> {
        let imports = self.collect_imports();
        if imports.is_empty() {
            return None;
        }
        
        let first = self.source.find(&imports[0])?;
        let last = self.source.find(&imports[imports.len() - 1])?;
        let last_end = last + imports[imports.len() - 1].len();
        
        Some((first, last_end))
    }
    
    /// Find node at range
    #[allow(dead_code)]
    #[allow(clippy::only_used_in_recursion)]
    fn find_node_at_range<'a>(&self, node: &'a Node, range: (usize, usize)) -> Option<&'a Node> {
        if node.location.start <= range.0 && node.location.end >= range.1 {
            match &node.kind {
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    for stmt in statements {
                        if let Some(result) = self.find_node_at_range(stmt, range) {
                            return Some(result);
                        }
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
    
    #[test]
    fn test_extract_variable() {
        let source = "my $x = length($string) + 10;";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();
        
        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 23)); // Select "length($string)"
        
        // Debug: print all actions
        for action in &actions {
            eprintln!("Action: {}", action.title);
        }
        
        assert!(!actions.is_empty(), "Expected at least one action");
        assert!(actions.iter().any(|a| a.title.contains("Extract")), 
            "Expected an Extract action, got: {:?}", 
            actions.iter().map(|a| &a.title).collect::<Vec<_>>());
    }
    
    #[test]
    fn test_add_error_checking() {
        let source = "open my $fh, '<', 'file.txt';";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();
        
        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 30));
        
        assert!(actions.iter().any(|a| a.title.contains("error checking")));
    }
    
    #[test]
    fn test_convert_to_postfix() {
        let source = "if ($debug) { print \"Debug\\n\"; }";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();
        
        let provider = EnhancedCodeActionsProvider::new(source.to_string());
        let actions = provider.get_enhanced_refactoring_actions(&ast, (0, source.len()));
        
        assert!(actions.iter().any(|a| a.title.contains("postfix")));
    }
}