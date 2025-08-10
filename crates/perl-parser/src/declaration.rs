//! Declaration Provider for LSP
//!
//! Provides go-to-declaration functionality for finding where symbols are declared.
//! Supports LocationLink for enhanced client experience.

use crate::ast::{Node, NodeKind};
use std::sync::Arc;

/// Provider for finding declarations
pub struct DeclarationProvider {
    ast: Arc<Node>,
    content: String,
}

/// Represents a location link from origin to target
#[derive(Debug, Clone)]
pub struct LocationLink {
    /// The range of the symbol being targeted at the origin
    pub origin_selection_range: (usize, usize),
    /// The target URI
    pub target_uri: String,
    /// The full range of the target declaration
    pub target_range: (usize, usize),
    /// The range to select in the target (e.g., just the name)
    pub target_selection_range: (usize, usize),
}

impl DeclarationProvider {
    pub fn new(ast: Arc<Node>, content: String) -> Self {
        Self { ast, content }
    }

    /// Find the declaration of the symbol at the given position
    pub fn find_declaration(&self, offset: usize) -> Option<Vec<LocationLink>> {
        // Find the node at the cursor position
        let node = self.find_node_at_offset(&self.ast, offset)?;
        
        // Check what kind of node we're on
        match &node.kind {
            NodeKind::Variable { name, .. } => self.find_variable_declaration(node, name),
            NodeKind::FunctionCall { name, .. } => self.find_subroutine_declaration(node, name),
            NodeKind::MethodCall { method, .. } => self.find_subroutine_declaration(node, method),
            NodeKind::Identifier { name } => self.find_identifier_declaration(node, name),
            _ => None,
        }
    }

    /// Find variable declaration (my, our, local, state)
    fn find_variable_declaration(&self, node: &Node, var_name: &str) -> Option<Vec<LocationLink>> {
        // Search for the closest enclosing scope with this variable declaration
        let declarations = self.find_variable_declarations(&self.ast, var_name);
        
        // Find the declaration that's before our usage and in scope
        for decl in declarations {
            if decl.location.start < node.location.start {
                return Some(vec![LocationLink {
                    origin_selection_range: (node.location.start, node.location.end),
                    target_uri: "current_file".to_string(),
                    target_range: (decl.location.start, decl.location.end),
                    target_selection_range: self.get_variable_name_range(decl),
                }]);
            }
        }
        
        None
    }

    /// Find subroutine declaration
    fn find_subroutine_declaration(&self, node: &Node, func_name: &str) -> Option<Vec<LocationLink>> {
        let declarations = self.find_subroutine_declarations(&self.ast, func_name);
        
        if let Some(decl) = declarations.first() {
            return Some(vec![LocationLink {
                origin_selection_range: (node.location.start, node.location.end),
                target_uri: "current_file".to_string(),
                target_range: (decl.location.start, decl.location.end),
                target_selection_range: self.get_subroutine_name_range(decl),
            }]);
        }
        
        None
    }

    /// Find declaration for an identifier
    fn find_identifier_declaration(&self, node: &Node, name: &str) -> Option<Vec<LocationLink>> {
        // Try to find as subroutine first
        if let Some(links) = self.find_subroutine_declaration(node, name) {
            return Some(links);
        }
        
        // Try to find as package
        let packages = self.find_package_declarations(&self.ast, name);
        if let Some(pkg) = packages.first() {
            return Some(vec![LocationLink {
                origin_selection_range: (node.location.start, node.location.end),
                target_uri: "current_file".to_string(),
                target_range: (pkg.location.start, pkg.location.end),
                target_selection_range: self.get_package_name_range(pkg),
            }]);
        }
        
        // Try to find as constant
        let constants = self.find_constant_declarations(&self.ast, name);
        if let Some(const_decl) = constants.first() {
            return Some(vec![LocationLink {
                origin_selection_range: (node.location.start, node.location.end),
                target_uri: "current_file".to_string(),
                target_range: (const_decl.location.start, const_decl.location.end),
                target_selection_range: (const_decl.location.start, const_decl.location.end),
            }]);
        }
        
        None
    }

    // Helper methods

