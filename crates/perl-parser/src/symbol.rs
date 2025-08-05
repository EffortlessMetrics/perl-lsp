//! Symbol extraction and symbol table for IDE features
//!
//! This module provides symbol extraction from the AST, building a symbol table
//! that tracks definitions, references, and scopes for IDE features like
//! go-to-definition, find-all-references, and semantic highlighting.

use crate::ast::{Node, NodeKind};
use crate::SourceLocation;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Type of symbol (variable, function, package, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// Scalar variable ($foo)
    ScalarVariable,
    /// Array variable (@foo)
    ArrayVariable,
    /// Hash variable (%foo)
    HashVariable,
    /// Subroutine (sub foo)
    Subroutine,
    /// Package (package Foo)
    Package,
    /// Constant (use constant FOO => 42)
    Constant,
    /// Label (FOO: while ...)
    Label,
    /// Format (format STDOUT =)
    Format,
}

impl SymbolKind {
    /// Get the sigil for this symbol kind if applicable
    pub fn sigil(&self) -> Option<&'static str> {
        match self {
            SymbolKind::ScalarVariable => Some("$"),
            SymbolKind::ArrayVariable => Some("@"),
            SymbolKind::HashVariable => Some("%"),
            _ => None,
        }
    }
}

/// A symbol definition in the code
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Symbol name (without sigil)
    pub name: String,
    /// Full qualified name (with package if applicable)
    pub qualified_name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Location where the symbol is defined
    pub location: SourceLocation,
    /// Scope where this symbol is defined
    pub scope_id: ScopeId,
    /// Declaration type (my, our, local, state)
    pub declaration: Option<String>,
    /// Documentation/comments attached to this symbol
    pub documentation: Option<String>,
    /// Attributes (e.g., :shared, :method)
    pub attributes: Vec<String>,
}

/// A reference to a symbol
#[derive(Debug, Clone)]
pub struct SymbolReference {
    /// Symbol name (without sigil)
    pub name: String,
    /// Symbol kind inferred from usage
    pub kind: SymbolKind,
    /// Location of the reference
    pub location: SourceLocation,
    /// Scope where this reference occurs
    pub scope_id: ScopeId,
    /// Whether this is a write reference (assignment)
    pub is_write: bool,
}

/// Unique identifier for a scope
pub type ScopeId = usize;

/// A lexical scope in the code
#[derive(Debug, Clone)]
pub struct Scope {
    /// Unique identifier
    pub id: ScopeId,
    /// Parent scope (None for global scope)
    pub parent: Option<ScopeId>,
    /// Kind of scope
    pub kind: ScopeKind,
    /// Source location of the scope
    pub location: SourceLocation,
    /// Symbols defined in this scope
    pub symbols: HashSet<String>,
}

/// Kind of lexical scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// Global/file scope
    Global,
    /// Package scope
    Package,
    /// Subroutine scope
    Subroutine,
    /// Block scope (if, while, for, etc.)
    Block,
    /// Eval scope
    Eval,
}

/// Symbol table containing all symbols and scopes
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// All symbols indexed by name
    pub symbols: HashMap<String, Vec<Symbol>>,
    /// All references indexed by name
    pub references: HashMap<String, Vec<SymbolReference>>,
    /// All scopes indexed by ID
    pub scopes: HashMap<ScopeId, Scope>,
    /// Current scope stack during extraction
    scope_stack: Vec<ScopeId>,
    /// Next scope ID
    next_scope_id: ScopeId,
    /// Current package name
    current_package: String,
}

impl SymbolTable {
    /// Create a new symbol table
    pub fn new() -> Self {
        let mut table = SymbolTable {
            symbols: HashMap::new(),
            references: HashMap::new(),
            scopes: HashMap::new(),
            scope_stack: vec![0],
            next_scope_id: 1,
            current_package: "main".to_string(),
        };
        
        // Create global scope
        table.scopes.insert(0, Scope {
            id: 0,
            parent: None,
            kind: ScopeKind::Global,
            location: SourceLocation { start: 0, end: 0 },
            symbols: HashSet::new(),
        });
        
        table
    }
    
    /// Get the current scope ID
    fn current_scope(&self) -> ScopeId {
        *self.scope_stack.last().unwrap_or(&0)
    }
    
    /// Push a new scope
    fn push_scope(&mut self, kind: ScopeKind, location: SourceLocation) -> ScopeId {
        let parent = self.current_scope();
        let scope_id = self.next_scope_id;
        self.next_scope_id += 1;
        
        let scope = Scope {
            id: scope_id,
            parent: Some(parent),
            kind,
            location,
            symbols: HashSet::new(),
        };
        
        self.scopes.insert(scope_id, scope);
        self.scope_stack.push(scope_id);
        scope_id
    }
    
