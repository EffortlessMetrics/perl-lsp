//! Declaration Provider for LSP
//!
//! Provides go-to-declaration functionality for finding where symbols are declared.
//! Supports LocationLink for enhanced client experience.

use crate::ast::{Node, NodeKind};
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// Type alias for parent map using fast hash
pub type ParentMap = FxHashMap<*const Node, *const Node>;

/// Provider for finding declarations
pub struct DeclarationProvider<'a> {
    ast: Arc<Node>,
    content: String,
    document_uri: String,
    parent_map: Option<&'a ParentMap>,
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

impl<'a> DeclarationProvider<'a> {
    pub fn new(ast: Arc<Node>, content: String, document_uri: String) -> Self {
        Self { 
            ast, 
            content,
            document_uri,
            parent_map: None,
        }
    }
    
    pub fn with_parent_map(mut self, parent_map: &'a ParentMap) -> Self {
        // Assert parent map is not empty in debug builds
        #[cfg(debug_assertions)]
        {
            if parent_map.is_empty() {
                eprintln!("Warning: DeclarationProvider constructed with empty parent map");
            }
        }
        self.parent_map = Some(parent_map);
        self
    }
    
    /// Build a parent map for efficient scope walking
    pub fn build_parent_map(node: &Node, map: &mut ParentMap, parent: Option<*const Node>) {
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
        
        // Build temporary parent map if not provided (for testing)
        let temp_parent_map;
        let parent_map = if let Some(pm) = self.parent_map {
            pm
        } else {
            temp_parent_map = {
                let mut map = FxHashMap::default();
                Self::build_parent_map(&self.ast, &mut map, None);
                map
            };
            &temp_parent_map
        };
        
        while let Some(&parent_ptr) = parent_map.get(&current_ptr) {
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
            return Some(vec![self.create_location_link(node, const_decl, self.get_constant_name_range_for(const_decl, name))]);
        }
        
        None
    }
    
    /// Find the current package context for a node
    fn find_current_package(&self, node: &Node) -> Option<String> {
        let mut current_ptr: *const Node = node as *const _;
        
        // Build temporary parent map if not provided (for testing)
        let temp_parent_map;
        let parent_map = if let Some(pm) = self.parent_map {
            pm
        } else {
            temp_parent_map = {
                let mut map = FxHashMap::default();
                Self::build_parent_map(&self.ast, &mut map, None);
                map
            };
            &temp_parent_map
        };
        
        while let Some(&parent_ptr) = parent_map.get(&current_ptr) {
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

    fn find_node_at_offset<'b>(&'b self, node: &'b Node, offset: usize) -> Option<&'b Node> {
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

    fn collect_subroutine_declarations<'b>(&'b self, node: &'b Node, sub_name: &str, subs: &mut Vec<&'b Node>) {
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

    fn find_package_declarations<'b>(&'b self, node: &'b Node, pkg_name: &str) -> Vec<&'b Node> {
        let mut packages = Vec::new();
        self.collect_package_declarations(node, pkg_name, &mut packages);
        packages
    }

    fn collect_package_declarations<'b>(&'b self, node: &'b Node, pkg_name: &str, packages: &mut Vec<&'b Node>) {
        if let NodeKind::Package { name, .. } = &node.kind {
            if name == pkg_name {
                packages.push(node);
            }
        }
        
        for child in self.get_children(node) {
            self.collect_package_declarations(child, pkg_name, packages);
        }
    }

    fn find_constant_declarations<'b>(&'b self, node: &'b Node, const_name: &str) -> Vec<&'b Node> {
        let mut constants = Vec::new();
        self.collect_constant_declarations(node, const_name, &mut constants);
        constants
    }

    /// Strip leading -options from constant args
    fn strip_constant_options<'b>(&self, args: &'b [String]) -> &'b [String] {
        let mut i = 0;
        while i < args.len() && args[i].starts_with('-') {
            i += 1;
        }
        // Also skip a comma if present after options
        if i < args.len() && args[i] == "," {
            i += 1;
        }
        &args[i..]
    }

