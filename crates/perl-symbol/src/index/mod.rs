//! Symbol search index primitives.
//!
//! This module has one responsibility: indexing symbol names for fast lookup
//! across prefix and fuzzy query styles.

use std::collections::HashMap;

/// Symbol index for fast lookups.
///
/// Supports both prefix and fuzzy matching using a trie and inverted index.
pub struct SymbolIndex {
    /// Trie structure for prefix matching
    trie: SymbolTrie,
    /// Inverted index for fuzzy matching
    inverted_index: HashMap<String, Vec<String>>,
    /// Per-document symbol membership used for replace/remove operations.
    document_symbols: HashMap<String, Vec<String>>,
}

/// Trie data structure for efficient prefix matching
struct SymbolTrie {
    /// Child nodes indexed by character
    children: HashMap<char, Box<SymbolTrie>>,
    /// Symbols stored at this node
    symbols: Vec<String>,
}

impl Default for SymbolIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolIndex {
    /// Create a new empty symbol index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            trie: SymbolTrie::new(),
            inverted_index: HashMap::new(),
            document_symbols: HashMap::new(),
        }
    }

    /// Add a symbol to the index.
    ///
    /// Indexes the symbol for both prefix and fuzzy matching.
    /// Duplicate calls with the same symbol are idempotent: the symbol is
    /// stored exactly once in both the trie and the inverted index.
    pub fn add_symbol(&mut self, symbol: String) {
        let global_doc_symbols = self.document_symbols.entry("__global__".to_string()).or_default();
        if global_doc_symbols.contains(&symbol) {
            return;
        }
        global_doc_symbols.push(symbol.clone());
        self.index_symbol(&symbol);
    }

    /// Replace all symbols for a given document.
    ///
    /// This is the primary API for live-editor indexing where each didOpen /
    /// didChange notification should overwrite prior symbol membership.
    pub fn replace_document_symbols(&mut self, document_uri: &str, symbols: Vec<String>) {
        let mut deduped = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            if !deduped.contains(&symbol) {
                deduped.push(symbol);
            }
        }
        self.document_symbols.insert(document_uri.to_string(), deduped);
        self.rebuild();
    }

    /// Remove all symbols contributed by a document.
    pub fn remove_document(&mut self, document_uri: &str) {
        if self.document_symbols.remove(document_uri).is_some() {
            self.rebuild();
        }
    }

    fn index_symbol(&mut self, symbol: &str) {
        // Add to trie for prefix matching; returns true only when newly inserted.
        // Deduplication here prevents the inverted index from accumulating
        // duplicate entries, which would inflate fuzzy-match scores.
        if !self.trie.insert(symbol) {
            return;
        }

        // Add to inverted index for fuzzy matching
        let tokens = Self::tokenize(symbol);
        for token in tokens {
            self.inverted_index.entry(token).or_default().push(symbol.to_string());
        }
    }

    fn rebuild(&mut self) {
        self.trie = SymbolTrie::new();
        self.inverted_index.clear();
        let mut seen = std::collections::HashSet::new();
        let all_symbols = self
            .document_symbols
            .values()
            .flat_map(|symbols| symbols.iter().cloned())
            .collect::<Vec<_>>();
        for symbol in all_symbols {
            if seen.insert(symbol.clone()) {
                self.index_symbol(&symbol);
            }
        }
    }

    /// Search symbols with prefix.
    ///
    /// Returns all symbols starting with the given prefix.
    #[must_use]
    pub fn search_prefix(&self, prefix: &str) -> Vec<String> {
        self.trie.search_prefix(prefix)
    }

    /// Fuzzy search symbols.
    ///
    /// Returns symbols matching any of the tokenized query words, sorted by relevance.
    #[must_use]
    pub fn search_fuzzy(&self, query: &str) -> Vec<String> {
        let tokens = Self::tokenize(query);
        let mut results = HashMap::new();

        for token in tokens {
            if let Some(symbols) = self.inverted_index.get(&token) {
                for symbol in symbols {
                    *results.entry(symbol.clone()).or_insert(0) += 1;
                }
            }
        }

        // Sort by relevance (number of matching tokens)
        let mut sorted: Vec<_> = results.into_iter().collect();
        sorted.sort_by(|(_, a), (_, b)| b.cmp(a));

        sorted.into_iter().map(|(symbol, _)| symbol).collect()
    }

    fn tokenize(s: &str) -> Vec<String> {
        // Split on word boundaries and case changes
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut prev_upper = false;

        for ch in s.chars() {
            if ch.is_uppercase() && !prev_upper && !current.is_empty() {
                tokens.push(current.to_lowercase());
                current = String::new();
            }

            if ch.is_alphanumeric() {
                current.push(ch);
                prev_upper = ch.is_uppercase();
            } else if !current.is_empty() {
                tokens.push(current.to_lowercase());
                current = String::new();
                prev_upper = false;
            }
        }

        if !current.is_empty() {
            tokens.push(current.to_lowercase());
        }

        tokens
    }
}

