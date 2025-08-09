//! Workspace-wide symbol index for fast cross-file lookups
//!
//! This module provides efficient indexing of symbols across an entire workspace,
//! enabling features like find-references, rename, and workspace symbol search.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use crate::ast::{Node, NodeKind};
use crate::Parser;
use crate::document_store::{Document, DocumentStore};

/// A position in a document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// A range in a document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// A location in a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// A symbol in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub uri: String,
    pub range: Range,
    pub qualified_name: Option<String>,
    pub documentation: Option<String>,
    pub container_name: Option<String>,
    #[serde(default = "default_has_body")]
    pub has_body: bool,  // For forward declarations
}

fn default_has_body() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Package,
    Subroutine,
    Method,
    Variable,
    Constant,
    Class,
    Role,
    Import,
    Export,
}

impl SymbolKind {
    /// Convert to LSP-compliant symbol kind number
    pub fn to_lsp_kind(self) -> u32 {
        // Using lsp_types::SymbolKind constants
        match self {
            SymbolKind::Package => 2,     // Module
            SymbolKind::Subroutine => 12, // Function
            SymbolKind::Method => 6,       // Method
            SymbolKind::Variable => 13,    // Variable
            SymbolKind::Constant => 14,    // Constant
            SymbolKind::Class => 5,        // Class
            SymbolKind::Role => 8,         // Interface (closest match)
            SymbolKind::Import => 2,       // Module
            SymbolKind::Export => 12,      // Function
        }
    }
}

/// Reference to a symbol
#[derive(Debug, Clone)]
pub struct SymbolReference {
    pub uri: String,
    pub range: Range,
    pub kind: ReferenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    Definition,
    Usage,
    Import,
    Read,
    Write,
}

/// LSP-compliant workspace symbol for wire format (no internal fields)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspWorkspaceSymbol {
    pub name: String,
    pub kind: u32,
    pub location: LspLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
}

/// LSP-compliant location
#[derive(Debug, Serialize)]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}

/// LSP-compliant range
#[derive(Debug, Serialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP-compliant position
#[derive(Debug, Serialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

impl From<&WorkspaceSymbol> for LspWorkspaceSymbol {
    fn from(sym: &WorkspaceSymbol) -> Self {
        Self {
            name: sym.name.clone(),
            kind: sym.kind.to_lsp_kind(),
            location: LspLocation {
                uri: sym.uri.clone(),
                range: LspRange {
                    start: LspPosition {
                        line: sym.range.start.line,
                        character: sym.range.start.character,
                    },
                    end: LspPosition {
                        line: sym.range.end.line,
                        character: sym.range.end.character,
                    },
                },
            },
            container_name: sym.container_name.clone(),
        }
    }
}

/// File-level index data
#[derive(Default)]
struct FileIndex {
    /// Symbols defined in this file
    symbols: Vec<WorkspaceSymbol>,
    /// References in this file (symbol name -> references)
    references: HashMap<String, Vec<SymbolReference>>,
    /// Dependencies (modules this file imports)
    dependencies: HashSet<String>,
}

/// Thread-safe workspace index
pub struct WorkspaceIndex {
    /// Index data per file URI (normalized key -> data)
    files: Arc<RwLock<HashMap<String, FileIndex>>>,
    /// Global symbol map (qualified name -> defining URI)
    symbols: Arc<RwLock<HashMap<String, String>>>,
    /// Document store for in-memory text
    document_store: DocumentStore,
}