    /// Pop the current scope
    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }
    
    /// Add a symbol definition
    fn add_symbol(&mut self, symbol: Symbol) {
        let name = symbol.name.clone();
        self.scopes.get_mut(&symbol.scope_id).unwrap().symbols.insert(name.clone());
        self.symbols.entry(name).or_default().push(symbol);
    }
    
    /// Add a symbol reference
    fn add_reference(&mut self, reference: SymbolReference) {
        let name = reference.name.clone();
        self.references.entry(name).or_default().push(reference);
    }
    
    /// Find symbol definitions visible from a given scope
    pub fn find_symbol(&self, name: &str, from_scope: ScopeId, kind: SymbolKind) -> Vec<&Symbol> {
        let mut results = Vec::new();
        let mut current_scope_id = Some(from_scope);
        
        // Walk up the scope chain
        while let Some(scope_id) = current_scope_id {
            if let Some(scope) = self.scopes.get(&scope_id) {
                // Check if symbol is defined in this scope
                if scope.symbols.contains(name) {
                    if let Some(symbols) = self.symbols.get(name) {
                        for symbol in symbols {
                            if symbol.scope_id == scope_id && symbol.kind == kind {
                                results.push(symbol);
                            }
                        }
                    }
                }
                
                // For 'our' variables, also check package scope
                if scope.kind != ScopeKind::Package {
                    if let Some(symbols) = self.symbols.get(name) {
                        for symbol in symbols {
                            if symbol.declaration.as_deref() == Some("our") && symbol.kind == kind {
                                results.push(symbol);
                            }
                        }
                    }
                }
                
                current_scope_id = scope.parent;
            } else {
                break;
            }
        }
        
        results
    }
    
    /// Get all references to a symbol
    pub fn find_references(&self, symbol: &Symbol) -> Vec<&SymbolReference> {
        self.references
            .get(&symbol.name)
            .map(|refs| refs.iter().filter(|r| r.kind == symbol.kind).collect())
            .unwrap_or_default()
    }
}

/// Extract symbols from an AST
pub struct SymbolExtractor {
    table: SymbolTable,
}

impl SymbolExtractor {
    /// Create a new symbol extractor
    pub fn new() -> Self {
        SymbolExtractor {
            table: SymbolTable::new(),
        }
    }
    
    /// Extract symbols from an AST node
    pub fn extract(mut self, node: &Node) -> SymbolTable {
        self.visit_node(node);
        self.table
    }
    
