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
    inverted_index: HashMap<String, HashSet<String>>,
    /// Reference count for each symbol across all indexed sources.
    symbol_refs: HashMap<String, usize>,
    /// Per-document symbols used by replace/remove operations.
    document_symbols: HashMap<String, Vec<String>>,
}

/// Trie data structure for efficient prefix matching.
struct SymbolTrie {
    /// Child nodes indexed by character.
    children: HashMap<char, SymbolTrie>,
    /// Whether this node marks a complete symbol.
    terminal: bool,
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
            symbol_refs: HashMap::new(),
            document_symbols: HashMap::new(),
        }
    }

    /// Add a symbol to the index.
    ///
    /// Indexes the symbol for both prefix and fuzzy matching.
    /// Duplicate calls with the same symbol are idempotent.
    pub fn add_symbol(&mut self, symbol: String) {
        if self.symbol_refs.contains_key(&symbol) {
            return;
        }

        self.add_symbol_ref(&symbol);
    }

    /// Replace all symbols indexed for one document/source id.
    pub fn replace_document_symbols(
        &mut self,
        doc_id: impl Into<String>,
        symbols: impl IntoIterator<Item = String>,
    ) {
        let doc_id = doc_id.into();

        let next_symbols_set: HashSet<_> = symbols.into_iter().collect();
        let next_symbols: Vec<_> = next_symbols_set.iter().cloned().collect();

        let prev_symbols = self.document_symbols.insert(doc_id, next_symbols).unwrap_or_default();
        let prev_symbols_set: HashSet<_> = prev_symbols.into_iter().collect();

        for symbol in prev_symbols_set.difference(&next_symbols_set) {
            self.remove_symbol_ref(symbol);
        }

        for symbol in next_symbols_set.difference(&prev_symbols_set) {
            self.add_symbol_ref(symbol);
        }
    }

    /// Remove all symbols for one document/source id.
    pub fn remove_document(&mut self, doc_id: &str) {
        let Some(symbols) = self.document_symbols.remove(doc_id) else {
            return;
        };

        let symbols_set: HashSet<_> = symbols.into_iter().collect();
        for symbol in symbols_set {
            self.remove_symbol_ref(&symbol);
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
        let tokens_set: HashSet<_> = Self::tokenize(query).into_iter().collect();
        let mut results: HashMap<&String, usize> = HashMap::new();

        for token in tokens_set {
            if let Some(symbols) = self.inverted_index.get(&token) {
                for symbol in symbols {
                    *results.entry(symbol).or_insert(0) += 1;
                }
            }
        }

        // Sort by relevance (number of matching tokens)
        let mut sorted: Vec<_> = results.into_iter().collect();
        sorted.sort_by(|(_, a), (_, b)| b.cmp(a));

        sorted.into_iter().map(|(symbol, _)| symbol.clone()).collect()
    }

    fn add_symbol_ref(&mut self, symbol: &str) {
        let refs = self.symbol_refs.entry(symbol.to_string()).or_insert(0);
        *refs += 1;

        if *refs > 1 {
            return;
        }

        self.trie.insert(symbol);

        let tokens: HashSet<_> = Self::tokenize(symbol).into_iter().collect();
        for token in tokens {
            self.inverted_index.entry(token).or_default().insert(symbol.to_string());
        }
    }

    fn remove_symbol_ref(&mut self, symbol: &str) {
        let Some(refs) = self.symbol_refs.get_mut(symbol) else {
            return;
        };

        *refs -= 1;
        if *refs > 0 {
            return;
        }

        self.symbol_refs.remove(symbol);
        self.trie.remove(symbol);

        let tokens: HashSet<_> = Self::tokenize(symbol).into_iter().collect();
        for token in tokens {
            if let Some(symbols) = self.inverted_index.get_mut(&token) {
                symbols.remove(symbol);
                if symbols.is_empty() {
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
        Self { children: HashMap::new(), terminal: false }
    }

    fn insert(&mut self, symbol: &str) {
        let mut node = self;

        for ch in symbol.chars() {
            node = node.children.entry(ch).or_insert_with(SymbolTrie::new);
        }

        node.terminal = true;
    }

    fn remove(&mut self, symbol: &str) {
        let chars: Vec<_> = symbol.chars().collect();
        let _ = Self::remove_inner(self, &chars, 0);
    }

    fn remove_inner(node: &mut SymbolTrie, chars: &[char], idx: usize) -> bool {
        if idx == chars.len() {
            node.terminal = false;
        } else if let Some(child) = node.children.get_mut(&chars[idx]) {
            if Self::remove_inner(child, chars, idx + 1) {
                node.children.remove(&chars[idx]);
            }
        }

        !node.terminal && node.children.is_empty()
    }

    fn search_prefix(&self, prefix: &str) -> Vec<String> {
        let mut node = self;

        for ch in prefix.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return Vec::new(),
            }
        }

        // Collect all symbols from this node and descendants.
        let mut results = Vec::new();
        let mut current = prefix.to_string();
        Self::collect_all(node, &mut current, &mut results);
        results
    }

    fn collect_all(node: &SymbolTrie, current: &mut String, results: &mut Vec<String>) {
        if node.terminal {
            results.push(current.clone());
        }

        for (&ch, child) in &node.children {
            current.push(ch);
            Self::collect_all(child, current, results);
            let _ = current.pop();
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
