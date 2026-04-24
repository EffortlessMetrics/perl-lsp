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
    inverted_index: HashMap<String, HashSet<String>>,
    /// Symbols currently present for each indexed document/source.
    document_symbols: HashMap<String, HashSet<String>>,
    /// Number of documents that currently contribute each symbol.
    symbol_ref_counts: HashMap<String, usize>,
}

/// Trie data structure for efficient prefix matching
struct SymbolTrie {
    /// Child nodes indexed by character
    children: HashMap<char, Box<SymbolTrie>>,
    /// Number of symbols ending at this node.
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
            document_symbols: HashMap::new(),
            symbol_ref_counts: HashMap::new(),
        }
    }

    /// Add a symbol to the index.
    ///
    /// Indexes the symbol for both prefix and fuzzy matching.
    /// Duplicate calls with the same symbol are idempotent: the symbol is
    /// stored exactly once in both the trie and the inverted index.
    pub fn add_symbol(&mut self, symbol: String) {
        self.add_document_symbol("__legacy__", symbol);
    }

    /// Replace the full symbol set for one document/source.
    ///
    /// Any existing symbols previously associated with `doc_id` are removed
    /// first, then `symbols` are added as the new authoritative set.
    pub fn replace_document_symbols<I>(&mut self, doc_id: &str, symbols: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.remove_document(doc_id);

        for symbol in symbols {
            self.add_document_symbol(doc_id, symbol);
        }
    }

    /// Remove all symbols currently associated with one document/source.
    pub fn remove_document(&mut self, doc_id: &str) {
        let Some(symbols) = self.document_symbols.remove(doc_id) else {
            return;
        };

        for symbol in symbols {
            let Some(ref_count) = self.symbol_ref_counts.get_mut(&symbol) else {
                continue;
            };
            *ref_count = ref_count.saturating_sub(1);
            if *ref_count != 0 {
                continue;
            }

            self.symbol_ref_counts.remove(&symbol);
            self.trie.remove(&symbol);

            for token in Self::tokenize(&symbol) {
                if let Some(candidates) = self.inverted_index.get_mut(&token) {
                    candidates.remove(&symbol);
                    if candidates.is_empty() {
                        self.inverted_index.remove(&token);
                    }
                }
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
                    *results.entry(symbol.as_str()).or_insert(0) += 1;
                }
            }
        }

        // Sort by relevance (number of matching tokens)
        let mut sorted: Vec<_> = results.into_iter().collect();
        sorted.sort_by(|(symbol_a, score_a), (symbol_b, score_b)| {
            score_b.cmp(score_a).then_with(|| symbol_a.cmp(symbol_b))
        });

        sorted.into_iter().map(|(symbol, _)| symbol.to_string()).collect()
    }

    fn add_document_symbol(&mut self, doc_id: &str, symbol: String) {
        let document_entry = self.document_symbols.entry(doc_id.to_string()).or_default();
        if !document_entry.insert(symbol.clone()) {
            return;
        }

        let ref_count = self.symbol_ref_counts.entry(symbol.clone()).or_insert(0);
        *ref_count += 1;
        if *ref_count != 1 {
            return;
        }

        self.trie.insert(&symbol);
        for token in Self::tokenize(&symbol) {
            self.inverted_index.entry(token).or_default().insert(symbol.clone());
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
        Self { children: HashMap::new(), terminal_count: 0 }
    }

    /// Insert `symbol` into the trie.
    ///
    fn insert(&mut self, symbol: &str) {
        let mut node = self;

        for ch in symbol.chars() {
            node = node.children.entry(ch).or_insert_with(|| Box::new(SymbolTrie::new()));
        }
        node.terminal_count += 1;
    }

    fn remove(&mut self, symbol: &str) -> bool {
        let chars: Vec<char> = symbol.chars().collect();
        let (removed, _) = Self::remove_recursive(self, &chars, 0);
        removed
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
        let mut current = prefix.to_string();
        Self::collect_all(node, &mut current, &mut results);
        results
    }

    fn collect_all(node: &SymbolTrie, current: &mut String, results: &mut Vec<String>) {
        if node.terminal_count > 0 {
            results.push(current.clone());
        }

        for (ch, child) in &node.children {
            current.push(*ch);
            Self::collect_all(child, current, results);
            current.pop();
        }
    }

    fn remove_recursive(node: &mut SymbolTrie, chars: &[char], index: usize) -> (bool, bool) {
        if index == chars.len() {
            if node.terminal_count == 0 {
                return (false, false);
            }
            node.terminal_count -= 1;
            return (true, node.terminal_count == 0 && node.children.is_empty());
        }

        let Some(child) = node.children.get_mut(&chars[index]) else {
            return (false, false);
        };
        let (removed, prune_child) = Self::remove_recursive(child, chars, index + 1);
        if prune_child {
            node.children.remove(&chars[index]);
        }

        (removed, removed && node.terminal_count == 0 && node.children.is_empty())
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
