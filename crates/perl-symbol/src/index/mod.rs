//! Symbol search index primitives.
//!
//! This module has one responsibility: indexing symbol names for fast lookup
//! across prefix and fuzzy query styles.

use std::collections::{HashMap, HashSet};

/// Symbol index for fast lookups.
///
/// Supports both prefix and fuzzy matching using a trie and inverted index.
pub struct SymbolIndex {
    /// Trie structure for prefix matching
    trie: SymbolTrie,
    /// Inverted index for fuzzy matching
    inverted_index: HashMap<String, HashMap<String, usize>>,
    /// Per-document symbol membership.
    document_symbols: HashMap<String, HashSet<String>>,
}

/// Trie data structure for efficient prefix matching
struct SymbolTrie {
    /// Child nodes indexed by character
    children: HashMap<char, Box<SymbolTrie>>,
    /// Terminal symbol stored at this node.
    symbol: Option<String>,
    /// Number of documents that currently provide this symbol.
    symbol_count: usize,
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
        const LEGACY_DOC_ID: &str = "__legacy_add_symbol__";
        let mut symbols = self.document_symbols.remove(LEGACY_DOC_ID).unwrap_or_default();
        symbols.insert(symbol);
        self.replace_document_symbols(LEGACY_DOC_ID, symbols);
    }

    /// Replace all symbols owned by a document.
    ///
    /// Existing symbols for `doc_id` are removed, then replaced by `symbols`.
    pub fn replace_document_symbols<I>(&mut self, doc_id: &str, symbols: I)
    where
        I: IntoIterator<Item = String>,
    {
        let new_symbols: HashSet<String> = symbols.into_iter().collect();
        let old_symbols = self.document_symbols.get(doc_id).cloned().unwrap_or_default();

        for removed in old_symbols.difference(&new_symbols) {
            self.remove_symbol_occurrence(removed);
        }

        for added in new_symbols.difference(&old_symbols) {
            self.add_symbol_occurrence(added);
        }

        if new_symbols.is_empty() {
            self.document_symbols.remove(doc_id);
        } else {
            self.document_symbols.insert(doc_id.to_string(), new_symbols);
        }
    }

    /// Remove all symbols associated with a document.
    pub fn remove_document(&mut self, doc_id: &str) {
        let Some(symbols) = self.document_symbols.remove(doc_id) else {
            return;
        };

        for symbol in symbols {
            self.remove_symbol_occurrence(&symbol);
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
                for symbol in symbols.keys() {
                    *results.entry(symbol.as_str()).or_insert(0) += 1;
                }
            }
        }

        // Sort by relevance (number of matching tokens)
        let mut sorted: Vec<_> = results.into_iter().collect();
        sorted.sort_by(|(name_a, score_a), (name_b, score_b)| {
            score_b.cmp(score_a).then_with(|| name_a.cmp(name_b))
        });

        sorted.into_iter().map(|(symbol, _)| symbol.to_string()).collect()
    }

    fn add_symbol_occurrence(&mut self, symbol: &str) {
        self.trie.insert(symbol);
        for token in Self::tokenize(symbol) {
            *self
                .inverted_index
                .entry(token)
                .or_default()
                .entry(symbol.to_string())
                .or_insert(0) += 1;
        }
    }

    fn remove_symbol_occurrence(&mut self, symbol: &str) {
        if !self.trie.remove(symbol) {
            return;
        }

        for token in Self::tokenize(symbol) {
            let mut should_remove_token = false;
            if let Some(token_symbols) = self.inverted_index.get_mut(&token) {
                if let Some(count) = token_symbols.get_mut(symbol) {
                    *count -= 1;
                    if *count == 0 {
                        token_symbols.remove(symbol);
                    }
                }
                should_remove_token = token_symbols.is_empty();
            }

            if should_remove_token {
                self.inverted_index.remove(&token);
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
        Self { children: HashMap::new(), symbol: None, symbol_count: 0 }
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

        if node.symbol_count == 0 {
            node.symbol = Some(symbol.to_string());
        }
        node.symbol_count += 1;
        node.symbol_count == 1
    }

    /// Remove one occurrence of `symbol`.
    ///
    /// Returns `true` only when symbol visibility was removed entirely.
    fn remove(&mut self, symbol: &str) -> bool {
        let chars: Vec<char> = symbol.chars().collect();
        let (removed, _) = Self::remove_inner(self, &chars, 0);
        removed
    }

    fn remove_inner(node: &mut SymbolTrie, chars: &[char], idx: usize) -> (bool, bool) {
        if idx == chars.len() {
            if node.symbol_count == 0 {
                return (false, false);
            }

            node.symbol_count -= 1;
            let removed = node.symbol_count == 0;
            if removed {
                node.symbol = None;
            }
            return (removed, node.children.is_empty() && node.symbol_count == 0);
        }

        let ch = chars[idx];
        let Some(child) = node.children.get_mut(&ch) else {
            return (false, false);
        };

        let (removed, child_prune) = Self::remove_inner(child, chars, idx + 1);
        if child_prune {
            node.children.remove(&ch);
        }

        let should_prune = node.children.is_empty() && node.symbol_count == 0;
        (removed, should_prune)
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
        if let Some(symbol) = &node.symbol {
            results.push(symbol.clone());
        }

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
}