impl WorkspaceIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            symbols: Arc::new(RwLock::new(HashMap::new())),
            document_store: DocumentStore::new(),
        }
    }
    
    /// Normalize a URI to a consistent form using lsp_types
    fn normalize_uri(uri: &str) -> String {
        // Try to parse as URL first
        use url::Url;
        
        if let Ok(url) = Url::parse(uri) {
            return url.to_string();
        }
        
        // If not a valid URL, try as file path
        if let Ok(url) = Url::from_file_path(uri) {
            return url.to_string();
        }
        
        // Fallback: try to construct file URL manually
        let path = if uri.starts_with("file://") {
            &uri[7..]
        } else {
            uri
        };
        
        // Try to create file URL from path
        std::path::Path::new(path)
            .canonicalize()
            .ok()
            .and_then(|p| Url::from_file_path(p).ok())
            .map(|u| u.to_string())
            .unwrap_or_else(|| {
                // Last resort: just ensure file:// prefix
                if uri.starts_with("file://") || uri.starts_with("untitled:") {
                    uri.to_string()
                } else if uri.starts_with('/') {
                    format!("file://{}", uri)
                } else {
                    // Relative path - keep as is
                    uri.to_string()
                }
            })
    }
    
    /// Index a file from its URI and text content
    pub fn index_file(&self, uri: &str, text: &str, version: i32) -> Result<(), String> {
        let normalized_uri = Self::normalize_uri(uri);
        
        // Update document store
        if self.document_store.is_open(&normalized_uri) {
            self.document_store.update(&normalized_uri, version, text.to_string());
        } else {
            self.document_store.open(normalized_uri.clone(), version, text.to_string());
        }
        
        // Parse the file
        let mut parser = Parser::new(text);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(e) => return Err(format!("Parse error: {}", e)),
        };
        
        // Get the document for line index
        let mut doc = self.document_store.get(&normalized_uri).ok_or("Document not found")?;
        
        // Extract symbols and references
        let mut file_index = FileIndex::default();
        let mut visitor = IndexVisitor::new(&mut doc, normalized_uri.clone());
        visitor.visit(&ast, &mut file_index);
        
        // Update the index
        let key = DocumentStore::uri_key(&normalized_uri);
        {
            let mut files = self.files.write().unwrap();
            files.insert(key.clone(), file_index);
        }
        
        // Update global symbol map
        {
            let files = self.files.read().unwrap();
            if let Some(file_index) = files.get(&key) {
                let mut symbols = self.symbols.write().unwrap();
                for symbol in &file_index.symbols {
                    if let Some(ref qname) = symbol.qualified_name {
                        symbols.insert(qname.clone(), uri.to_string());
                    } else {
                        symbols.insert(symbol.name.clone(), uri.to_string());
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Remove a file from the index
    pub fn remove_file(&self, uri: &str) {
        let normalized_uri = Self::normalize_uri(uri);
        let key = DocumentStore::uri_key(&normalized_uri);
        
        // Remove from document store
        self.document_store.close(&normalized_uri);
        
        // Remove file index
        let mut files = self.files.write().unwrap();
        if let Some(file_index) = files.remove(&key) {
            // Remove from global symbol map
            let mut symbols = self.symbols.write().unwrap();
            for symbol in file_index.symbols {
                if let Some(ref qname) = symbol.qualified_name {
                    symbols.remove(qname);
                } else {
                    symbols.remove(&symbol.name);
                }
            }
        }
    }
    
    /// Clear a file from the index (alias for remove_file)
    pub fn clear_file(&self, uri: &str) {
        let normalized_uri = Self::normalize_uri(uri);
        self.remove_file(&normalized_uri);
    }
    
    /// Find all references to a symbol
    pub fn find_references(&self, symbol_name: &str) -> Vec<Location> {
        let mut locations = Vec::new();
        let files = self.files.read().unwrap();
        
        for (_uri_key, file_index) in files.iter() {
            if let Some(refs) = file_index.references.get(symbol_name) {
                for reference in refs {
                    locations.push(Location {
                        uri: reference.uri.clone(),
                        range: reference.range,
                    });
                }
            }
        }
        
        locations
    }
    
    /// Find the definition of a symbol
    pub fn find_definition(&self, symbol_name: &str) -> Option<Location> {
        let files = self.files.read().unwrap();
        
        for (_uri_key, file_index) in files.iter() {
            for symbol in &file_index.symbols {
                if symbol.name == symbol_name || symbol.qualified_name.as_deref() == Some(symbol_name) {
                    return Some(Location {
                        uri: symbol.uri.clone(),
                        range: symbol.range,
                    });
                }
            }
        }
        
        None
    }
    
    /// Get all symbols in the workspace
    pub fn all_symbols(&self) -> Vec<WorkspaceSymbol> {
        let files = self.files.read().unwrap();
        let mut symbols = Vec::new();
        
        for (_uri_key, file_index) in files.iter() {
            symbols.extend(file_index.symbols.clone());
        }
        
        symbols
    }
    
    /// Search for symbols by query
    pub fn search_symbols(&self, query: &str) -> Vec<WorkspaceSymbol> {
        let query_lower = query.to_lowercase();
        self.all_symbols()
            .into_iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower) ||
                s.qualified_name.as_ref()
                    .map(|qn| qn.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
            })
            .collect()
    }
    
    /// Find symbols by query (alias for search_symbols for compatibility)
    pub fn find_symbols(&self, query: &str) -> Vec<WorkspaceSymbol> {
        self.search_symbols(query)
    }
    
    /// Get symbols in a specific file
    pub fn file_symbols(&self, uri: &str) -> Vec<WorkspaceSymbol> {
        let normalized_uri = Self::normalize_uri(uri);
        let key = DocumentStore::uri_key(&normalized_uri);
        let files = self.files.read().unwrap();
        
        files.get(&key)
            .map(|fi| fi.symbols.clone())
            .unwrap_or_default()
    }
    
    /// Get dependencies of a file
    pub fn file_dependencies(&self, uri: &str) -> HashSet<String> {
        let normalized_uri = Self::normalize_uri(uri);
        let key = DocumentStore::uri_key(&normalized_uri);
        let files = self.files.read().unwrap();
        
        files.get(&key)
            .map(|fi| fi.dependencies.clone())
            .unwrap_or_default()
    }
    
    /// Find all files that depend on a module
    pub fn find_dependents(&self, module_name: &str) -> Vec<String> {
        let files = self.files.read().unwrap();
        let mut dependents = Vec::new();
        
        for (uri_key, file_index) in files.iter() {
            if file_index.dependencies.contains(module_name) {
                dependents.push(uri_key.clone());
            }
        }
        
        dependents
    }
    
    /// Get the document store
    pub fn document_store(&self) -> &DocumentStore {
        &self.document_store
    }
    
    /// Find unused symbols in the workspace
    pub fn find_unused_symbols(&self) -> Vec<WorkspaceSymbol> {
        let files = self.files.read().unwrap();
        let mut unused = Vec::new();
        
        // Collect all defined symbols
        for (_uri_key, file_index) in files.iter() {
            for symbol in &file_index.symbols {
                // Check if this symbol has any references beyond its definition
                let has_usage = files.values().any(|fi| {
                    if let Some(refs) = fi.references.get(&symbol.name) {
                        refs.iter().any(|r| r.kind != ReferenceKind::Definition)
                    } else {
                        false
                    }
                });
                
                if !has_usage {
                    unused.push(symbol.clone());
                }
            }
        }
        
        unused
    }
    
    /// Get all symbols that belong to a specific package
    pub fn get_package_members(&self, package_name: &str) -> Vec<WorkspaceSymbol> {
        let files = self.files.read().unwrap();
        let mut members = Vec::new();
        
        for (_uri_key, file_index) in files.iter() {
            for symbol in &file_index.symbols {
                // Check if symbol belongs to this package
                if let Some(ref container) = symbol.container_name {
                    if container == package_name {
                        members.push(symbol.clone());
                    }
                }
                // Also check qualified names
                if let Some(ref qname) = symbol.qualified_name {
                    if qname.starts_with(&format!("{}::", package_name)) {
                        // Avoid duplicates - only add if not already in via container_name
                        if symbol.container_name.as_deref() != Some(package_name) {
                            members.push(symbol.clone());
                        }
                    }
                }
            }
        }
        
        members
    }
}

/// AST visitor for extracting symbols and references
struct IndexVisitor {
    document: Document,
    uri: String,
    current_package: Option<String>,
}

impl IndexVisitor {
    fn new(document: &mut Document, uri: String) -> Self {
        Self {
            document: document.clone(),
            uri,
            current_package: Some("main".to_string()),
        }
    }
    
    fn visit(&mut self, node: &Node, file_index: &mut FileIndex) {
        self.visit_node(node, file_index);
    }
    
    fn visit_node(&mut self, node: &Node, file_index: &mut FileIndex) {
        match &node.kind {
            NodeKind::Package { name, .. } => {
                let package_name = name.clone();
                
                // Update the current package (replaces the previous one, not a stack)
                self.current_package = Some(package_name.clone());
                
                file_index.symbols.push(WorkspaceSymbol {
                    name: package_name.clone(),
                    kind: SymbolKind::Package,
                    uri: self.uri.clone(),
                    range: self.node_to_range(node),
                    qualified_name: Some(package_name),
                    documentation: None,
                    container_name: None,
                    has_body: true,
                });
            }
            
            NodeKind::Subroutine { name, body, .. } => {
                if let Some(name_str) = name.clone() {
                    let qualified_name = if let Some(ref pkg) = self.current_package {
                        format!("{}::{}", pkg, name_str)
                    } else {
                        name_str.clone()
                    };
                    
                    // Check if this is a forward declaration or update to existing symbol
                    let existing_symbol_idx = file_index.symbols.iter().position(|s| {
                        s.name == name_str && s.container_name == self.current_package
                    });
                    
                    if let Some(idx) = existing_symbol_idx {
                        // Update existing forward declaration with body
                        file_index.symbols[idx].range = self.node_to_range(node);
                    } else {
                        // New symbol
                        file_index.symbols.push(WorkspaceSymbol {
                            name: name_str.clone(),
                            kind: SymbolKind::Subroutine,
                            uri: self.uri.clone(),
                            range: self.node_to_range(node),
                            qualified_name: Some(qualified_name),
                            documentation: None,
                            container_name: self.current_package.clone(),
                            has_body: true,  // Subroutine node always has body
                        });
                    }
                    
                    // Mark as definition
                    file_index.references
                        .entry(name_str.clone())
                        .or_default()
                        .push(SymbolReference {
                            uri: self.uri.clone(),
                            range: self.node_to_range(node),
                            kind: ReferenceKind::Definition,
                        });
                }
                
                // Visit body
                self.visit_node(body, file_index);
            }
            
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    let var_name = format!("{}{}", sigil, name);
                    
                    file_index.symbols.push(WorkspaceSymbol {
                        name: var_name.clone(),
                        kind: SymbolKind::Variable,
                        uri: self.uri.clone(),
                        range: self.node_to_range(variable),
                        qualified_name: None,
                        documentation: None,
                        container_name: self.current_package.clone(),
                        has_body: true,  // Variables always have body
                    });
                    
                    // Mark as definition
                    file_index.references
                        .entry(var_name.clone())
                        .or_default()
                        .push(SymbolReference {
                            uri: self.uri.clone(),
                            range: self.node_to_range(variable),
                            kind: ReferenceKind::Definition,
                        });
                }
                
                // Visit initializer
                if let Some(init) = initializer {
                    self.visit_node(init, file_index);
                }
            }
            
            NodeKind::VariableListDeclaration { variables, initializer, .. } => {
                // Handle each variable in the list declaration
                for var in variables {
                    if let NodeKind::Variable { sigil, name } = &var.kind {
                        let var_name = format!("{}{}", sigil, name);
                        
                        file_index.symbols.push(WorkspaceSymbol {
                            name: var_name.clone(),
                            kind: SymbolKind::Variable,
                            uri: self.uri.clone(),
                            range: self.node_to_range(var),
                            qualified_name: None,
                            documentation: None,
                            container_name: self.current_package.clone(),
                            has_body: true,
                        });
                        
                        // Mark as definition
                        file_index.references
                            .entry(var_name)
                            .or_default()
                            .push(SymbolReference {
                                uri: self.uri.clone(),
                                range: self.node_to_range(var),
                                kind: ReferenceKind::Definition,
                            });
                    }
                }
                
                // Visit the initializer
                if let Some(init) = initializer {
                    self.visit_node(init, file_index);
                }
            }
            
            NodeKind::Variable { sigil, name } => {
                let var_name = format!("{}{}", sigil, name);
                
                // Track as usage (could be read or write based on context)
                file_index.references
                    .entry(var_name)
                    .or_default()
                    .push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(node),
                        kind: ReferenceKind::Read, // Default to read, would need context for write
                    });
            }
            
            NodeKind::FunctionCall { name, args, .. } => {
                let func_name = name.clone();
                
                // Track as usage
                file_index.references
                    .entry(func_name)
                    .or_default()
                    .push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(node),
                        kind: ReferenceKind::Usage,
                    });
                
                // Visit arguments
                for arg in args {
                    self.visit_node(arg, file_index);
                }
            }
            
            NodeKind::Use { module, .. } => {
                let module_name = module.clone();
                file_index.dependencies.insert(module_name.clone());
                
                // Track as import
                file_index.references
                    .entry(module_name)
                    .or_default()
                    .push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(node),
                        kind: ReferenceKind::Import,
                    });
            }
            
            // Handle assignment to detect writes
            NodeKind::Assignment { lhs, rhs, op } => {
                // For compound assignments (+=, -=, .=, etc.), the LHS is both read and written
                let is_compound = op != "=";
                
                if let NodeKind::Variable { sigil, name } = &lhs.kind {
                    let var_name = format!("{}{}", sigil, name);
                    
                    // For compound assignments, it's a read first
                    if is_compound {
                        file_index.references
                            .entry(var_name.clone())
                            .or_default()
                            .push(SymbolReference {
                                uri: self.uri.clone(),
                                range: self.node_to_range(lhs),
                                kind: ReferenceKind::Read,
                            });
                    }
                    
                    // Then it's always a write
                    file_index.references
                        .entry(var_name)
                        .or_default()
                        .push(SymbolReference {
                            uri: self.uri.clone(),
                            range: self.node_to_range(lhs),
                            kind: ReferenceKind::Write,
                        });
                }
                
                // Right side could have reads
                self.visit_node(rhs, file_index);
            }
            
            // Recursively visit child nodes
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, file_index);
                }
            }
            
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.visit_node(condition, file_index);
                self.visit_node(then_branch, file_index);
                for (cond, branch) in elsif_branches {
                    self.visit_node(cond, file_index);
                    self.visit_node(branch, file_index);
                }
                if let Some(else_br) = else_branch {
                    self.visit_node(else_br, file_index);
                }
            }
            
            NodeKind::While { condition, body, continue_block } => {
                self.visit_node(condition, file_index);
                self.visit_node(body, file_index);
                if let Some(cont) = continue_block {
                    self.visit_node(cont, file_index);
                }
            }
            
            NodeKind::For { init, condition, update, body, continue_block } => {
                if let Some(i) = init {
                    self.visit_node(i, file_index);
                }
                if let Some(c) = condition {
                    self.visit_node(c, file_index);
                }
                if let Some(u) = update {
                    self.visit_node(u, file_index);
                }
                self.visit_node(body, file_index);
                if let Some(cont) = continue_block {
                    self.visit_node(cont, file_index);
                }
            }
            
            NodeKind::Foreach { variable, list, body } => {
                // Iterator is a write context
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    let var_name = format!("{}{}", sigil, name);
                    file_index.references
                        .entry(var_name)
                        .or_default()
                        .push(SymbolReference {
                            uri: self.uri.clone(),
                            range: self.node_to_range(variable),
                            kind: ReferenceKind::Write,
                        });
                }
                self.visit_node(variable, file_index); 
                self.visit_node(list, file_index);
                self.visit_node(body, file_index);
            }
            
            NodeKind::MethodCall { object, method, args } => {
                // Check if this is a static method call (Package->method)
                let qualified_method = if let NodeKind::Identifier { name } = &object.kind {
                    // Static method call: Package->method
                    Some(format!("{}::{}", name, method))
                } else {
                    // Instance method call: $obj->method
                    None
                };
                
                // Object is a read context
                self.visit_node(object, file_index);
                
                // Track method call with qualified name if applicable
                let method_key = qualified_method.as_ref().unwrap_or(method);
                file_index.references
                    .entry(method_key.clone())
                    .or_default()
                    .push(SymbolReference {
                        uri: self.uri.clone(),
                        range: self.node_to_range(node),
                        kind: ReferenceKind::Usage,
                    });
                
                // Visit arguments
                for arg in args {
                    self.visit_node(arg, file_index);
                }
            }
            
            NodeKind::No { module, .. } => {
                let module_name = module.clone();
                file_index.dependencies.insert(module_name.clone());
            }
            
            NodeKind::Class { name, .. } => {
                let class_name = name.clone();
                self.current_package = Some(class_name.clone());
                
                file_index.symbols.push(WorkspaceSymbol {
                    name: class_name.clone(),
                    kind: SymbolKind::Class,
                    uri: self.uri.clone(),
                    range: self.node_to_range(node),
                    qualified_name: Some(class_name),
                    documentation: None,
                    container_name: None,
                    has_body: true,
                });
            }
            
            NodeKind::Method { name, body, params } => {
                let method_name = name.clone();
                let qualified_name = if let Some(ref pkg) = self.current_package {
                    format!("{}::{}", pkg, method_name)
                } else {
                    method_name.clone()
                };
                
                file_index.symbols.push(WorkspaceSymbol {
                    name: method_name.clone(),
                    kind: SymbolKind::Method,
                    uri: self.uri.clone(),
                    range: self.node_to_range(node),
                    qualified_name: Some(qualified_name),
                    documentation: None,
                    container_name: self.current_package.clone(),
                    has_body: true,
                });
                
                // Visit params
                for param in params {
                    self.visit_node(param, file_index);
                }
                
                // Visit body
                self.visit_node(body, file_index);
            }
            
            // Handle special assignments (++ and --)
            NodeKind::Unary { op, operand } if op == "++" || op == "--" => {
                // Pre/post increment/decrement are both read and write
                if let NodeKind::Variable { sigil, name } = &operand.kind {
                    let var_name = format!("{}{}", sigil, name);
                    
                    // It's both a read and a write
                    file_index.references
                        .entry(var_name.clone())
                        .or_default()
                        .push(SymbolReference {
                            uri: self.uri.clone(),
                            range: self.node_to_range(operand),
                            kind: ReferenceKind::Read,
                        });
                        
                    file_index.references
                        .entry(var_name)
                        .or_default()
                        .push(SymbolReference {
                            uri: self.uri.clone(),
                            range: self.node_to_range(operand),
                            kind: ReferenceKind::Write,
                        });
                }
            }
            
            _ => {
                // For other node types, just visit children
                self.visit_children(node, file_index);
            }
        }
    }
    
    fn visit_children(&mut self, node: &Node, file_index: &mut FileIndex) {
        // Generic visitor for unhandled node types - visit all nested nodes
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, file_index);
                }
            }
            // Expression nodes
            NodeKind::Unary { operand, .. } => {
                self.visit_node(operand, file_index);
            }
            NodeKind::Binary { left, right, .. } => {
                self.visit_node(left, file_index);
                self.visit_node(right, file_index);
            }
            NodeKind::Ternary { condition, then_expr, else_expr } => {
                self.visit_node(condition, file_index);
                self.visit_node(then_expr, file_index);
                self.visit_node(else_expr, file_index);
            }
            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    self.visit_node(elem, file_index);
                }
            }
            NodeKind::HashLiteral { pairs } => {
                for (key, value) in pairs {
                    self.visit_node(key, file_index);
                    self.visit_node(value, file_index);
                }
            }
            NodeKind::Return { value } => {
                if let Some(val) = value {
                    self.visit_node(val, file_index);
                }
            }
            NodeKind::Eval { block } | NodeKind::Do { block } => {
                self.visit_node(block, file_index);
            }
            NodeKind::Try { body, catch_blocks, finally_block } => {
                self.visit_node(body, file_index);
                for (_, block) in catch_blocks {
                    self.visit_node(block, file_index);
                }
                if let Some(finally) = finally_block {
                    self.visit_node(finally, file_index);
                }
            }
            NodeKind::Given { expr, body } => {
                self.visit_node(expr, file_index);
                self.visit_node(body, file_index);
            }
            NodeKind::When { condition, body } => {
                self.visit_node(condition, file_index);
                self.visit_node(body, file_index);
            }
            NodeKind::Default { body } => {
                self.visit_node(body, file_index);
            }
            NodeKind::StatementModifier { statement, condition, .. } => {
                self.visit_node(statement, file_index);
                self.visit_node(condition, file_index);
            }
            NodeKind::VariableWithAttributes { variable, .. } => {
                self.visit_node(variable, file_index);
            }
            NodeKind::LabeledStatement { statement, .. } => {
                self.visit_node(statement, file_index);
            }
            _ => {
                // For other node types, no children to visit
            }
        }
    }
    
    fn node_to_range(&mut self, node: &Node) -> Range {
        // LineIndex.range returns line numbers and UTF-16 code unit columns
        let ((start_line, start_col), (end_line, end_col)) = 
            self.document.line_index.range(node.location.start, node.location.end);
        Range {
            start: Position { line: start_line, character: start_col },
            end: Position { line: end_line, character: end_col },
        }
    }
}

