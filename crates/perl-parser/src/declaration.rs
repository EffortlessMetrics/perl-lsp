//! Declaration Provider for LSP
//!
//! Provides go-to-declaration functionality for finding where symbols are declared.
//! Supports LocationLink for enhanced client experience.

use crate::ast::{Node, NodeKind};
use std::collections::HashMap;
use std::sync::Arc;

/// Provider for finding declarations
pub struct DeclarationProvider {
    ast: Arc<Node>,
    content: String,
    document_uri: String,
    parent_map: HashMap<*const Node, *const Node>,
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
    pub fn new(ast: Arc<Node>, content: String, document_uri: String) -> Self {
        let mut parent_map = HashMap::new();
        Self::build_parent_map(&ast, &mut parent_map, None);
        
        Self { 
            ast, 
            content,
            document_uri,
            parent_map,
        }
    }
    
    /// Build a parent map for efficient scope walking
    fn build_parent_map(node: &Node, map: &mut HashMap<*const Node, *const Node>, parent: Option<*const Node>) {
        if let Some(p) = parent {
            map.insert(node as *const _, p);
        }
        
        for child in Self::get_children_static(node) {
            Self::build_parent_map(child, map, Some(node as *const _));
        }
    }

    /// Find the declaration of the symbol at the given position
    pub fn find_declaration(&self, offset: usize) -> Option<Vec<LocationLink>> {
        // Find the node at the cursor position
        let node = self.find_node_at_offset(&self.ast, offset)?;
        
        // Check what kind of node we're on
        match &node.kind {
            NodeKind::Variable { name, .. } => self.find_variable_declaration(node, name),
            NodeKind::FunctionCall { name, .. } => self.find_subroutine_declaration(node, name),
            NodeKind::MethodCall { method, object, .. } => self.find_method_declaration(node, method, object),
            NodeKind::Identifier { name } => self.find_identifier_declaration(node, name),
            _ => None,
        }
    }

    /// Find variable declaration using scope-aware lookup
    fn find_variable_declaration(&self, usage: &Node, var_name: &str) -> Option<Vec<LocationLink>> {
        // Walk upwards through scopes to find the nearest declaration
        let mut current_ptr: *const Node = usage as *const _;
        
        while let Some(&parent_ptr) = self.parent_map.get(&current_ptr) {
            let parent = unsafe { &*parent_ptr };
            
            // Check siblings before this node in the current scope
            for child in self.get_children(parent) {
                // Stop when we reach or pass the usage node
                if child.location.start >= usage.location.start {
                    break;
                }
                
                // Check if this is a variable declaration matching our name
                if let NodeKind::VariableDeclaration { variable, .. } = &child.kind {
                    if let NodeKind::Variable { name, .. } = &variable.kind {
                        if name == var_name {
                            return Some(vec![LocationLink {
                                origin_selection_range: (usage.location.start, usage.location.end),
                                target_uri: self.document_uri.clone(),
                                target_range: (child.location.start, child.location.end),
                                target_selection_range: (variable.location.start, variable.location.end),
                            }]);
                        }
                    }
                }
                
                // Also check variable list declarations
                if let NodeKind::VariableListDeclaration { variables, .. } = &child.kind {
                    for var in variables {
                        if let NodeKind::Variable { name, .. } = &var.kind {
                            if name == var_name {
                                return Some(vec![LocationLink {
                                    origin_selection_range: (usage.location.start, usage.location.end),
                                    target_uri: self.document_uri.clone(),
                                    target_range: (child.location.start, child.location.end),
                                    target_selection_range: (var.location.start, var.location.end),
                                }]);
                            }
                        }
                    }
                }
            }
            
            current_ptr = parent_ptr;
        }
        
        None
    }

    /// Find subroutine declaration
    fn find_subroutine_declaration(&self, node: &Node, func_name: &str) -> Option<Vec<LocationLink>> {
        // First check current package context
        let current_package = self.find_current_package(node);
        
        // Search for subroutines, preferring same package
        let mut declarations = Vec::new();
        self.collect_subroutine_declarations(&self.ast, func_name, &mut declarations);
        
        // If we have a current package, prefer subs in the same package
        if let Some(pkg_name) = current_package {
            if let Some(decl) = declarations.iter().find(|d| {
                self.find_current_package(d) == Some(pkg_name.clone())
            }) {
                return Some(vec![self.create_location_link(node, decl, self.get_subroutine_name_range(decl))]);
            }
        }
        
        // Otherwise return the first match
        if let Some(decl) = declarations.first() {
            return Some(vec![self.create_location_link(node, decl, self.get_subroutine_name_range(decl))]);
        }
        
        None
    }
    
