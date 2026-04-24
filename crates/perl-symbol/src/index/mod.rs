//! Symbol search index primitives.
//!
//! This module has one responsibility: indexing symbol names for fast lookup
//! across prefix and fuzzy query styles.

use std::collections::{HashMap, HashSet};

/// Symbol index for fast lookups.
///
/// Supports both prefix and fuzzy matching using a trie and inverted index.
pub struct SymbolIndex {
    /// Trie structure for prefix matching.
    trie: SymbolTrie,
    /// Inverted index for fuzzy matching.
    ///
    /// token -> (symbol -> refcount)
    inverted_index: HashMap<String, HashMap<String, usize>>,
    /// Global symbol refcounts across all sources/documents.
    symbol_refcounts: HashMap<String, usize>,
    /// Per-document symbol membership for replace/remove operations.
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
            symbol_refcounts: HashMap::new(),
            document_symbols: HashMap::new(),
        }
    }

    /// Add a symbol to the index.
    ///
    /// Indexes the symbol for both prefix and fuzzy matching.
    /// Duplicate calls with the same symbol are idempotent from a search-output
    /// perspective: matches return each symbol name at most once.
    pub fn add_symbol(&mut self, symbol: String) {
        self.increment_symbol(symbol);
    }

    /// Replace all symbols associated with a document.
    ///
    /// This is the primary mutation API for incremental indexing: reindexing the
    /// same document removes stale names and inserts the new set atomically from
    /// the caller's perspective.
    pub fn set_document_symbols<I>(&mut self, document_id: &str, symbols: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.remove_document(document_id);

        let mut seen = HashSet::new();
        let deduped: Vec<String> = symbols.into_iter().filter(|s| seen.insert(s.clone())).collect();

        for symbol in &deduped {
            self.increment_symbol(symbol.clone());
        }

        if !deduped.is_empty() {
            self.document_symbols.insert(document_id.to_string(), deduped);
        }
    }

    /// Remove all symbols for a document.
    pub fn remove_document(&mut self, document_id: &str) {
        if let Some(symbols) = self.document_symbols.remove(document_id) {
            for symbol in symbols {
                self.decrement_symbol(&symbol);
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
                for symbol in symbols.keys() {
                    *results.entry(symbol.clone()).or_insert(0) += 1;
                }
            }
        }

        // Sort by relevance (number of matching tokens)
        let mut sorted: Vec<_> = results.into_iter().collect();
        sorted.sort_by(|(_, a), (_, b)| b.cmp(a));

        sorted.into_iter().map(|(symbol, _)| symbol).collect()
    }

    fn increment_symbol(&mut self, symbol: String) {
        let count = self.symbol_refcounts.entry(symbol.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            return;
        }

        self.trie.insert(&symbol);
        let tokens = Self::tokenize(&symbol);
        for token in tokens {
            let bucket = self.inverted_index.entry(token).or_default();
            *bucket.entry(symbol.clone()).or_insert(0) += 1;
        }
    }

    fn decrement_symbol(&mut self, symbol: &str) {
        let Some(count) = self.symbol_refcounts.get_mut(symbol) else {
            return;
        };

        *count = count.saturating_sub(1);
        if *count > 0 {
            return;
        }

        self.symbol_refcounts.remove(symbol);
        self.trie.remove(symbol);

        for token in Self::tokenize(symbol) {
            if let Some(bucket) = self.inverted_index.get_mut(&token) {
                bucket.remove(symbol);
                if bucket.is_empty() {
                    self.inverted_index.remove(&token);
                }
            }
        }
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
    fn insert(&mut self, symbol: &str) {
        let mut node = self;

        for ch in symbol.chars() {
            node = node.children.entry(ch).or_insert_with(|| Box::new(SymbolTrie::new()));
        }

        let owned = symbol.to_string();
        if !node.symbols.contains(&owned) {
            node.symbols.push(owned);
        }
    }

    fn remove(&mut self, symbol: &str) {
        let chars: Vec<char> = symbol.chars().collect();
        Self::remove_recursive(self, &chars, 0, symbol);
    }

    fn remove_recursive(node: &mut SymbolTrie, chars: &[char], depth: usize, symbol: &str) -> bool {
        if depth == chars.len() {
            node.symbols.retain(|existing| existing != symbol);
        } else if let Some(child) = node.children.get_mut(&chars[depth]) {
            if Self::remove_recursive(child, chars, depth + 1, symbol) {
                node.children.remove(&chars[depth]);
            }
        }

        node.children.is_empty() && node.symbols.is_empty()
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
    fn replacing_document_symbols_removes_stale_entries() {
        let mut index = SymbolIndex::new();

        index.set_document_symbols("file:///a.pl", ["alpha".to_string(), "beta".to_string()]);
        assert!(index.search_prefix("al").contains(&"alpha".to_string()));

        index.set_document_symbols("file:///a.pl", ["gamma".to_string()]);
        assert!(index.search_prefix("al").is_empty());
        assert!(index.search_prefix("ga").contains(&"gamma".to_string()));
    }

    #[test]
    fn remove_document_preserves_symbols_still_referenced_elsewhere() {
        let mut index = SymbolIndex::new();

        index.set_document_symbols("file:///a.pl", ["shared".to_string()]);
        index.set_document_symbols("file:///b.pl", ["shared".to_string()]);

        index.remove_document("file:///a.pl");
        assert!(index.search_prefix("sha").contains(&"shared".to_string()));

        index.remove_document("file:///b.pl");
        assert!(index.search_prefix("sha").is_empty());
    }
}
