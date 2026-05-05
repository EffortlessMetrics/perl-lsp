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
    /// Reference count for each symbol across indexed documents.
    symbol_ref_counts: HashMap<String, usize>,
    /// Per-document symbol sets for replace/remove updates.
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
            symbol_ref_counts: HashMap::new(),
            document_symbols: HashMap::new(),
        }
    }

    /// Add a symbol to the index.
    ///
    /// Indexes the symbol for both prefix and fuzzy matching.
    /// Duplicate calls with the same symbol are idempotent: the symbol is
    /// stored exactly once in both the trie and the inverted index.
    pub fn add_symbol(&mut self, symbol: String) {
        self.add_symbol_occurrence(&symbol);
    }

    /// Replace all symbols for a specific document.
    ///
    /// This operation is idempotent and removes stale symbols that no longer
    /// exist in the latest version of the document.
    pub fn replace_document_symbols(&mut self, uri: &str, symbols: Vec<String>) {
        self.remove_document(uri);

        let mut seen: HashSet<String> = HashSet::new();
        let mut unique_symbols: Vec<String> = Vec::new();
        for symbol in symbols {
            if seen.insert(symbol.clone()) {
                self.add_symbol_occurrence(&symbol);
                unique_symbols.push(symbol);
            }
        }

        if !unique_symbols.is_empty() {
            self.document_symbols.insert(uri.to_string(), unique_symbols);
        }
    }

    /// Remove all symbols for a specific document.
    pub fn remove_document(&mut self, uri: &str) {
        if let Some(existing) = self.document_symbols.remove(uri) {
            for symbol in existing {
                self.remove_symbol_occurrence(&symbol);
            }
        }
    }

    /// Search symbols with prefix.
    ///
    /// Returns all symbols starting with the given prefix.
    #[must_use]
    pub fn search_prefix(&self, prefix: &str) -> Vec<String> {
        let mut results = self.trie.search_prefix(prefix);
        results.sort();
        results
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
        sorted.sort_by(|(name_a, score_a), (name_b, score_b)| {
            score_b
                .cmp(score_a)
                .then_with(|| name_a.len().cmp(&name_b.len()))
                .then_with(|| name_a.cmp(name_b))
        });

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

    fn add_symbol_occurrence(&mut self, symbol: &str) {
        let count = self.symbol_ref_counts.entry(symbol.to_string()).or_insert(0);
        *count += 1;
        if *count > 1 {
            return;
        }

        self.trie.insert(symbol);
        let tokens = Self::tokenize(symbol);
        for token in tokens {
            self.inverted_index.entry(token).or_default().push(symbol.to_string());
        }
    }

    fn remove_symbol_occurrence(&mut self, symbol: &str) {
        let Some(count) = self.symbol_ref_counts.get_mut(symbol) else {
            return;
        };

        if *count > 1 {
            *count -= 1;
            return;
        }

        self.symbol_ref_counts.remove(symbol);
        self.trie.remove(symbol);
        let tokens = Self::tokenize(symbol);
        for token in tokens {
            let mut should_prune = false;
            if let Some(symbols) = self.inverted_index.get_mut(&token) {
                symbols.retain(|candidate| candidate != symbol);
                should_prune = symbols.is_empty();
            }
            if should_prune {
                self.inverted_index.remove(&token);
            }
        }
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

    fn remove(&mut self, symbol: &str) {
        let chars: Vec<char> = symbol.chars().collect();
        self.remove_recursive(&chars, 0, symbol);
    }

    fn remove_recursive(&mut self, chars: &[char], idx: usize, symbol: &str) -> bool {
        if idx == chars.len() {
            self.symbols.retain(|value| value != symbol);
        } else if let Some(child) = self.children.get_mut(&chars[idx]) {
            let prune_child = child.remove_recursive(chars, idx + 1, symbol);
            if prune_child {
                self.children.remove(&chars[idx]);
            }
        }

        self.children.is_empty() && self.symbols.is_empty()
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
    fn duplicate_add_symbol_does_not_duplicate_search_results() {
        let mut index = SymbolIndex::new();
        index.add_symbol("get_user_name".to_string());
        index.add_symbol("get_user_name".to_string());

        assert_eq!(index.search_prefix("get"), vec!["get_user_name".to_string()]);
        assert_eq!(index.search_fuzzy("user name"), vec!["get_user_name".to_string()]);
    }

    #[test]
    fn replace_document_symbols_removes_stale_entries() {
        let mut index = SymbolIndex::new();
        index.replace_document_symbols(
            "file:///a.pl",
            vec!["old_name".to_string(), "shared".to_string()],
        );
        index.replace_document_symbols(
            "file:///a.pl",
            vec!["new_name".to_string(), "shared".to_string()],
        );

        assert!(index.search_prefix("old").is_empty());
        assert!(index.search_prefix("new").contains(&"new_name".to_string()));
        assert!(index.search_prefix("sha").contains(&"shared".to_string()));
    }

    #[test]
    fn remove_document_preserves_symbols_from_other_documents() {
        let mut index = SymbolIndex::new();
        index.replace_document_symbols(
            "file:///a.pl",
            vec!["shared".to_string(), "only_a".to_string()],
        );
        index.replace_document_symbols(
            "file:///b.pl",
            vec!["shared".to_string(), "only_b".to_string()],
        );
        index.remove_document("file:///a.pl");

        let shared = index.search_prefix("shared");
        assert_eq!(shared, vec!["shared".to_string()]);
        assert!(index.search_prefix("only_a").is_empty());
        assert_eq!(index.search_prefix("only_b"), vec!["only_b".to_string()]);
    }

    #[test]
    fn trie_remove_preserves_symbols_with_shared_prefix() {
        // Removing "foo" must not evict "foobar" from the trie.
        let mut index = SymbolIndex::new();
        index.replace_document_symbols(
            "file:///a.pl",
            vec!["foo".to_string(), "foobar".to_string()],
        );
        index.replace_document_symbols("file:///a.pl", vec!["foobar".to_string()]);

        assert!(index.search_prefix("foo").contains(&"foobar".to_string()));
        assert!(!index.search_prefix("foo").contains(&"foo".to_string()));
    }

    #[test]
    fn replace_with_empty_list_clears_document_symbols() {
        // A file edited to contain no symbols must have its prior symbols removed.
        let mut index = SymbolIndex::new();
        index.replace_document_symbols("file:///a.pl", vec!["old_sub".to_string()]);
        index.replace_document_symbols("file:///a.pl", vec![]);

        assert!(index.search_prefix("old_sub").is_empty());

        // A subsequent replace with real symbols must work correctly.
        index.replace_document_symbols("file:///a.pl", vec!["new_sub".to_string()]);
        assert!(index.search_prefix("new_sub").contains(&"new_sub".to_string()));
    }

    #[test]
    fn document_symbols_are_fuzzy_searchable_after_replace() {
        // Symbols added via replace_document_symbols must appear in fuzzy results,
        // not only in prefix results. The inverted index must be populated.
        let mut index = SymbolIndex::new();
        index.replace_document_symbols("file:///a.pl", vec!["get_user_name".to_string()]);

        let results = index.search_fuzzy("user name");
        assert!(
            results.contains(&"get_user_name".to_string()),
            "document symbols must be fuzzy-searchable"
        );

        // After replacing the document, the old symbol must not appear in fuzzy results.
        index.replace_document_symbols("file:///a.pl", vec!["set_password".to_string()]);
        let results = index.search_fuzzy("user name");
        assert!(
            !results.contains(&"get_user_name".to_string()),
            "replaced symbol must be removed from fuzzy index"
        );
    }

    #[test]
    fn replace_document_symbols_deduplicates_same_symbol_in_one_document() {
        let mut index = SymbolIndex::new();
        index.replace_document_symbols(
            "file:///a.pl",
            vec!["shared_name".to_string(), "shared_name".to_string()],
        );

        assert_eq!(index.search_prefix("shared"), vec!["shared_name".to_string()]);
        assert_eq!(index.search_fuzzy("shared name"), vec!["shared_name".to_string()]);
    }

    #[test]
    fn fuzzy_ranking_prefers_more_matching_tokens() {
        let mut index = SymbolIndex::new();
        index.add_symbol("alpha_beta_gamma".to_string());
        index.add_symbol("alpha_beta".to_string());
        index.add_symbol("alpha_only".to_string());

        assert_eq!(
            index.search_fuzzy("alpha beta gamma"),
            vec![
                "alpha_beta_gamma".to_string(),
                "alpha_beta".to_string(),
                "alpha_only".to_string()
            ]
        );
    }

    #[test]
    fn fuzzy_handles_camel_snake_and_mixed_delimiters() {
        let mut index = SymbolIndex::new();
        index.add_symbol("parseHTTPHeader".to_string());
        index.add_symbol("parse_http_header".to_string());
        index.add_symbol("parse-http-header".to_string());

        let results = index.search_fuzzy("parse http header");
        assert_eq!(
            results,
            vec![
                "parse-http-header".to_string(),
                "parse_http_header".to_string(),
                "parseHTTPHeader".to_string()
            ]
        );
    }

    #[test]
    fn empty_query_returns_empty_result() {
        let mut index = SymbolIndex::new();
        index.add_symbol("anything".to_string());
        assert!(index.search_fuzzy("").is_empty());
    }

    #[test]
    fn unknown_prefix_returns_empty_result() {
        let mut index = SymbolIndex::new();
        index.add_symbol("known_symbol".to_string());
        assert!(index.search_prefix("zzz").is_empty());
    }

    #[test]
    fn fuzzy_tie_breakers_are_deterministic() {
        let mut index = SymbolIndex::new();
        index.add_symbol("alpha_zeta".to_string());
        index.add_symbol("alpha_beta".to_string());
        index.add_symbol("alpha".to_string());

        assert_eq!(
            index.search_fuzzy("alpha"),
            vec![
                "alpha".to_string(),
                "alpha_beta".to_string(),
                "alpha_zeta".to_string()
            ]
        );
    }
}