impl Default for WorkspaceIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_indexing() {
        let index = WorkspaceIndex::new();
        let uri = "file:///test.pl";
        
        let code = r#"
package MyPackage;

sub hello {
    print "Hello";
}

my $var = 42;
"#;
        
        index.index_file(uri, code, 1).unwrap();
        
        // Should have indexed the package and subroutine
        let symbols = index.file_symbols(uri);
        assert!(symbols.iter().any(|s| s.name == "MyPackage" && s.kind == SymbolKind::Package));
        assert!(symbols.iter().any(|s| s.name == "hello" && s.kind == SymbolKind::Subroutine));
        assert!(symbols.iter().any(|s| s.name == "$var" && s.kind == SymbolKind::Variable));
    }
    
    #[test]
    fn test_find_references() {
        let index = WorkspaceIndex::new();
        let uri = "file:///test.pl";
        
        let code = r#"
sub test {
    my $x = 1;
    $x = 2;
    print $x;
}
"#;
        
        index.index_file(uri, code, 1).unwrap();
        
        let refs = index.find_references("$x");
        assert!(refs.len() >= 2); // Definition + at least one usage
    }
    
    #[test]
    fn test_dependencies() {
        let index = WorkspaceIndex::new();
        let uri = "file:///test.pl";
        
        let code = r#"
use strict;
use warnings;
use Data::Dumper;
"#;
        
        index.index_file(uri, code, 1).unwrap();
        
        let deps = index.file_dependencies(uri);
        assert!(deps.contains("strict"));
        assert!(deps.contains("warnings"));
        assert!(deps.contains("Data::Dumper"));
    }
}