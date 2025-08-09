//! Workspace-wide symbol index for fast cross-file lookups
//!
//! This module provides efficient indexing of symbols across an entire workspace,
//! enabling features like find-references, rename, and workspace symbol search.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use crate::ast::{AST, Node};
use crate::Parser;

/// A symbol in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub qualified_name: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Reference to a symbol
#[derive(Debug, Clone)]
pub struct SymbolReference {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub kind: ReferenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    Definition,
    Usage,
    Import,
    Export,
    TypeAnnotation,
}

/// Workspace index for fast symbol lookups
#[derive(Clone)]
pub struct WorkspaceIndex {
    symbols: Arc<RwLock<HashMap<String, Vec<WorkspaceSymbol>>>>,
    references: Arc<RwLock<HashMap<String, Vec<SymbolReference>>>>,
    dependencies: Arc<RwLock<HashMap<PathBuf, HashSet<PathBuf>>>>,
    dependents: Arc<RwLock<HashMap<PathBuf, HashSet<PathBuf>>>>,
    file_mtimes: Arc<RwLock<HashMap<PathBuf, std::time::SystemTime>>>,
}

impl WorkspaceIndex {
    pub fn new() -> Self {
        Self {
            symbols: Arc::new(RwLock::new(HashMap::new())),
            references: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            dependents: Arc::new(RwLock::new(HashMap::new())),
            file_mtimes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Index a single file, extracting all symbols and references
    pub fn index_file(&self, path: &Path, content: &str) -> Result<(), String> {
        // Parse the file
        let mut parser = Parser::new(content);
        let ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;
        
        // Clear old data for this file
        self.clear_file(path);
        
        // Extract symbols and references
        let mut current_package = None;
        let mut symbols_to_add = Vec::new();
        let mut references_to_add = Vec::new();
        
        self.walk_ast(&ast, content, path, &mut current_package, &mut symbols_to_add, &mut references_to_add);
        
        // Add to index
        let mut symbols = self.symbols.write().unwrap();
        for symbol in symbols_to_add {
            symbols.entry(symbol.name.clone())
                .or_default()
                .push(symbol);
        }
        
        let mut references = self.references.write().unwrap();
        for (name, reference) in references_to_add {
            references.entry(name)
                .or_default()
                .push(reference);
        }
        
        // Track file modification time
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(mtime) = metadata.modified() {
                let mut mtimes = self.file_mtimes.write().unwrap();
                mtimes.insert(path.to_path_buf(), mtime);
            }
        }
        
        Ok(())
    }
    
    /// Walk AST and extract symbols/references
    fn walk_ast(
        &self,
        ast: &AST,
        content: &str,
        path: &Path,
        current_package: &mut Option<String>,
        symbols: &mut Vec<WorkspaceSymbol>,
        references: &mut Vec<(String, SymbolReference)>,
    ) {
        self.visit_node(&ast.root, content, path, current_package, symbols, references);
    }
    
    /// Visit a node and its children
    fn visit_node(
        &self,
        node: &Node,
        content: &str,
        path: &Path,
        current_package: &mut Option<String>,
        symbols: &mut Vec<WorkspaceSymbol>,
        references: &mut Vec<(String, SymbolReference)>,
    ) {
        use crate::NodeKind;
        
        // Extract based on node kind
        match &node.kind {
            NodeKind::Package { name, .. } => {
                *current_package = Some(name.clone());
                
                symbols.push(WorkspaceSymbol {
                    name: name.clone(),
                    kind: SymbolKind::Package,
                    file_path: path.to_path_buf(),
                    line: node.location.line,
                    column: node.location.column,
                    qualified_name: Some(name.clone()),
                    documentation: None,
                });
            }
            NodeKind::Subroutine { name, .. } => {
                let qualified_name = if let Some(pkg) = current_package {
                    Some(format!("{}::{}", pkg, name))
                } else {
                    Some(name.clone())
                };
                
                symbols.push(WorkspaceSymbol {
                    name: name.clone(),
                    kind: SymbolKind::Subroutine,
                    file_path: path.to_path_buf(),
                    line: node.location.line,
                    column: node.location.column,
                    qualified_name,
                    documentation: None,
                });
                
                // This is also a definition reference
                references.push((name.clone(), SymbolReference {
                    file_path: path.to_path_buf(),
                    line: node.location.line,
                    column: node.location.column,
                    kind: ReferenceKind::Definition,
                }));
            }
            NodeKind::VariableDeclaration { variable, .. } => {
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    let var_name = format!("{}{}", sigil, name);
                    symbols.push(WorkspaceSymbol {
                        name: var_name.clone(),
                        kind: SymbolKind::Variable,
                        file_path: path.to_path_buf(),
                        line: variable.location.line,
                        column: variable.location.column,
                        qualified_name: None,
                        documentation: None,
                    });
                }
            }
            NodeKind::VariableListDeclaration { variables, .. } => {
                for var in variables {
                    if let NodeKind::Variable { sigil, name } = &var.kind {
                        let var_name = format!("{}{}", sigil, name);
                        symbols.push(WorkspaceSymbol {
                            name: var_name.clone(),
                            kind: SymbolKind::Variable,
                            file_path: path.to_path_buf(),
                            line: var.location.line,
                            column: var.location.column,
                            qualified_name: None,
                            documentation: None,
                        });
                    }
                }
            }
            NodeKind::FunctionCall { name, .. } => {
                references.push((name.clone(), SymbolReference {
                    file_path: path.to_path_buf(),
                    line: node.location.line,
                    column: node.location.column,
                    kind: ReferenceKind::Usage,
                }));
            }
            NodeKind::MethodCall { method, .. } => {
                references.push((method.clone(), SymbolReference {
                    file_path: path.to_path_buf(),
                    line: node.location.line,
                    column: node.location.column,
                    kind: ReferenceKind::Usage,
                }));
            }
            NodeKind::Use { module, .. } => {
                references.push((module.clone(), SymbolReference {
                    file_path: path.to_path_buf(),
                    line: node.location.line,
                    column: node.location.column,
                    kind: ReferenceKind::Import,
                }));
                
                // Track dependency
                let mut deps = self.dependencies.write().unwrap();
                deps.entry(path.to_path_buf())
                    .or_default()
                    .insert(PathBuf::from(module.replace("::", "/") + ".pm"));
            }
            _ => {}
        }
        
        // Visit children - use the children() method from scope_analyzer.rs
        use crate::scope_analyzer::*;  // Import Node trait impl
        for child in node.children() {
            self.visit_node(child, content, path, current_package, symbols, references);
        }
    }