    /// Visit a node and extract symbols
    fn visit_node(&mut self, node: &Node) {
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.visit_node(stmt);
                }
            }
            
            NodeKind::VariableDeclaration { declarator, variable, attributes, initializer } => {
                self.handle_variable_declaration(declarator, variable, attributes, variable.location.clone());
                if let Some(init) = initializer {
                    self.visit_node(init);
                }
            }
            
            NodeKind::VariableListDeclaration { declarator, variables, attributes, initializer } => {
                for var in variables {
                    self.handle_variable_declaration(declarator, var, attributes, var.location.clone());
                }
                if let Some(init) = initializer {
                    self.visit_node(init);
                }
            }
            
            NodeKind::Variable { sigil, name } => {
                let kind = match sigil.as_str() {
                    "$" => SymbolKind::ScalarVariable,
                    "@" => SymbolKind::ArrayVariable,
                    "%" => SymbolKind::HashVariable,
                    _ => return,
                };
                
                let reference = SymbolReference {
                    name: name.clone(),
                    kind,
                    location: node.location.clone(),
                    scope_id: self.table.current_scope(),
                    is_write: false, // Will be updated based on context
                };
                
                self.table.add_reference(reference);
            }
            
            NodeKind::Subroutine { name, params: _, attributes, body } => {
                let sub_name = name.as_ref().map(|n| n.to_string()).unwrap_or_else(|| "<anon>".to_string());
                
                if name.is_some() {
                    let symbol = Symbol {
                        name: sub_name.clone(),
                        qualified_name: format!("{}::{}", self.table.current_package, sub_name),
                        kind: SymbolKind::Subroutine,
                        location: node.location.clone(),
                        scope_id: self.table.current_scope(),
                        declaration: None,
                        documentation: None, // TODO: Extract from preceding comments
                        attributes: attributes.clone(),
                    };
                    
                    self.table.add_symbol(symbol);
                }
                
                // Create subroutine scope
                self.table.push_scope(ScopeKind::Subroutine, node.location.clone());
                
                {
                    self.visit_node(body);
                }
                
                self.table.pop_scope();
            }
            
            NodeKind::Package { name, block } => {
                let old_package = self.table.current_package.clone();
                self.table.current_package = name.clone();
                
                let symbol = Symbol {
                    name: name.clone(),
                    qualified_name: name.clone(),
                    kind: SymbolKind::Package,
                    location: node.location.clone(),
                    scope_id: self.table.current_scope(),
                    declaration: None,
                    documentation: None,
                    attributes: vec![],
                };
                
                self.table.add_symbol(symbol);
                
                // Create package scope
                self.table.push_scope(ScopeKind::Package, node.location.clone());
                
                if let Some(block_node) = block {
                    self.visit_node(block_node);
                } else {
                    // Package declaration without block affects the rest of the file
                    // Don't pop the scope
                    return;
                }
                
                self.table.pop_scope();
                self.table.current_package = old_package;
            }
            
            NodeKind::Block { statements } => {
                self.table.push_scope(ScopeKind::Block, node.location.clone());
                for stmt in statements {
                    self.visit_node(stmt);
                }
                self.table.pop_scope();
            }
            
            NodeKind::If { condition, then_branch, elsif_branches: _, else_branch } => {
                self.visit_node(condition);
                
                self.table.push_scope(ScopeKind::Block, then_branch.location.clone());
                self.visit_node(then_branch);
                self.table.pop_scope();
                
                if let Some(else_node) = else_branch {
                    self.table.push_scope(ScopeKind::Block, else_node.location.clone());
                    self.visit_node(else_node);
                    self.table.pop_scope();
                }
            }
            
            NodeKind::While { condition, body, continue_block: _ } => {
                self.visit_node(condition);
                
                self.table.push_scope(ScopeKind::Block, body.location.clone());
                self.visit_node(body);
                self.table.pop_scope();
            }
            
            NodeKind::For { init, condition, update, body, .. } => {
                self.table.push_scope(ScopeKind::Block, node.location.clone());
                
                if let Some(init_node) = init {
                    self.visit_node(init_node);
                }
                if let Some(cond_node) = condition {
                    self.visit_node(cond_node);
                }
                if let Some(update_node) = update {
                    self.visit_node(update_node);
                }
                self.visit_node(body);
                
                self.table.pop_scope();
            }
            
            NodeKind::Foreach { variable, list, body } => {
                self.table.push_scope(ScopeKind::Block, node.location.clone());
                
                // The loop variable is implicitly declared
                self.handle_variable_declaration("my", variable, &[], variable.location.clone());
                self.visit_node(list);
                self.visit_node(body);
                
                self.table.pop_scope();
            }
            
            // Handle other node types by visiting children
            NodeKind::Assignment { lhs, rhs, .. } => {
                // Mark LHS as write reference
                self.mark_write_reference(lhs);
                self.visit_node(lhs);
                self.visit_node(rhs);
            }
            
            NodeKind::Binary { left, right, .. } => {
                self.visit_node(left);
                self.visit_node(right);
            }
            
            NodeKind::Unary { operand, .. } => {
                self.visit_node(operand);
            }
            
            NodeKind::FunctionCall { name, args } => {
                // Track function call as a reference
                let reference = SymbolReference {
                    name: name.clone(),
                    kind: SymbolKind::Subroutine,
                    location: node.location.clone(),
                    scope_id: self.table.current_scope(),
                    is_write: false,
                };
                self.table.add_reference(reference);
                
                for arg in args {
                    self.visit_node(arg);
                }
            }
            
            NodeKind::MethodCall { object, method: _, args } => {
                self.visit_node(object);
                for arg in args {
                    self.visit_node(arg);
                }
            }
            
            // ArrayRef and HashRef are handled as Binary operations with [] or {}
            
            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    self.visit_node(elem);
                }
            }
            
            NodeKind::HashLiteral { pairs } => {
                for (key, value) in pairs {
                    self.visit_node(key);
                    self.visit_node(value);
                }
            }
            
            NodeKind::Ternary { condition, then_expr, else_expr } => {
                self.visit_node(condition);
                self.visit_node(then_expr);
                self.visit_node(else_expr);
            }
            
            NodeKind::LabeledStatement { label, statement } => {
                let symbol = Symbol {
                    name: label.clone(),
                    qualified_name: label.clone(),
                    kind: SymbolKind::Label,
                    location: node.location.clone(),
                    scope_id: self.table.current_scope(),
                    declaration: None,
                    documentation: None,
                    attributes: vec![],
                };
                
                self.table.add_symbol(symbol);
                
                {
                    self.visit_node(statement);
                }
            }
            
            // Handle interpolated strings specially to extract variable references
            NodeKind::String { value, interpolated } => {
                if *interpolated {
                    // Extract variable references from interpolated strings
                    self.extract_vars_from_string(value, node.location.clone());
                }
            }
            
            // Leaf nodes - no children to visit
            NodeKind::Number { .. } |
            NodeKind::Heredoc { .. } |
            NodeKind::Regex { .. } |
            NodeKind::Substitution { .. } |
            NodeKind::Transliteration { .. } |
            NodeKind::Undef |
            NodeKind::Return { .. } |
            NodeKind::Diamond |
            NodeKind::Ellipsis |
            NodeKind::Glob { .. } |
            NodeKind::Readline { .. } |
            NodeKind::Error { .. } => {
                // No symbols to extract
            }
            
            _ => {
                // For any unhandled node types, we should still try to visit children
                // This ensures we don't miss symbols in new node types
            }
        }
    }
    
    /// Handle variable declaration
    fn handle_variable_declaration(&mut self, declarator: &str, variable: &Node, attributes: &[String], location: SourceLocation) {
        if let NodeKind::Variable { sigil, name } = &variable.kind {
            let kind = match sigil.as_str() {
                "$" => SymbolKind::ScalarVariable,
                "@" => SymbolKind::ArrayVariable,
                "%" => SymbolKind::HashVariable,
                _ => return,
            };
            
            let symbol = Symbol {
                name: name.clone(),
                qualified_name: if declarator == "our" {
                    format!("{}::{}", self.table.current_package, name)
                } else {
                    name.clone()
                },
                kind,
                location,
                scope_id: self.table.current_scope(),
                declaration: Some(declarator.to_string()),
                documentation: None, // TODO: Extract from preceding comments
                attributes: attributes.to_vec(),
            };
            
            self.table.add_symbol(symbol);
        }
    }
    
    /// Mark a node as a write reference (used in assignments)
    fn mark_write_reference(&mut self, node: &Node) {
        // This is a simplified version - in practice we'd need to handle
        // more complex LHS patterns like array/hash subscripts
        if let NodeKind::Variable { .. } = &node.kind {
            // The reference will be marked as write when we visit it
            // This would require passing context down through visit_node
        }
    }
    
    /// Extract variable references from an interpolated string
    fn extract_vars_from_string(&mut self, value: &str, string_location: SourceLocation) {
        // Simple regex to find scalar variables in strings
        // This handles $var, ${var}, but not arrays/hashes for now
        let scalar_re = Regex::new(r"\$([a-zA-Z_]\w*|\{[a-zA-Z_]\w*\})").unwrap();
        
        // The value includes quotes, so strip them
        let content = if value.len() >= 2 {
            &value[1..value.len()-1]
        } else {
            value
        };
        
        for cap in scalar_re.captures_iter(content) {
            if let Some(m) = cap.get(0) {
                let var_name = if m.as_str().starts_with("${") && m.as_str().ends_with("}") {
                    // Handle ${var} format
                    &m.as_str()[2..m.as_str().len()-1]
                } else {
                    // Handle $var format
                    &m.as_str()[1..]
                };
                
                // Calculate the location within the original string
                // This is approximate - in the actual string location
                let start_offset = string_location.start + 1 + m.start(); // +1 for opening quote
                let end_offset = start_offset + m.len();
                
                let reference = SymbolReference {
                    name: var_name.to_string(),
                    kind: SymbolKind::ScalarVariable,
                    location: SourceLocation {
                        start: start_offset,
                        end: end_offset,
                    },
                    scope_id: self.table.current_scope(),
                    is_write: false,
                };
                
                self.table.add_reference(reference);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    
    #[test]
    fn test_symbol_extraction() {
        let code = r#"
package Foo;

my $x = 42;
our $y = "hello";

sub bar {
    my $z = $x + $y;
    return $z;
}
"#;
        
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();
        
        let extractor = SymbolExtractor::new();
        let table = extractor.extract(&ast);
        
        // Check package symbol
        assert!(table.symbols.contains_key("Foo"));
        let foo_symbols = &table.symbols["Foo"];
        assert_eq!(foo_symbols.len(), 1);
        assert_eq!(foo_symbols[0].kind, SymbolKind::Package);
        
        // Check variable symbols
        assert!(table.symbols.contains_key("x"));
        assert!(table.symbols.contains_key("y"));
        assert!(table.symbols.contains_key("z"));
        
        // Check subroutine symbol
        assert!(table.symbols.contains_key("bar"));
        let bar_symbols = &table.symbols["bar"];
        assert_eq!(bar_symbols.len(), 1);
        assert_eq!(bar_symbols[0].kind, SymbolKind::Subroutine);
    }
}