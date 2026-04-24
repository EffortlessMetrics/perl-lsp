//! Symbol search index primitives.
//!
//! This module has one responsibility: indexing symbol names for fast lookup
//! across prefix and fuzzy query styles.

use std::collections::{HashMap, HashSet};

const LEGACY_DOC_ID: &str = "__legacy_add_symbol__";

/// Symbol index for fast lookups.
///
/// Supports both prefix and fuzzy matching using a trie and inverted index.
pub struct SymbolIndex {
    /// Trie structure for prefix matching.
    trie: SymbolTrie,
    /// Inverted index for fuzzy matching.
    inverted_index: HashMap<String, HashSet<String>>,
    /// Global symbol ref-count across all documents.
    symbol_counts: HashMap<String, usize>,
    /// Per-document symbols for replacement/removal.
    document_symbols: HashMap<String, HashSet<String>>,
}

/// Trie data structure for efficient prefix matching.
struct SymbolTrie {
    /// Child nodes indexed by character.
    children: HashMap<char, Box<SymbolTrie>>,
    /// Number of live symbols ending at this node.
    terminal_count: usize,
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
            symbol_counts: HashMap::new(),
            document_symbols: HashMap::new(),
        }
    }

    /// Add a symbol to the compatibility legacy document.
    ///
    /// Repeated calls with the same symbol remain idempotent.
    pub fn add_symbol(&mut self, symbol: String) {
        self.add_symbols_to_document(LEGACY_DOC_ID, [symbol]);
    }

    /// Replace all indexed symbols for `doc_id` with `symbols`.
    pub fn replace_document_symbols(
        &mut self,
        doc_id: &str,
        symbols: impl IntoIterator<Item = String>,
    ) {
        let new_symbols: HashSet<String> = symbols.into_iter().collect();

        let old_symbols = self.document_symbols.remove(doc_id).unwrap_or_default();
        for symbol in old_symbols.difference(&new_symbols) {
            self.remove_symbol_occurrence(symbol);
        }

        for symbol in new_symbols.difference(&old_symbols) {
            self.add_symbol_occurrence(symbol);
        }

        if !new_symbols.is_empty() {
            self.document_symbols.insert(doc_id.to_string(), new_symbols);
        }
    }

    /// Remove all indexed symbols for `doc_id`.
    pub fn remove_document(&mut self, doc_id: &str) {
        if let Some(symbols) = self.document_symbols.remove(doc_id) {
            for symbol in symbols {
                self.remove_symbol_occurrence(&symbol);
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
        let mut results: HashMap<&str, usize> = HashMap::new();

        for token in tokens {
            if let Some(symbols) = self.inverted_index.get(&token) {
                for symbol in symbols {
                    *results.entry(symbol.as_str()).or_insert(0) += 1;
                }
            }
        }

        // Sort by relevance (number of matching tokens).
        let mut sorted: Vec<_> = results.into_iter().collect();
        sorted.sort_by(|(_, a), (_, b)| b.cmp(a));

        sorted.into_iter().map(|(symbol, _)| symbol.to_string()).collect()
    }

    fn add_symbols_to_document(&mut self, doc_id: &str, symbols: impl IntoIterator<Item = String>) {
        let existing = self.document_symbols.entry(doc_id.to_string()).or_default();
        let mut added = Vec::new();
        for symbol in symbols {
            if existing.insert(symbol.clone()) {
                added.push(symbol);
            }
        }

        for symbol in added {
            self.add_symbol_occurrence(&symbol);
        }
    }

    fn add_symbol_occurrence(&mut self, symbol: &str) {
        let count = self.symbol_counts.entry(symbol.to_string()).or_insert(0);
        *count += 1;

        if *count != 1 {
            return;
        }

        self.trie.insert(symbol);

        let tokens = Self::tokenize(symbol);
        for token in tokens {
            self.inverted_index.entry(token).or_default().insert(symbol.to_string());
        }
    }

    fn remove_symbol_occurrence(&mut self, symbol: &str) {
        let Some(count) = self.symbol_counts.get_mut(symbol) else {
            return;
        };

        *count -= 1;
        if *count != 0 {
            return;
        }

        self.symbol_counts.remove(symbol);
        self.trie.remove(symbol);

        let tokens = Self::tokenize(symbol);
        for token in tokens {
            let mut should_remove_token = false;
            if let Some(symbols) = self.inverted_index.get_mut(&token) {
                symbols.remove(symbol);
                should_remove_token = symbols.is_empty();
            }

            if should_remove_token {
                self.inverted_index.remove(&token);
            }
        }
    }

    fn tokenize(s: &str) -> Vec<String> {
        // Split on word boundaries and case changes.
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
        Self { children: HashMap::new(), terminal_count: 0 }
    }

    fn insert(&mut self, symbol: &str) {
        let mut node = self;

        for ch in symbol.chars() {
            node = node.children.entry(ch).or_insert_with(|| Box::new(SymbolTrie::new()));
        }

        node.terminal_count += 1;
    }

    fn remove(&mut self, symbol: &str) {
        let chars: Vec<char> = symbol.chars().collect();
        self.remove_internal(&chars, 0);
    }

    fn remove_internal(&mut self, chars: &[char], index: usize) -> bool {
        if index == chars.len() {
            if self.terminal_count > 0 {
                self.terminal_count -= 1;
            }
        } else if let Some(child) = self.children.get_mut(&chars[index]) {
            let remove_child = child.remove_internal(chars, index + 1);
            if remove_child {
                self.children.remove(&chars[index]);
            }
        }

        self.terminal_count == 0 && self.children.is_empty()
    }

    fn search_prefix(&self, prefix: &str) -> Vec<String> {
        let mut node = self;

        for ch in prefix.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return Vec::new(),
            }
        }

        let mut results = Vec::new();
        let mut symbol = prefix.to_string();
        Self::collect_all(node, &mut symbol, &mut results);
        results
    }

    fn collect_all(node: &SymbolTrie, symbol: &mut String, results: &mut Vec<String>) {
        if node.terminal_count > 0 {
            results.push(symbol.clone());
        }

        for (ch, child) in &node.children {
            symbol.push(*ch);
            Self::collect_all(child, symbol, results);
            symbol.pop();
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
}
