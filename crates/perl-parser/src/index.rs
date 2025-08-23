//! Cross-file workspace indexing for Perl symbols
//!
//! This module provides efficient indexing of symbols across all files in a workspace,
//! enabling fast cross-file navigation, references, and refactoring.

use crate::symbol::{SymbolKind, SymbolTable};
use std::collections::{HashMap, HashSet};

/// A symbol definition in the workspace
#[derive(Clone, Debug)]
pub struct SymbolDef {
    /// The name of the symbol
    pub name: String,
    /// The kind of symbol (function, variable, package, etc.)
    pub kind: SymbolKind,
    /// The URI of the file containing this symbol
    pub uri: String,
    /// Start byte offset in the file
    pub start: usize,
    /// End byte offset in the file
    pub end: usize,
}

/// Workspace-wide index for fast symbol lookups
#[derive(Default)]
pub struct WorkspaceIndex {
    /// Index from symbol name to all its definitions
    by_name: HashMap<String, Vec<SymbolDef>>,
    /// Index from URI to all symbol names in that file (for fast removal)
    by_uri: HashMap<String, HashSet<String>>,
}

impl WorkspaceIndex {
    /// Create a new empty workspace index
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the index with symbols from a document
    pub fn update_from_document(&mut self, uri: &str, _content: &str, symtab: &SymbolTable) {
        // Remove old symbols from this file
        self.remove_document(uri);

        // Track all symbol names in this file
        let mut names_in_file = HashSet::new();

        // Add all symbols from the symbol table
        for symbols in symtab.symbols.values() {
            for symbol in symbols {
                let name = symbol.name.clone();
                names_in_file.insert(name.clone());

                let def = SymbolDef {
                    name: symbol.name.clone(),
                    kind: symbol.kind,
                    uri: uri.to_string(),
                    start: symbol.location.start,
                    end: symbol.location.end,
                };

                self.by_name.entry(name).or_default().push(def);
            }
        }

        // Track which names are in this file
        self.by_uri.insert(uri.to_string(), names_in_file);
    }

    /// Remove all symbols from a document
    pub fn remove_document(&mut self, uri: &str) {
        if let Some(names) = self.by_uri.remove(uri) {
            for name in names {
                if let Some(defs) = self.by_name.get_mut(&name) {
                    defs.retain(|d| d.uri != uri);
                    if defs.is_empty() {
                        self.by_name.remove(&name);
                    }
                }
            }
        }
    }

    /// Find all definitions of a symbol by name
    pub fn find_defs(&self, name: &str) -> &[SymbolDef] {
        static EMPTY: Vec<SymbolDef> = Vec::new();
        self.by_name.get(name).map(|v| v.as_slice()).unwrap_or(&EMPTY[..])
    }

    /// Find all references to a symbol (simplified version)
    /// In a full implementation, this would analyze usage sites
    pub fn find_refs(&self, name: &str) -> Vec<SymbolDef> {
        // For now, return all definitions as references
        // A full implementation would scan all files for usage sites
        self.find_defs(name).to_vec()
    }

    /// Get all symbols in the workspace matching a query
    pub fn search_symbols(&self, query: &str) -> Vec<SymbolDef> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (name, defs) in &self.by_name {
            if name.to_lowercase().contains(&query_lower) {
                results.extend(defs.clone());
            }
        }

        results
    }

    /// Get the total number of indexed symbols
    pub fn symbol_count(&self) -> usize {
        self.by_name.values().map(|v| v.len()).sum()
    }

    /// Get the number of indexed files
    pub fn file_count(&self) -> usize {
        self.by_uri.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceLocation;
    use crate::symbol::Symbol;

    #[test]
    fn test_workspace_index() {
        let mut index = WorkspaceIndex::new();

        // Create a mock symbol table
        let mut symtab = SymbolTable::new();

        // Add a symbol to the symbol table
        let symbol = Symbol {
            name: "test_func".to_string(),
            qualified_name: "main::test_func".to_string(),
            kind: SymbolKind::Subroutine,
            location: SourceLocation { start: 0, end: 10 },
            scope_id: 0,
            declaration: Some("sub".to_string()),
            documentation: None,
            attributes: Vec::new(),
        };

        symtab.symbols.entry("test_func".to_string()).or_default().push(symbol);

        // Add document to index
        index.update_from_document("file:///test.pl", "", &symtab);

        // Find definitions
        let defs = index.find_defs("test_func");
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "test_func");
        assert_eq!(defs[0].uri, "file:///test.pl");

        // Remove document
        index.remove_document("file:///test.pl");
        assert_eq!(index.find_defs("test_func").len(), 0);
    }
}
