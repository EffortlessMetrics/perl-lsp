//! Performance optimizations for large projects

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::ast::Node;

/// Cache for parsed ASTs with TTL
pub struct AstCache {
    cache: Arc<Mutex<HashMap<String, CachedAst>>>,
    max_size: usize,
    ttl: Duration,
}

struct CachedAst {
    ast: Arc<Node>,
    content_hash: u64,
    last_accessed: Instant,
}

impl AstCache {
    pub fn new(max_size: usize, ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            max_size,
            ttl: Duration::from_secs(ttl_seconds),
        }
    }
    
    /// Get cached AST if still valid
    pub fn get(&self, uri: &str, content: &str) -> Option<Arc<Node>> {
        let content_hash = Self::hash_content(content);
        
        let mut cache = self.cache.lock().unwrap();
        if let Some(cached) = cache.get_mut(uri) {
            let age = cached.last_accessed.elapsed();
            
            // Check if cache is still valid
            if age < self.ttl && cached.content_hash == content_hash {
                cached.last_accessed = Instant::now();
                return Some(Arc::clone(&cached.ast));
            } else {
                // Remove stale entry
                cache.remove(uri);
            }
        }
        None
    }
    
    /// Store AST in cache
    pub fn put(&self, uri: String, content: &str, ast: Arc<Node>) {
        let content_hash = Self::hash_content(content);
        
        let mut cache = self.cache.lock().unwrap();
        
        // Evict oldest entries if cache is full
        if cache.len() >= self.max_size {
            let oldest_key = cache.iter()
                .min_by_key(|(_, v)| v.last_accessed)
                .map(|(k, _)| k.clone());
            
            if let Some(key) = oldest_key {
                cache.remove(&key);
            }
        }
        
        cache.insert(uri, CachedAst {
            ast,
            content_hash,
            last_accessed: Instant::now(),
        });
    }
    
    /// Clear expired entries
    pub fn cleanup(&self) {
        let mut cache = self.cache.lock().unwrap();
        let now = Instant::now();
        
        cache.retain(|_, cached| {
            now.duration_since(cached.last_accessed) < self.ttl
        });
    }
    
    fn hash_content(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}

/// Incremental parsing optimizer
pub struct IncrementalParser {
    /// Track changed regions for incremental parsing
    changed_regions: Vec<(usize, usize)>,
}

impl IncrementalParser {
    pub fn new() -> Self {
        Self {
            changed_regions: Vec::new(),
        }
    }
    
    /// Mark a region as changed
    pub fn mark_changed(&mut self, start: usize, end: usize) {
        self.changed_regions.push((start, end));
        self.merge_overlapping_regions();
    }
    
    /// Check if a node needs reparsing based on changed regions
    pub fn needs_reparse(&self, node_start: usize, node_end: usize) -> bool {
        self.changed_regions.iter().any(|(start, end)| {
            // Check if node overlaps with any changed region
            node_start < *end && node_end > *start
        })
    }
    
    /// Clear all changed regions
    pub fn clear(&mut self) {
        self.changed_regions.clear();
    }
    
    fn merge_overlapping_regions(&mut self) {
        if self.changed_regions.len() < 2 {
            return;
        }
        
        self.changed_regions.sort_by_key(|(start, _)| *start);
        
        let mut merged = Vec::new();
        let mut current = self.changed_regions[0];
        
        for &(start, end) in &self.changed_regions[1..] {
            if start <= current.1 {
                // Overlapping or adjacent regions
                current.1 = current.1.max(end);
            } else {
                // Non-overlapping region
                merged.push(current);
                current = (start, end);
            }
        }
        merged.push(current);
        
        self.changed_regions = merged;
    }
}

/// Parallel processing utilities for large workspaces
pub mod parallel {
    use std::sync::mpsc;
    use std::thread;
    
    /// Process files in parallel with a worker pool
    pub fn process_files_parallel<T, F>(
        files: Vec<String>,
        num_workers: usize,
        processor: F,
    ) -> Vec<T>
    where
        T: Send + 'static,
        F: Fn(String) -> T + Send + Sync + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let work_queue = Arc::new(Mutex::new(files));
        let processor = Arc::new(processor);
        
        let mut handles = vec![];
        
        for _ in 0..num_workers {
            let tx = tx.clone();
            let work_queue = Arc::clone(&work_queue);
            let processor = Arc::clone(&processor);
            
            let handle = thread::spawn(move || {
                loop {
                    let file = {
                        let mut queue = work_queue.lock().unwrap();
                        queue.pop()
                    };
                    
                    match file {
                        Some(f) => {
                            let result = processor(f);
                            tx.send(result).unwrap();
                        }
                        None => break,
                    }
                }
            });
            
            handles.push(handle);
        }
        