    /// Find method declaration with package resolution
    fn find_method_declaration(&self, node: &Node, method_name: &str, object: &Node) -> Option<Vec<LocationLink>> {
        // Try to determine the package from the object
        let package_name = match &object.kind {
            NodeKind::Identifier { name } if name.chars().next()?.is_uppercase() => {
                // Likely a package name (e.g., Foo->method)
                Some(name.clone())
            }
            _ => None,
        };
        
        if let Some(pkg) = package_name {
            // Look for the method in the specific package
            let mut declarations = Vec::new();
            self.collect_subroutine_declarations(&self.ast, method_name, &mut declarations);
            
            if let Some(decl) = declarations.iter().find(|d| {
                self.find_current_package(d) == Some(pkg.clone())
            }) {
                return Some(vec![self.create_location_link(node, decl, self.get_subroutine_name_range(decl))]);
            }
        }
        
        // Fall back to any subroutine with this name
        self.find_subroutine_declaration(node, method_name)
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
            return Some(vec![self.create_location_link(node, pkg, self.get_package_name_range(pkg))]);
        }
        
        // Try to find as constant (supporting multiple forms)
        let constants = self.find_constant_declarations(&self.ast, name);
        if let Some(const_decl) = constants.first() {
            return Some(vec![self.create_location_link(node, const_decl, self.get_constant_name_range(const_decl))]);
        }
        
        None
    }
    
    /// Find the current package context for a node
    fn find_current_package(&self, node: &Node) -> Option<String> {
        let mut current_ptr: *const Node = node as *const _;
        
        while let Some(&parent_ptr) = self.parent_map.get(&current_ptr) {
            let parent = unsafe { &*parent_ptr };
            
            // Check siblings before this node for package declarations
            for child in self.get_children(parent) {
                if child.location.start >= node.location.start {
                    break;
                }
                
                if let NodeKind::Package { name, .. } = &child.kind {
                    return Some(name.clone());
                }
            }
            
            current_ptr = parent_ptr;
        }
        
        None
    }
    
    /// Create a location link
    fn create_location_link(&self, origin: &Node, target: &Node, name_range: (usize, usize)) -> LocationLink {
        LocationLink {
            origin_selection_range: (origin.location.start, origin.location.end),
            target_uri: self.document_uri.clone(),
            target_range: (target.location.start, target.location.end),
            target_selection_range: name_range,
        }
    }

    // Helper methods

    fn find_node_at_offset<'a>(&'a self, node: &'a Node, offset: usize) -> Option<&'a Node> {
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

    fn find_package_declarations<'a>(&'a self, node: &'a Node, pkg_name: &str) -> Vec<&'a Node> {
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

    fn find_constant_declarations<'a>(&'a self, node: &'a Node, const_name: &str) -> Vec<&'a Node> {
        let mut constants = Vec::new();
        self.collect_constant_declarations(node, const_name, &mut constants);
        constants
    }

    fn collect_constant_declarations<'a>(&'a self, node: &'a Node, const_name: &str, constants: &mut Vec<&'a Node>) {
        // Handle multiple forms of constant declarations
        if let NodeKind::Use { module, args } = &node.kind {
            if module == "constant" {
                // Form 1: use constant FOO => 42;
                if !args.is_empty() && args[0] == const_name {
                    constants.push(node);
                }
                
                // Form 2: use constant { FOO => 1, BAR => 2 };
                // This would need more complex parsing of the args
                // TODO: Add support for hash form
                
                // Form 3: use constant qw(FOO BAR);
                // Check if const_name is in the qw list
                for arg in args {
                    if arg == const_name {
                        constants.push(node);
                        break;
                    }
                }
            }
        }
        
        for child in self.get_children(node) {
            self.collect_constant_declarations(child, const_name, constants);
        }
    }

    fn get_subroutine_name_range(&self, decl: &Node) -> (usize, usize) {
        // TODO: Store name spans in AST to avoid this heuristic
        let text = self.get_node_text(decl);
        if text.starts_with("sub ") {
            let name_start = decl.location.start + 4;
            let rest = &self.content[name_start..decl.location.end];
            let name_len = rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .count();
            return (name_start, name_start + name_len);
        }
        (decl.location.start, decl.location.end)
    }

    fn get_package_name_range(&self, decl: &Node) -> (usize, usize) {
        // TODO: Store name spans in AST to avoid this heuristic
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
    
    fn get_constant_name_range(&self, decl: &Node) -> (usize, usize) {
        // For now, return the whole range
        // TODO: Parse constant name position properly
        (decl.location.start, decl.location.end)
    }

    fn get_children<'a>(&self, node: &'a Node) -> Vec<&'a Node> {
        Self::get_children_static(node)
    }
    
    fn get_children_static(node: &Node) -> Vec<&Node> {
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
                vec![variable.as_ref(), list.as_ref(), body.as_ref()]
            }
            _ => vec![],
        }
    }

    fn get_node_text(&self, node: &Node) -> String {
        self.content[node.location.start..node.location.end].to_string()
    }
}