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

/// Normalize legacy package separator ' to ::
fn norm_pkg(s: &str) -> String {
    if s.contains('\'') { s.replace('\'', "::") } else { s.to_string() }
}

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

/// Match type for ranking search results
#[derive(Debug, Clone, Copy)]
enum MatchType {
    Exact,
    Prefix,
    Contains,
    Fuzzy,
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
    pub fn index_document(&mut self, uri: &str, ast: &Node, source: &str) {
        let extractor = SymbolExtractor::new_with_source(source);
        let table = extractor.extract(ast);

        let mut symbols = Vec::new();

        // Extract symbols from the symbol table
        for symbol_list in table.symbols.values() {
            for symbol in symbol_list {
                let container =
                    symbol.qualified_name.rsplit_once("::").map(|(pkg, _)| pkg.to_string());

                symbols.push(SymbolInfo {
                    name: symbol.name.clone(),
                    kind: symbol.kind,
                    location: symbol.location,
                    container,
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
                    container_name: symbol.container.as_ref().map(|s| norm_pkg(s)),
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
        self.search_with_limit(query, source_map, 1000) // Default limit for performance
    }

    /// Search with result limit for better performance
    pub fn search_with_limit(
        &self,
        query: &str,
        source_map: &HashMap<String, String>,
        limit: usize,
    ) -> Vec<WorkspaceSymbol> {
        let query_lower = query.to_lowercase();
        let mut exact_matches = Vec::new();
        let mut prefix_matches = Vec::new();
        let mut contains_matches = Vec::new();
        let mut fuzzy_matches = Vec::new();

        let mut total_processed = 0;
        const MAX_PROCESS: usize = 10000; // Limit processing for performance

        'documents: for (uri, symbols) in &self.documents {
            // Get source for this document to convert offsets
            let source = match source_map.get(uri) {
                Some(s) => s,
                None => continue,
            };

            for (i, symbol) in symbols.iter().enumerate() {
                // Cooperative yield every 32 symbols
                if i & 0x1f == 0 {
                    std::thread::yield_now();
                }

                total_processed += 1;
                if total_processed >= MAX_PROCESS {
                    break 'documents;
                }

                let match_result = self.classify_match(&symbol.name, &query_lower);
                if let Some(match_type) = match_result {
                    let workspace_symbol = self.symbol_to_workspace_symbol(uri, symbol, source);

                    match match_type {
                        MatchType::Exact => {
                            exact_matches.push(workspace_symbol);
                            // Stop early if we have enough exact matches
                            if exact_matches.len() >= limit {
                                break 'documents;
                            }
                        }
                        MatchType::Prefix => prefix_matches.push(workspace_symbol),
                        MatchType::Contains => contains_matches.push(workspace_symbol),
                        MatchType::Fuzzy => fuzzy_matches.push(workspace_symbol),
                    }

                    // Stop early if we have collected enough results
                    let total_found = exact_matches.len()
                        + prefix_matches.len()
                        + contains_matches.len()
                        + fuzzy_matches.len();
                    if total_found >= limit * 2 {
                        break 'documents;
                    }
                }
            }
        }

        // Combine results in order of relevance, respecting limit
        let mut results = Vec::new();
        results.extend(exact_matches.into_iter().take(limit));
        let remaining = limit.saturating_sub(results.len());

        if remaining > 0 {
            results.extend(prefix_matches.into_iter().take(remaining));
            let remaining = limit.saturating_sub(results.len());

            if remaining > 0 {
                results.extend(contains_matches.into_iter().take(remaining));
                let remaining = limit.saturating_sub(results.len());

                if remaining > 0 {
                    results.extend(fuzzy_matches.into_iter().take(remaining));
                }
            }
        }

        // Sort within each category by name for consistency
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }

    /// Classify match type for better ranking
    fn classify_match(&self, name: &str, query: &str) -> Option<MatchType> {
        if query.is_empty() {
            return Some(MatchType::Contains); // Return all symbols for empty query
        }

        let name_lower = name.to_lowercase();

        // Exact match (highest priority)
        if name_lower == query {
            return Some(MatchType::Exact);
        }

        // Prefix match (high priority)
        if name_lower.starts_with(query) {
            return Some(MatchType::Prefix);
        }

        // Contains match (medium priority)
        if name_lower.contains(query) {
            return Some(MatchType::Contains);
        }

        // Simple fuzzy match (lowest priority)
        if self.fuzzy_matches(&name_lower, query) {
            return Some(MatchType::Fuzzy);
        }

        None
    }

    /// Check for fuzzy match
    fn fuzzy_matches(&self, name: &str, query: &str) -> bool {
        let mut query_chars = query.chars();
        let mut current_char = query_chars.next();

        for ch in name.chars() {
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

    /// Check if a symbol name matches the query (legacy method)
    fn matches_query(&self, name: &str, query: &str) -> bool {
        self.classify_match(name, query).is_some()
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
            container_name: symbol.container.as_ref().map(|s| norm_pkg(s)),
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

        // Verify container information is indexed
        let all_symbols = provider.get_all_symbols();
        let pkg = all_symbols.iter().find(|s| s.name == "MyPackage").unwrap();
        assert!(pkg.container_name.is_none());
        let foo = all_symbols.iter().find(|s| s.name == "foo").unwrap();
        assert_eq!(foo.container_name.as_deref(), Some("MyPackage"));

        // Test exact match
        let results = provider.search("foo", &source_map);
        assert_eq!(results.len(), 2); // foo and foobar
        assert_eq!(results[0].name, "foo"); // Exact match first
        assert_eq!(results[0].container_name.as_deref(), Some("MyPackage"));

        // Test prefix match
        let results = provider.search("fo", &source_map);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|s| s.container_name.as_deref() == Some("MyPackage")));

        // Test contains match
        let results = provider.search("bar", &source_map);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "foobar");
        assert_eq!(results[0].container_name.as_deref(), Some("MyPackage"));

        // Test fuzzy match
        let results = provider.search("fb", &source_map);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "foobar");
        assert_eq!(results[0].container_name.as_deref(), Some("MyPackage"));
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