    /// Find all symbols with a given name
    pub fn find_symbols(&self, name: &str) -> Vec<WorkspaceSymbol> {
        let symbols = self.symbols.read().unwrap();
        symbols.get(name).cloned().unwrap_or_default()
    }

    /// Find all references to a symbol
    pub fn find_references(&self, name: &str) -> Vec<SymbolReference> {
        let references = self.references.read().unwrap();
        references.get(name).cloned().unwrap_or_default()
    }

    /// Get all files that depend on a given file
    pub fn get_dependents(&self, file: &Path) -> HashSet<PathBuf> {
        let dependents = self.dependents.read().unwrap();
        dependents.get(file).cloned().unwrap_or_default()
    }

    /// Get all files that a given file depends on
    pub fn get_dependencies(&self, file: &Path) -> HashSet<PathBuf> {
        let dependencies = self.dependencies.read().unwrap();
        dependencies.get(file).cloned().unwrap_or_default()
    }

    /// Find unused symbols
    pub fn find_unused_symbols(&self) -> Vec<WorkspaceSymbol> {
        let symbols = self.symbols.read().unwrap();
        let references = self.references.read().unwrap();
        let mut unused = Vec::new();
        
        for (name, symbol_list) in symbols.iter() {
            // Check if this symbol has any non-definition references
            let refs = references.get(name);
            let has_usage = refs.map_or(false, |refs| {
                refs.iter().any(|r| r.kind != ReferenceKind::Definition)
            });
            
            if !has_usage {
                // Check if it's exported or has special meaning
                let is_special = name == "main" || 
                                 name.starts_with("BEGIN") || 
                                 name.starts_with("END") ||
                                 name.starts_with("CHECK") ||
                                 name.starts_with("INIT");
                
                if !is_special {
                    unused.extend(symbol_list.clone());
                }
            }
        }
        
        unused
    }

    /// Check if a file needs re-indexing
    pub fn needs_reindex(&self, path: &Path) -> bool {
        let mtimes = self.file_mtimes.read().unwrap();
        if let Some(stored_mtime) = mtimes.get(path) {
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(current_mtime) = metadata.modified() {
                    return current_mtime > *stored_mtime;
                }
            }
        }
        true
    }

    /// Get package members for completion
    pub fn get_package_members(&self, package: &str) -> Vec<WorkspaceSymbol> {
        let symbols = self.symbols.read().unwrap();
        let mut members = Vec::new();
        
        for (_, symbol_list) in symbols.iter() {
            for symbol in symbol_list {
                if let Some(ref qualified) = symbol.qualified_name {
                    if qualified.starts_with(&format!("{}::", package)) {
                        members.push(symbol.clone());
                    }
                }
            }
        }
        
        members
    }
    
    /// Clear index for a file
    pub fn clear_file(&self, path: &Path) {
        let mut symbols = self.symbols.write().unwrap();
        let mut references = self.references.write().unwrap();
        
        for syms in symbols.values_mut() {
            syms.retain(|s| s.file_path != path);
        }
        
        for refs in references.values_mut() {
            refs.retain(|r| r.file_path != path);
        }

        let mut deps = self.dependencies.write().unwrap();
        deps.remove(path);

        let mut rev_deps = self.dependents.write().unwrap();
        for dependents_set in rev_deps.values_mut() {
            dependents_set.remove(path);
        }
    }
}

impl Default for WorkspaceIndex {
    fn default() -> Self {
        Self::new()
    }
}