    fn find_node_at_offset<'a>(&self, node: &'a Node, offset: usize) -> Option<&'a Node> {
        if offset >= node.location.start && offset <= node.location.end {
            // Check children first for more specific match
            for child in self.get_children(node) {
                if let Some(found) = self.find_node_at_offset(child, offset) {
                    return Some(found);
                }
            }
            return Some(node);
        }
        None
    }

    fn find_variable_declarations<'a>(&self, node: &'a Node, var_name: &str) -> Vec<&'a Node> {
        let mut declarations = Vec::new();
        self.collect_variable_declarations(node, var_name, &mut declarations);
        declarations
    }

    fn collect_variable_declarations<'a>(&'a self, node: &'a Node, var_name: &str, declarations: &mut Vec<&'a Node>) {
        match &node.kind {
            NodeKind::VariableDeclaration { variable, .. } => {
                if let NodeKind::Variable { name, .. } = &variable.kind {
                    if name == var_name {
                        declarations.push(node);
                    }
                }
            }
            NodeKind::VariableListDeclaration { variables, .. } => {
                for var in variables {
                    if let NodeKind::Variable { name, .. } = &var.kind {
                        if name == var_name {
                            declarations.push(node);
                        }
                    }
                }
            }
            _ => {}
        }
        
        for child in self.get_children(node) {
            self.collect_variable_declarations(child, var_name, declarations);
        }
    }

    fn find_subroutine_declarations<'a>(&self, node: &'a Node, sub_name: &str) -> Vec<&'a Node> {
        let mut subs = Vec::new();
        self.collect_subroutine_declarations(node, sub_name, &mut subs);
        subs
    }

    fn collect_subroutine_declarations<'a>(&'a self, node: &'a Node, sub_name: &str, subs: &mut Vec<&'a Node>) {
        if let NodeKind::Subroutine { name, .. } = &node.kind {
            if let Some(name_str) = name {
                if name_str == sub_name {
                    subs.push(node);
                }
            }
        }
        
        for child in self.get_children(node) {
            self.collect_subroutine_declarations(child, sub_name, subs);
        }
    }

    fn find_package_declarations<'a>(&self, node: &'a Node, pkg_name: &str) -> Vec<&'a Node> {
        let mut packages = Vec::new();
        self.collect_package_declarations(node, pkg_name, &mut packages);
        packages
    }

    fn collect_package_declarations<'a>(&'a self, node: &'a Node, pkg_name: &str, packages: &mut Vec<&'a Node>) {
        if let NodeKind::Package { name, .. } = &node.kind {
            if name == pkg_name {
                packages.push(node);
            }
        }
        
        for child in self.get_children(node) {
            self.collect_package_declarations(child, pkg_name, packages);
        }
    }

    fn find_constant_declarations<'a>(&self, node: &'a Node, const_name: &str) -> Vec<&'a Node> {
        let mut constants = Vec::new();
        self.collect_constant_declarations(node, const_name, &mut constants);
        constants
    }

    fn collect_constant_declarations<'a>(&'a self, node: &'a Node, const_name: &str, constants: &mut Vec<&'a Node>) {
        // Look for 'use constant FOO => value' pattern
        if let NodeKind::Use { module, args } = &node.kind {
            if module == "constant" && !args.is_empty() && args[0] == const_name {
                constants.push(node);
            }
        }
        
        for child in self.get_children(node) {
            self.collect_constant_declarations(child, const_name, constants);
        }
    }

    fn get_variable_name_range(&self, decl: &Node) -> (usize, usize) {
        if let NodeKind::VariableDeclaration { variable, .. } = &decl.kind {
            return (variable.location.start, variable.location.end);
        }
        (decl.location.start, decl.location.end)
    }

    fn get_subroutine_name_range(&self, decl: &Node) -> (usize, usize) {
        // The subroutine name is part of the declaration
        // For now, use a heuristic - skip "sub " prefix
        let text = self.get_node_text(decl);
        if text.starts_with("sub ") {
            let name_start = decl.location.start + 4;
            // Find the end of the name
            let rest = &self.content[name_start..decl.location.end];
            let name_len = rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .count();
            return (name_start, name_start + name_len);
        }
        (decl.location.start, decl.location.end)
    }

    fn get_package_name_range(&self, decl: &Node) -> (usize, usize) {
        // Skip "package " prefix
        let text = self.get_node_text(decl);
        if text.starts_with("package ") {
            let name_start = decl.location.start + 8;
            let rest = &self.content[name_start..decl.location.end];
            let name_len = rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
                .count();
            return (name_start, name_start + name_len);
        }
        (decl.location.start, decl.location.end)
    }

    fn get_children(&self, node: &Node) -> Vec<&Node> {
        match &node.kind {
            NodeKind::Program { statements } => statements.iter().collect(),
            NodeKind::Block { statements } => statements.iter().collect(),
            NodeKind::If { condition, then_branch, else_branch, .. } => {
                let mut children = vec![condition.as_ref(), then_branch.as_ref()];
                if let Some(else_b) = else_branch {
                    children.push(else_b.as_ref());
                }
                children
            }
            NodeKind::Binary { left, right, .. } => vec![left.as_ref(), right.as_ref()],
            NodeKind::Unary { operand, .. } => vec![operand.as_ref()],
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                let mut children = vec![variable.as_ref()];
                if let Some(init) = initializer {
                    children.push(init.as_ref());
                }
                children
            }
            NodeKind::Subroutine { params, body, .. } => {
                let mut children = vec![body.as_ref()];
                children.extend(params.iter());
                children
            }
            NodeKind::FunctionCall { args, .. } => {
                args.iter().collect()
            }
            NodeKind::MethodCall { object, args, .. } => {
                let mut children = vec![object.as_ref()];
                children.extend(args.iter());
                children
            }
            NodeKind::While { condition, body, .. } => {
                vec![condition.as_ref(), body.as_ref()]
            }
            NodeKind::For { init, condition, update, body, .. } => {
                let mut children = Vec::new();
                if let Some(i) = init {
                    children.push(i.as_ref());
                }
                if let Some(c) = condition {
                    children.push(c.as_ref());
                }
                if let Some(u) = update {
                    children.push(u.as_ref());
                }
                children.push(body.as_ref());
                children
            }
            NodeKind::Foreach { variable, list, body, .. } => {
                let mut children = vec![list.as_ref(), body.as_ref()];
                if let Some(v) = variable.as_ref() {
                    children.push(v);
                }
                children
            }
            _ => vec![],
        }
    }

    fn get_node_text(&self, node: &Node) -> String {
        self.content[node.location.start..node.location.end].to_string()
    }
}