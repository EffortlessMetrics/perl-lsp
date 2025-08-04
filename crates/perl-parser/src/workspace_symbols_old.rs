//! Workspace symbols provider for LSP
//!
//! Provides workspace/symbol functionality for searching symbols across all files.

use crate::{
    ast::Node,
    symbol::{Symbol, SymbolKind, SymbolExtractor},
    SourceLocation,
};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// LSP WorkspaceSymbol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: i32,
    pub location: Location,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
}

/// LSP Location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
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
    /// Map of file URI to symbols
    symbols: HashMap<String, Vec<Symbol>>,
}

impl WorkspaceSymbolsProvider {
    /// Create a new workspace symbols provider
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }
    
    /// Index a document's symbols
    pub fn index_document(&mut self, uri: &str, ast: &Node) {
        let extractor = SymbolExtractor::new();
        let symbols = extractor.extract(ast);
        self.symbols.insert(uri.to_string(), symbols);
    }
    
    /// Remove a document from the index
    pub fn remove_document(&mut self, uri: &str) {
        self.symbols.remove(uri);
    }
    
    /// Search for symbols matching a query
    pub fn search(&self, query: &str) -> Vec<WorkspaceSymbol> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        
        for (uri, symbols) in &self.symbols {
            for symbol in symbols {
                if self.matches_query(&symbol.name, &query_lower) {
                    results.push(self.symbol_to_workspace_symbol(uri, symbol));
                }
            }
        }
        
        // Sort by relevance (exact matches first, then prefix matches, then contains)
        results.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;
            let a_prefix = a.name.to_lowercase().starts_with(&query_lower);
            let b_prefix = b.name.to_lowercase().starts_with(&query_lower);
            
            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match (a_prefix, b_prefix) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            }
        });
        
        results
    }
    
    /// Check if a symbol name matches the query
    fn matches_query(&self, name: &str, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        
        let name_lower = name.to_lowercase();
        
        // Exact match
        if name_lower == query {
            return true;
        }
        
        // Prefix match
        if name_lower.starts_with(query) {
            return true;
        }
        
        // Contains match
        if name_lower.contains(query) {
            return true;
        }
        
        // Fuzzy match (simple version - each query char appears in order)
        let mut query_chars = query.chars();
        let mut current_char = query_chars.next();
        
        for ch in name_lower.chars() {
            if let Some(qch) = current_char {
                if ch == qch {
                    current_char = query_chars.next();
                }
            } else {
                return true; // All query chars found
            }
        }
        
        current_char.is_none()
    }
    
    /// Convert internal Symbol to LSP WorkspaceSymbol
    fn symbol_to_workspace_symbol(&self, uri: &str, symbol: &Symbol) -> WorkspaceSymbol {
        WorkspaceSymbol {
            name: symbol.name.clone(),
            kind: self.symbol_kind_to_lsp(symbol.kind),
            location: Location {
                uri: uri.to_string(),
                range: Range {
                    start: Position {
                        line: symbol.location.start_line.saturating_sub(1),
                        character: symbol.location.start_column.saturating_sub(1),
                    },
                    end: Position {
                        line: symbol.location.end_line.saturating_sub(1),
                        character: symbol.location.end_column.saturating_sub(1),
                    },
                },
            },
            container_name: None, // TODO: Add container tracking
        }
    }
    
    /// Convert internal SymbolKind to LSP symbol kind
    fn symbol_kind_to_lsp(&self, kind: SymbolKind) -> i32 {
        match kind {
            SymbolKind::Package => 4,     // Namespace
            SymbolKind::Subroutine => 12, // Function
            SymbolKind::Variable => 13,   // Variable
            SymbolKind::Constant => 14,   // Constant
            SymbolKind::Class => 5,       // Class
            SymbolKind::Method => 6,      // Method
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    
    #[test]
    fn test_workspace_symbols_search() {
        let mut provider = WorkspaceSymbolsProvider::new();
        
        // Index a test file
        let code = r#"
package MyPackage;

sub foo {
    my $x = 42;
}

sub foobar {
    my $y = 'test';
}

sub baz {
    # Another function
}
"#;
        
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();
        
        provider.index_document("file:///test.pl", &ast);
        
        // Test exact match
        let results = provider.search("foo");
        assert_eq!(results.len(), 2); // foo and foobar
        assert_eq!(results[0].name, "foo"); // Exact match first
        
        // Test prefix match
        let results = provider.search("fo");
        assert_eq!(results.len(), 2);
        
        // Test contains match
        let results = provider.search("bar");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "foobar");
        
        // Test fuzzy match
        let results = provider.search("fb");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "foobar");
        
        // Test empty query (returns all)
        let results = provider.search("");
        assert!(results.len() >= 3); // At least package, foo, foobar, baz
    }
    
    #[test]
    fn test_workspace_symbols_multiple_files() {
        let mut provider = WorkspaceSymbolsProvider::new();
        
        // Index first file
        let code1 = "sub test_one { }";
        let mut parser1 = Parser::new(code1);
        let ast1 = parser1.parse().unwrap();
        provider.index_document("file:///test1.pl", &ast1);
        
        // Index second file
        let code2 = "sub test_two { }";
        let mut parser2 = Parser::new(code2);
        let ast2 = parser2.parse().unwrap();
        provider.index_document("file:///test2.pl", &ast2);
        
        // Search across both files
        let results = provider.search("test");
        assert_eq!(results.len(), 2);
        
        // Remove one file
        provider.remove_document("file:///test1.pl");
        let results = provider.search("test");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_two");
    }
}