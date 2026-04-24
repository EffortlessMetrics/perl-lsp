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
    inverted_index: HashMap<String, Vec<String>>,
    /// Symbols currently tracked per document URI.
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
        // Add to trie for prefix matching; returns true only when newly inserted.
        // Deduplication here prevents the inverted index from accumulating
        // duplicate entries, which would inflate fuzzy-match scores.
        if !self.trie.insert(&symbol) {
            return;
        }

        // Add to inverted index for fuzzy matching
        let tokens = Self::tokenize(&symbol);
        for token in tokens {
            self.inverted_index.entry(token).or_default().push(symbol.clone());
        }
    }

    /// Replace all symbols associated with a single document.
    ///
    /// Existing symbols for `document_uri` are removed before the new symbol
    /// set is added, preventing stale matches during incremental re-index.
    pub fn index_document<I>(&mut self, document_uri: &str, symbols: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.remove_document(document_uri);

        let mut unique_symbols = HashSet::new();
        let mut stored_symbols = Vec::new();
        for symbol in symbols {
            if unique_symbols.insert(symbol.clone()) {
                self.add_symbol(symbol.clone());
                stored_symbols.push(symbol);
            }
        }

        if stored_symbols.is_empty() {
            return;
        }

        self.document_symbols.insert(document_uri.to_string(), stored_symbols);
    }

    /// Remove all symbols tracked for a document.
    ///
    /// This keeps prefix/fuzzy search results in sync when a document is
    /// deleted, closed, or re-indexed into an AST-less fallback state.
    pub fn remove_document(&mut self, document_uri: &str) {
        let Some(existing_symbols) = self.document_symbols.remove(document_uri) else {
            return;
        };

        for symbol in existing_symbols {
            if self.document_symbols.values().any(|document_list| document_list.contains(&symbol)) {
                continue;
            }

            self.trie.remove(&symbol);
            for token in Self::tokenize(&symbol) {
                if let Some(bucket) = self.inverted_index.get_mut(&token) {
                    bucket.retain(|existing| existing != &symbol);
                    if bucket.is_empty() {
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

    fn remove(&mut self, symbol: &str) {
        let chars: Vec<char> = symbol.chars().collect();
        Self::remove_recursive(self, &chars, 0, symbol);
    }

    fn remove_recursive(node: &mut SymbolTrie, chars: &[char], index: usize, symbol: &str) -> bool {
        if index == chars.len() {
            node.symbols.retain(|existing| existing != symbol);
        } else {
            let ch = chars[index];
            if let Some(child) = node.children.get_mut(&ch) {
                let should_prune_child = Self::remove_recursive(child, chars, index + 1, symbol);
                if should_prune_child {
                    node.children.remove(&ch);
                }
            }
        }

        node.children.is_empty() && node.symbols.is_empty()
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
    fn document_reindex_replaces_stale_symbols() {
        let mut index = SymbolIndex::new();

        index.index_document(
            "file:///one.pl",
            vec!["old_symbol".to_string(), "shared_symbol".to_string()],
        );
        index.index_document(
            "file:///one.pl",
            vec!["new_symbol".to_string(), "shared_symbol".to_string()],
        );

        let all_symbols = index.search_prefix("");
        assert!(!all_symbols.contains(&"old_symbol".to_string()));
        assert!(all_symbols.contains(&"new_symbol".to_string()));
        assert!(all_symbols.contains(&"shared_symbol".to_string()));
    }

    #[test]
    fn removing_document_preserves_shared_symbols() {
        let mut index = SymbolIndex::new();

        index.index_document("file:///one.pl", vec!["shared_symbol".to_string()]);
        index.index_document("file:///two.pl", vec!["shared_symbol".to_string()]);

        index.remove_document("file:///one.pl");
        assert!(index.search_prefix("shared").contains(&"shared_symbol".to_string()));

        index.remove_document("file:///two.pl");
        assert!(index.search_prefix("shared").is_empty());
    }
}