impl SymbolTrie {
    fn new() -> Self {
        Self { children: HashMap::new(), symbols: Vec::new() }
    }

    /// Insert `symbol` into the trie.
    ///
    /// Returns `true` if the symbol was newly inserted, `false` if it was
    /// already present (duplicate).  Callers use this to gate inverted-index
    /// updates so both structures stay in sync.
    fn insert(&mut self, symbol: &str) -> bool {
        let mut node = self;

        for ch in symbol.chars() {
            node = node.children.entry(ch).or_insert_with(|| Box::new(SymbolTrie::new()));
        }

        // Deduplicate: workspace indexing may call add_symbol for the same
        // qualified name multiple times during incremental re-index.  Storing
        // duplicates causes search_prefix to return the same entry N times,
        // which produces duplicate completions in the UI.
        let owned = symbol.to_string();
        if node.symbols.contains(&owned) {
            return false;
        }
        node.symbols.push(owned);
        true
    }

    fn search_prefix(&self, prefix: &str) -> Vec<String> {
        let mut node = self;

        for ch in prefix.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return Vec::new(),
            }
        }

        // Collect all symbols from this node and descendants
        let mut results = Vec::new();
        Self::collect_all(node, &mut results);
        results
    }

    fn collect_all(node: &SymbolTrie, results: &mut Vec<String>) {
        results.extend(node.symbols.clone());

        for child in node.children.values() {
            Self::collect_all(child, results);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SymbolIndex;

    #[test]
    fn indexes_symbols_for_prefix_and_fuzzy_search() {
        let mut index = SymbolIndex::new();

        index.add_symbol("calculate_total".to_string());
        index.add_symbol("calculateAverage".to_string());
        index.add_symbol("get_user_name".to_string());

        let prefix_results = index.search_prefix("calc");
        assert_eq!(prefix_results.len(), 2);
        assert!(prefix_results.contains(&"calculate_total".to_string()));
        assert!(prefix_results.contains(&"calculateAverage".to_string()));

        let fuzzy_results = index.search_fuzzy("user name");
        assert!(fuzzy_results.contains(&"get_user_name".to_string()));
    }

    #[test]
    fn replace_and_remove_document_symbols_evict_stale_entries() {
        let mut index = SymbolIndex::new();

        index.replace_document_symbols(
            "file:///a.pl",
            vec!["old_name".to_string(), "shared_name".to_string()],
        );
        index.replace_document_symbols("file:///b.pl", vec!["shared_name".to_string()]);
        assert!(index.search_prefix("old").contains(&"old_name".to_string()));
        assert!(index.search_prefix("shared").contains(&"shared_name".to_string()));

        index.replace_document_symbols("file:///a.pl", vec!["new_name".to_string()]);
        assert!(!index.search_prefix("old").contains(&"old_name".to_string()));
        assert!(index.search_prefix("new").contains(&"new_name".to_string()));
        assert!(index.search_prefix("shared").contains(&"shared_name".to_string()));

        index.remove_document("file:///b.pl");
        assert!(!index.search_prefix("shared").contains(&"shared_name".to_string()));
    }
}