        drop(tx);
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        rx.into_iter().collect()
    }
    
    use super::*;
}

/// Symbol index for fast lookups
pub struct SymbolIndex {
    /// Trie structure for prefix matching
    trie: SymbolTrie,
    /// Inverted index for fuzzy matching
    inverted_index: HashMap<String, Vec<String>>,
}

struct SymbolTrie {
    children: HashMap<char, Box<SymbolTrie>>,
    symbols: Vec<String>,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self {
            trie: SymbolTrie::new(),
            inverted_index: HashMap::new(),
        }
    }
    
    /// Add a symbol to the index
    pub fn add_symbol(&mut self, symbol: String) {
        // Add to trie for prefix matching
        self.trie.insert(&symbol);
        
        // Add to inverted index for fuzzy matching
        let tokens = Self::tokenize(&symbol);
        for token in tokens {
            self.inverted_index
                .entry(token)
                .or_default()
                .push(symbol.clone());
        }
    }
    
    /// Search symbols with prefix
    pub fn search_prefix(&self, prefix: &str) -> Vec<String> {
        self.trie.search_prefix(prefix)
    }
    
    /// Fuzzy search symbols
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
        Self {
            children: HashMap::new(),
            symbols: Vec::new(),
        }
    }
    
    fn insert(&mut self, symbol: &str) {
        let mut node = self;
        
        for ch in symbol.chars() {
            node = node.children
                .entry(ch)
                .or_insert_with(|| Box::new(SymbolTrie::new()));
        }
        
        node.symbols.push(symbol.to_string());
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
    use super::*;
    
    #[test]
    fn test_ast_cache() {
        let cache = AstCache::new(2, 60);
        let ast1 = Arc::new(Node::new(
            crate::ast::NodeKind::Program { statements: vec![] },
            crate::ast::SourceLocation { start: 0, end: 0 }
        ));
        let ast2 = Arc::new(Node::new(
            crate::ast::NodeKind::Program { statements: vec![] },
            crate::ast::SourceLocation { start: 0, end: 0 }
        ));
        
        cache.put("file1.pl".to_string(), "content1", ast1.clone());
        cache.put("file2.pl".to_string(), "content2", ast2.clone());
        
        // Should retrieve cached AST
        assert!(cache.get("file1.pl", "content1").is_some());
        
        // Different content should miss cache
        assert!(cache.get("file1.pl", "different").is_none());
        
        // Adding third item should evict oldest
        let ast3 = Arc::new(Node::new(
            crate::ast::NodeKind::Program { statements: vec![] },
            crate::ast::SourceLocation { start: 0, end: 0 }
        ));
        cache.put("file3.pl".to_string(), "content3", ast3);
        
        // file1 should be evicted (oldest)
        assert!(cache.get("file1.pl", "content1").is_none());
        assert!(cache.get("file2.pl", "content2").is_some());
        assert!(cache.get("file3.pl", "content3").is_some());
    }
    
    #[test]
    fn test_incremental_parser() {
        let mut parser = IncrementalParser::new();
        
        parser.mark_changed(10, 20);
        parser.mark_changed(30, 40);
        
        // Node overlapping with changed region
        assert!(parser.needs_reparse(15, 25));
        assert!(parser.needs_reparse(35, 45));
        
        // Node not overlapping
        assert!(!parser.needs_reparse(0, 5));
        assert!(!parser.needs_reparse(50, 60));
        
        // Test merging overlapping regions
        parser.mark_changed(18, 35);
        assert_eq!(parser.changed_regions.len(), 1);
        assert_eq!(parser.changed_regions[0], (10, 40));
    }
    
    #[test]
    fn test_symbol_index() {
        let mut index = SymbolIndex::new();
        
        index.add_symbol("calculate_total".to_string());
        index.add_symbol("calculate_average".to_string());
        index.add_symbol("get_user_name".to_string());
        
        // Prefix search
        let results = index.search_prefix("calc");
        assert_eq!(results.len(), 2);
        assert!(results.contains(&"calculate_total".to_string()));
        assert!(results.contains(&"calculate_average".to_string()));
        
        // Fuzzy search
        let results = index.search_fuzzy("user name");
        assert!(results.contains(&"get_user_name".to_string()));
    }
}