    fn collect_constant_declarations<'b>(&'b self, node: &'b Node, const_name: &str, constants: &mut Vec<&'b Node>) {
        if let NodeKind::Use { module, args } = &node.kind {
            if module == "constant" {
                // Strip leading options like -strict, -nonstrict, -force
                let stripped_args = self.strip_constant_options(args);
                
                // Form 1: FOO => ...
                if stripped_args.first().map(|s| s.as_str()) == Some(const_name) {
                    constants.push(node);
                    // keep scanning siblings too (there can be multiple `use constant`)
                }
                
                // Flattened args text once (cheap)
                let args_text = stripped_args.join(" ");
                
                // Form 2: { FOO => 1, BAR => 2 }
                if self.contains_name_in_hash(&args_text, const_name) {
                    constants.push(node);
                }
                
                // Form 3: qw(FOO BAR) / qw/FOO BAR/
                if self.contains_name_in_qw(&args_text, const_name) {
                    constants.push(node);
                }
            }
        }
        
        for child in self.get_children(node) {
            self.collect_constant_declarations(child, const_name, constants);
        }
    }
    
    /// Check if a byte is part of an ASCII identifier
    #[inline]
    fn is_ident_ascii(b: u8) -> bool {
        matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'_')
    }
    
    /// Iterate over all qw windows in the string
    fn for_each_qw_window<F>(&self, s: &str, mut f: F) -> bool 
    where
        F: FnMut(usize, usize) -> bool,
    {
        let mut i = 0;
        while let Some(pos) = s[i..].find("qw") {
            let q = i + pos;
            
            // Check word boundary before qw
            let left_ok = q == 0 || !Self::is_ident_ascii(s.as_bytes()[q.saturating_sub(1)]);
            if !left_ok { 
                i = q + 2; 
                continue; 
            }
            
            // Check word boundary after qw (ensure it's not qwerty)
            let right_idx = q + 2;
            if right_idx < s.len() && Self::is_ident_ascii(s.as_bytes()[right_idx]) {
                i = q + 2;
                continue;
            }
            
            let mut j = q + 2;
            let bytes = s.as_bytes();
            while j < bytes.len() && bytes[j].is_ascii_whitespace() { 
                j += 1; 
            }
            if j >= bytes.len() { 
                return false; 
            }
            
            let open = bytes[j] as char;
            let (open_delim, close_delim) = match open {
                '(' => ('(', ')'),
                '[' => ('[', ']'),
                '{' => ('{', '}'),
                '<' => ('<', '>'),
                _ if !open.is_alphanumeric() && !open.is_whitespace() => (open, open),
                _ => {
                    i = j + 1;
                    continue;
                }
            };
            
            if let Some(start_rel) = s[j..].find(open_delim) {
                let start = j + start_rel + 1;
                if let Some(end_rel) = s[start..].find(close_delim) {
                    let end = start + end_rel;
                    if f(start, end) {
                        return true;
                    }
                    i = end + 1;
                    continue;
                }
            }
            i = j + 1;
        }
        false
    }
    
    /// Iterate over all {...} pairs in the string
    fn for_each_brace_window<F>(&self, s: &str, mut f: F) -> bool
    where
        F: FnMut(usize, usize) -> bool,
    {
        let mut i = 0;
        while let Some(open) = s[i..].find('{') {
            let start = i + open + 1;
            if let Some(close_rel) = s[start..].find('}') {
                let end = start + close_rel;
                if f(start, end) {
                    return true;
                }
                i = end + 1;
            } else {
                break;
            }
        }
        false
    }
    
    fn contains_name_in_hash(&self, s: &str, name: &str) -> bool {
        // for { FOO => 1, BAR => 2 } form - check all {...} pairs
        self.for_each_brace_window(s, |start, end| {
            // only scan that slice
            self.find_word(&s[start..end], name).is_some()
        })
    }
    
    fn contains_name_in_qw(&self, s: &str, name: &str) -> bool {
        // looks for qw(...) / qw[...] / qw/.../ etc. with word boundaries
        self.for_each_qw_window(s, |start, end| {
            // tokens are whitespace separated
            s[start..end].split_whitespace().any(|tok| tok == name)
        })
    }
    
    fn find_word(&self, hay: &str, needle: &str) -> Option<(usize, usize)> {
        if needle.is_empty() { return None; }
        let mut find_from = 0;
        while let Some(hit) = hay[find_from..].find(needle) {
            let start = find_from + hit;
            let end = start + needle.len();
            let left_ok = start == 0 || !Self::is_ident_ascii(hay.as_bytes()[start-1]);
            let right_ok = end == hay.len() || !Self::is_ident_ascii(*hay.as_bytes().get(end).unwrap_or(&b' '));
            if left_ok && right_ok { return Some((start, end)); }
            find_from = end;
        }
        None
    }
    
    fn first_all_caps_word(&self, s: &str) -> Option<(usize, usize)> {
        // very small scanner: find FOO-ish
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            while i < bytes.len() && !Self::is_ident_ascii(bytes[i]) { i += 1; }
            let start = i;
            while i < bytes.len() && Self::is_ident_ascii(bytes[i]) { i += 1; }
            if start < i {
                let w = &s[start..i];
                if w.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') {
                    return Some((start, i));
                }
            }
        }
        None
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
        let text = self.get_node_text(decl);
        
        // Prefer an exact span if we can find the first occurrence with word boundaries
        if let NodeKind::Use { args, .. } = &decl.kind {
            let best_guess = args.first().map(|s| s.as_str()).unwrap_or("");
            if let Some((lo, hi)) = self.find_word(&text, best_guess) {
                let abs_lo = decl.location.start + lo;
                let abs_hi = decl.location.start + hi;
                return (abs_lo, abs_hi);
            }
        }
        
        // Try any constant-looking all-caps token in the decl
        if let Some((lo, hi)) = self.first_all_caps_word(&text) {
            return (decl.location.start + lo, decl.location.start + hi);
        }
        
        // Fallback to whole range
        (decl.location.start, decl.location.end)
    }
    
    fn get_constant_name_range_for(&self, decl: &Node, name: &str) -> (usize, usize) {
        let text = self.get_node_text(decl);
        
        // Fast path: try to find the exact word
        if let Some((lo, hi)) = self.find_word(&text, name) {
            return (decl.location.start + lo, decl.location.start + hi);
        }
        
        // Try inside all qw(...) windows
        let mut found_range = None;
        self.for_each_qw_window(&text, |start, end| {
            // Find the exact token position within this qw window
            if let Some((lo, hi)) = self.find_word(&text[start..end], name) {
                found_range = Some((decl.location.start + start + lo, decl.location.start + start + hi));
                true  // Stop searching
            } else {
                false  // Continue to next window
            }
        });
        if let Some(range) = found_range {
            return range;
        }
        
        // Try inside all { ... } blocks (hash form)
        self.for_each_brace_window(&text, |start, end| {
            if let Some((lo, hi)) = self.find_word(&text[start..end], name) {
                found_range = Some((decl.location.start + start + lo, decl.location.start + start + hi));
                true  // Stop searching
            } else {
                false  // Continue to next window
            }
        });
        if let Some(range) = found_range {
            return range;
        }
        
        // Final fallback to heuristics
        self.get_constant_name_range(decl)
    }

    fn get_children<'b>(&self, node: &'b Node) -> Vec<&'b Node> {
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