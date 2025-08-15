//! Workspace symbols provider for LSP
//!
//! Provides workspace/symbol functionality for searching symbols across all files.

use crate::{
    SourceLocation,
    ast::Node,
    symbol::{SymbolExtractor, SymbolKind},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Internal symbol info
#[derive(Debug, Clone)]
struct SymbolInfo {
    name: String,
    kind: SymbolKind,
    location: SourceLocation,
    container: Option<String>,
}

/// Workspace symbols provider
pub struct WorkspaceSymbolsProvider {
    /// Map of file URI to symbols
    documents: HashMap<String, Vec<SymbolInfo>>,
}

impl Default for WorkspaceSymbolsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSymbolsProvider {
    /// Create a new workspace symbols provider
    pub fn new() -> Self {
        Self { documents: HashMap::new() }
    }

    /// Index a document's symbols
    pub fn index_document(&mut self, uri: &str, ast: &Node, _source: &str) {
        let extractor = SymbolExtractor::new();
        let table = extractor.extract(ast);

        let mut symbols = Vec::new();

        // Extract symbols from the symbol table
        for (name, symbol_list) in &table.symbols {
            for symbol in symbol_list {
                symbols.push(SymbolInfo {
                    name: name.clone(),
                    kind: symbol.kind,
                    location: symbol.location,
                    container: None, // TODO: Track containing package/class
                });
            }
        }

        self.documents.insert(uri.to_string(), symbols);
    }

    /// Remove a document from the index
    pub fn remove_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Get all symbols (for indexing)
    pub fn get_all_symbols(&self) -> Vec<WorkspaceSymbol> {
        let mut all_symbols = Vec::new();

        for (uri, symbols) in &self.documents {
            for symbol in symbols {
                // Create a minimal workspace symbol for indexing
                all_symbols.push(WorkspaceSymbol {
                    name: symbol.name.clone(),
                    kind: symbol_kind_to_lsp(&symbol.kind),
                    location: Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: 0 },
                        },
                    },
                    container_name: symbol.container.clone(),
                });
            }
        }

        all_symbols
    }

    /// Search for symbols matching a query
    /// Search with pre-filtered candidate names for better performance
    pub fn search_with_candidates(
        &self,
        query: &str,
        source_map: &HashMap<String, String>,
        candidates: &[String],
    ) -> Vec<WorkspaceSymbol> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        // Create a set of candidate names for fast lookup
        let candidate_set: std::collections::HashSet<_> =
            candidates.iter().map(|s| s.to_lowercase()).collect();

        for (uri, symbols) in &self.documents {
            // Get source for this document to convert offsets
            let source = match source_map.get(uri) {
                Some(s) => s,
                None => continue,
            };

            for symbol in symbols {
                // Check if this symbol is in our candidate set
                if candidate_set.contains(&symbol.name.to_lowercase())
                    && self.matches_query(&symbol.name, &query_lower)
                {
                    results.push(self.symbol_to_workspace_symbol(uri, symbol, source));
                }
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;

            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    let a_starts = a.name.to_lowercase().starts_with(&query_lower);
                    let b_starts = b.name.to_lowercase().starts_with(&query_lower);

                    match (a_starts, b_starts) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                }
            }
        });

        results
    }

    pub fn search(
        &self,
        query: &str,
        source_map: &HashMap<String, String>,
    ) -> Vec<WorkspaceSymbol> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (uri, symbols) in &self.documents {
            // Get source for this document to convert offsets
            let source = match source_map.get(uri) {
                Some(s) => s,
                None => continue,
            };

            for symbol in symbols {
                if self.matches_query(&symbol.name, &query_lower) {
                    results.push(self.symbol_to_workspace_symbol(uri, symbol, source));
                }
            }
        }

        // Sort by relevance
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
                },
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

        // Simple fuzzy match
        let mut query_chars = query.chars();
        let mut current_char = query_chars.next();

        for ch in name_lower.chars() {
            if let Some(qch) = current_char {
                if ch == qch {
                    current_char = query_chars.next();
                }
            } else {
                return true;
            }
        }

        current_char.is_none()
    }

    /// Convert internal Symbol to LSP WorkspaceSymbol
    fn symbol_to_workspace_symbol(
        &self,
        uri: &str,
        symbol: &SymbolInfo,
        source: &str,
    ) -> WorkspaceSymbol {
        let (start_line, start_col) = offset_to_line_col(source, symbol.location.start);
        let (end_line, end_col) = offset_to_line_col(source, symbol.location.end);

        WorkspaceSymbol {
            name: symbol.name.clone(),
            kind: symbol_kind_to_lsp(&symbol.kind),
            location: Location {
                uri: uri.to_string(),
                range: Range {
                    start: Position { line: start_line as u32, character: start_col as u32 },
                    end: Position { line: end_line as u32, character: end_col as u32 },
                },
            },
            container_name: symbol.container.clone(),
        }
    }
}

/// Convert internal SymbolKind to LSP symbol kind
fn symbol_kind_to_lsp(kind: &SymbolKind) -> i32 {
    match kind {
        SymbolKind::Package => 4,         // Namespace
        SymbolKind::Subroutine => 12,     // Function
        SymbolKind::ScalarVariable => 13, // Variable
        SymbolKind::ArrayVariable => 13,  // Variable
        SymbolKind::HashVariable => 13,   // Variable
        SymbolKind::Constant => 14,       // Constant
        SymbolKind::Label => 15,          // String
        SymbolKind::Format => 23,         // Struct
    }
}

/// Convert byte offset to line/column position
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    let mut byte_pos = 0;

    for ch in source.chars() {
        if byte_pos >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }

        byte_pos += ch.len_utf8();
    }

    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[test]
    fn test_workspace_symbols_search() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        // Index a test file
        let source = r#"
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

        source_map.insert("file:///test.pl".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        provider.index_document("file:///test.pl", &ast, source);

        // Test exact match
        let results = provider.search("foo", &source_map);
        assert_eq!(results.len(), 2); // foo and foobar
        assert_eq!(results[0].name, "foo"); // Exact match first

        // Test prefix match
        let results = provider.search("fo", &source_map);
        assert_eq!(results.len(), 2);

        // Test contains match
        let results = provider.search("bar", &source_map);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "foobar");

        // Test fuzzy match
        let results = provider.search("fb", &source_map);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "foobar");
    }

    #[test]
    fn test_offset_to_line_col() {
        let source = "hello\nworld\n123";

        assert_eq!(offset_to_line_col(source, 0), (0, 0)); // 'h'
        assert_eq!(offset_to_line_col(source, 5), (0, 5)); // '\n'
        assert_eq!(offset_to_line_col(source, 6), (1, 0)); // 'w'
        assert_eq!(offset_to_line_col(source, 11), (1, 5)); // '\n'
        assert_eq!(offset_to_line_col(source, 12), (2, 0)); // '1'
    }
}
