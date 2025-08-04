//! Workspace symbols provider for LSP
//!
//! Provides workspace/symbol functionality for searching symbols across all files.

use crate::{
    ast::{Node, NodeKind},
    symbol::{Symbol, SymbolKind, SymbolExtractor},
    lsp::{WorkspaceFeatureProvider, FeatureProvider},
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

/// Workspace symbol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub uri: String,
    pub range: Range,
    pub container_name: Option<String>,
}

/// LSP Range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// LSP Position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Workspace symbols provider
pub struct WorkspaceSymbolsProvider {
    /// Symbol index for fast searching
    index: Arc<RwLock<SymbolIndex>>,
    /// Whether to index local symbols
    include_locals: bool,
}

/// Internal symbol index
struct SymbolIndex {
    /// Symbols by document URI
    documents: HashMap<String, Vec<IndexedSymbol>>,
    /// Global symbol name index for fast lookup
    name_index: HashMap<String, Vec<SymbolLocation>>,
}

/// Indexed symbol with location
#[derive(Debug, Clone)]
struct IndexedSymbol {
    name: String,
    kind: SymbolKind,
    range: Range,
    container_name: Option<String>,
}

/// Symbol location reference
#[derive(Debug, Clone)]
struct SymbolLocation {
    uri: String,
    symbol_index: usize,
}

impl WorkspaceSymbolsProvider {
    /// Create a new workspace symbols provider
    pub fn new() -> Self {
        Self {
            index: Arc::new(RwLock::new(SymbolIndex::new())),
            include_locals: false,
        }
    }
    
    /// Search for symbols matching the query
    pub fn search(&self, query: &str) -> Vec<WorkspaceSymbol> {
        let index = self.index.read().unwrap();
        let mut results = Vec::new();
        
        // Simple substring search for now
        // TODO: Implement fuzzy matching
        let query_lower = query.to_lowercase();
        
        for (uri, symbols) in &index.documents {
            for (idx, symbol) in symbols.iter().enumerate() {
                if symbol.name.to_lowercase().contains(&query_lower) {
                    results.push(WorkspaceSymbol {
                        name: symbol.name.clone(),
                        kind: symbol.kind.clone(),
                        uri: uri.clone(),
                        range: symbol.range.clone(),
                        container_name: symbol.container_name.clone(),
                    });
                }
            }
        }
        
        // Sort by relevance (exact matches first, then by name)
        results.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;
            
            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        
        results
    }
    
    /// Convert AST span to LSP range
    fn span_to_range(content: &str, start: usize, end: usize) -> Range {
        let mut line = 0;
        let mut character = 0;
        let mut start_pos = None;
        let mut end_pos = None;
        
        for (idx, ch) in content.char_indices() {
            if idx == start {
                start_pos = Some(Position { line, character });
            }
            if idx == end {
                end_pos = Some(Position { line, character });
                break;
            }
            
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }
        
        Range {
            start: start_pos.unwrap_or(Position { line: 0, character: 0 }),
            end: end_pos.unwrap_or(Position { line, character }),
        }
    }
}

impl FeatureProvider for WorkspaceSymbolsProvider {
    fn name(&self) -> &'static str {
        "workspaceSymbols"
    }
    
    fn initialize(&mut self, params: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
        // Check initialization options
        if let Some(options) = params.get("initializationOptions") {
            if let Some(include_locals) = options.get("includeLocalSymbols").and_then(|v| v.as_bool()) {
                self.include_locals = include_locals;
            }
        }
        Ok(())
    }
}

impl WorkspaceFeatureProvider for WorkspaceSymbolsProvider {
    fn index_document(&mut self, uri: &str, ast: &Node) {
        let mut index = self.index.write().unwrap();
        
        // Extract symbols from AST
        let extractor = SymbolExtractor::new();
        let symbols = extractor.extract(ast);
        
        // Convert to indexed symbols
        let mut indexed_symbols = Vec::new();
        let mut container_stack: Vec<String> = Vec::new();
        
        for symbol in symbols {
            // Skip local variables if not included
            if !self.include_locals && matches!(symbol.kind, SymbolKind::Variable) {
                if symbol.name.starts_with("my ") || symbol.name.starts_with("local ") {
                    continue;
                }
            }
            
            // Track container names for nested symbols
            let container_name = if !container_stack.is_empty() {
                Some(container_stack.join("::"))
            } else {
                None
            };
            
            indexed_symbols.push(IndexedSymbol {
                name: symbol.name.clone(),
                kind: symbol.kind.clone(),
                range: Self::span_to_range("", symbol.range.start, symbol.range.end),
                container_name,
            });
            
            // Update container stack for packages and subs
            match symbol.kind {
                SymbolKind::Module | SymbolKind::Class => {
                    container_stack.clear();
                    container_stack.push(symbol.name.clone());
                }
                SymbolKind::Function | SymbolKind::Method => {
                    if symbol.name != "ANON" {
                        let current_len = container_stack.len();
                        if current_len > 1 {
                            container_stack.truncate(1);
                        }
                        container_stack.push(symbol.name.clone());
                    }
                }
                _ => {}
            }
        }
        
        // Update document index
        index.documents.insert(uri.to_string(), indexed_symbols);
        
        // Rebuild name index
        index.rebuild_name_index();
    }
    
    fn remove_document(&mut self, uri: &str) {
        let mut index = self.index.write().unwrap();
        index.documents.remove(uri);
        index.rebuild_name_index();
    }
    
    fn clear(&mut self) {
        let mut index = self.index.write().unwrap();
        index.documents.clear();
        index.name_index.clear();
    }
}

impl SymbolIndex {
    fn new() -> Self {
        Self {
            documents: HashMap::new(),
            name_index: HashMap::new(),
        }
    }
    
    fn rebuild_name_index(&mut self) {
        self.name_index.clear();
        
        for (uri, symbols) in &self.documents {
            for (idx, symbol) in symbols.iter().enumerate() {
                self.name_index
                    .entry(symbol.name.clone())
                    .or_insert_with(Vec::new)
                    .push(SymbolLocation {
                        uri: uri.clone(),
                        symbol_index: idx,
                    });
            }
        }
    }
}

impl Default for WorkspaceSymbolsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    
    #[test]
    fn test_workspace_symbol_search() {
        let mut provider = WorkspaceSymbolsProvider::new();
        
        // Index a test document
        let code = r#"
package TestModule;

sub test_function {
    my $var = 42;
    return $var;
}

sub another_test {
    # ...
}
"#;
        
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();
        
        provider.index_document("test.pl", &ast);
        
        // Search for "test"
        let results = provider.search("test");
        assert_eq!(results.len(), 3); // TestModule, test_function, another_test
        
        // Search for "var" (should be empty if include_locals is false)
        let var_results = provider.search("var");
        assert_eq!(var_results.len(), 0);
    }
}