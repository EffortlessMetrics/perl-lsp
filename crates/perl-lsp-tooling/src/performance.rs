//! Performance optimizations for large projects.
//!
//! This module is designed for large workspace scaling, including repositories
//! with tens of thousands of files where cache hit rates and bounded memory
//! usage are required to keep indexing and analysis responsive for enterprise
//! and large-file workloads.

use moka::sync::Cache;
use perl_parser_core::Node;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Cache for parsed ASTs with TTL.
///
/// Stores parsed ASTs with content hashing to avoid re-parsing unchanged files.
/// Uses a high-performance concurrent cache with automatic eviction.
pub struct AstCache {
    /// Concurrent cache storage with TTL and LRU eviction
    cache: Cache<String, CachedAst>,
}

/// A cached AST entry with metadata
#[derive(Clone)]
struct CachedAst {
    /// The cached AST node
    ast: Arc<Node>,
    /// Hash of the source content for validation
    content_hash: u64,
}

impl AstCache {
    /// Create a new AST cache with the given size limit and TTL
    pub fn new(max_size: usize, ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_size as u64)
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();

        Self { cache }
    }

    /// Get cached AST if still valid
    pub fn get(&self, uri: &str, content: &str) -> Option<Arc<Node>> {
        let content_hash = Self::hash_content(content);

        if let Some(cached) = self.cache.get(uri) {
            if cached.content_hash == content_hash {
                return Some(Arc::clone(&cached.ast));
            }
            self.cache.remove(uri);
        }
        None
    }

    /// Store AST in cache.
    ///
    /// Moka handles eviction automatically when capacity is reached.
    pub fn put(&self, uri: String, content: &str, ast: Arc<Node>) {
        let content_hash = Self::hash_content(content);
        self.cache.insert(uri, CachedAst { ast, content_hash });
    }

    /// Clear expired entries.
    ///
    /// Moka handles expiration automatically, but this method is kept for API compatibility.
    pub fn cleanup(&self) {
        self.cache.run_pending_tasks();
    }

    fn hash_content(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}

/// Incremental parsing optimizer.
///
/// Tracks changed regions to determine which AST nodes need reparsing.
pub struct IncrementalParser {
    /// Track changed regions as (start, end) byte offsets
    changed_regions: Vec<(usize, usize)>,
}

impl Default for IncrementalParser {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalParser {
    /// Create a new incremental parser with no changed regions
    pub fn new() -> Self {
        Self { changed_regions: Vec::new() }
    }

    /// Mark a region as changed.
    ///
    /// Overlapping regions are automatically merged.
    pub fn mark_changed(&mut self, start: usize, end: usize) {
        self.changed_regions.push((start, end));
        self.merge_overlapping_regions();
    }

    /// Check if a node needs reparsing based on changed regions.
    ///
    /// Returns true if the node overlaps with any changed region.
    pub fn needs_reparse(&self, node_start: usize, node_end: usize) -> bool {
        self.changed_regions.iter().any(|(start, end)| node_start < *end && node_end > *start)
    }

    /// Clear all changed regions.
    ///
    /// Call after reparsing to reset the change tracking.
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
                current.1 = current.1.max(end);
            } else {
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
    use super::*;
    use std::sync::mpsc;
    use std::thread;

    /// Process files in parallel with a worker pool.
    ///
    /// Distributes file processing across multiple threads for faster indexing.
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
                        let Ok(mut queue) = work_queue.lock() else {
                            break;
                        };
                        queue.pop()
                    };

                    match file {
                        Some(f) => {
                            let result = processor(f);
                            if tx.send(result).is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            });

            handles.push(handle);
        }

        drop(tx);

        for handle in handles {
            let _ = handle.join();
        }

        rx.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_cache() {
        let cache = AstCache::new(2, 60);
        let ast1 = Arc::new(Node::new(
            perl_parser_core::NodeKind::Program { statements: vec![] },
            perl_parser_core::SourceLocation { start: 0, end: 0 },
        ));
        let ast2 = Arc::new(Node::new(
            perl_parser_core::NodeKind::Program { statements: vec![] },
            perl_parser_core::SourceLocation { start: 0, end: 0 },
        ));

        cache.put("file1.pl".to_string(), "content1", ast1.clone());
        cache.put("file2.pl".to_string(), "content2", ast2.clone());

        assert!(cache.get("file1.pl", "content1").is_some());
        assert!(cache.get("file1.pl", "different").is_none());

        let ast3 = Arc::new(Node::new(
            perl_parser_core::NodeKind::Program { statements: vec![] },
            perl_parser_core::SourceLocation { start: 0, end: 0 },
        ));
        cache.put("file3.pl".to_string(), "content3", ast3);

        assert!(cache.get("file1.pl", "content1").is_none());
        assert!(cache.get("file2.pl", "content2").is_some());
        assert!(cache.get("file3.pl", "content3").is_some());
    }

    #[test]
    fn test_incremental_parser() {
        let mut parser = IncrementalParser::new();

        parser.mark_changed(10, 20);
        parser.mark_changed(30, 40);

        assert!(parser.needs_reparse(15, 25));
        assert!(parser.needs_reparse(35, 45));

        assert!(!parser.needs_reparse(0, 5));
        assert!(!parser.needs_reparse(50, 60));

        parser.mark_changed(18, 35);
        assert_eq!(parser.changed_regions.len(), 1);
        assert_eq!(parser.changed_regions[0], (10, 40));
    }

    #[test]
    fn test_cache_concurrent_access() {
        use std::thread;

        let cache = Arc::new(AstCache::new(100, 60));
        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                let ast = Arc::new(Node::new(
                    perl_parser_core::NodeKind::Program { statements: vec![] },
                    perl_parser_core::SourceLocation { start: 0, end: 0 },
                ));

                for j in 0..50 {
                    let key = format!("file_{}_{}.pl", i, j);
                    let content = format!("content_{}", j);

                    cache_clone.put(key.clone(), &content, ast.clone());
                    let _ = cache_clone.get(&key, &content);

                    if j % 10 == 0 {
                        cache_clone.cleanup();
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            let res = handle.join();
            assert!(res.is_ok(), "Thread should not panic");
        }

        let ast = Arc::new(Node::new(
            perl_parser_core::NodeKind::Program { statements: vec![] },
            perl_parser_core::SourceLocation { start: 0, end: 0 },
        ));
        cache.put("final.pl".to_string(), "final", ast.clone());
        assert!(cache.get("final.pl", "final").is_some());
    }

    #[test]
    fn test_cache_ttl_expiration() {
        use std::thread;
        use std::time::Duration;

        let cache = AstCache::new(10, 1);
        let ast = Arc::new(Node::new(
            perl_parser_core::NodeKind::Program { statements: vec![] },
            perl_parser_core::SourceLocation { start: 0, end: 0 },
        ));

        cache.put("test.pl".to_string(), "content", ast.clone());
        assert!(cache.get("test.pl", "content").is_some());

        thread::sleep(Duration::from_millis(1100));
        assert!(cache.get("test.pl", "content").is_none());
    }

    #[test]
    fn test_cache_content_hash_validation() {
        let cache = AstCache::new(10, 60);
        let ast1 = Arc::new(Node::new(
            perl_parser_core::NodeKind::Program { statements: vec![] },
            perl_parser_core::SourceLocation { start: 0, end: 0 },
        ));
        let ast2 = Arc::new(Node::new(
            perl_parser_core::NodeKind::Program { statements: vec![] },
            perl_parser_core::SourceLocation { start: 1, end: 1 },
        ));

        cache.put("test.pl".to_string(), "original content", ast1.clone());
        assert!(cache.get("test.pl", "original content").is_some());

        assert!(cache.get("test.pl", "modified content").is_none());

        cache.put("test.pl".to_string(), "modified content", ast2.clone());
        assert!(cache.get("test.pl", "modified content").is_some());
        assert!(cache.get("test.pl", "original content").is_none());
    }

    #[test]
    fn test_parallel_processing_graceful_degradation() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let processed = Arc::new(AtomicUsize::new(0));
        let files = vec!["file1.pl".to_string(), "file2.pl".to_string(), "file3.pl".to_string()];

        let processed_clone = Arc::clone(&processed);
        let results = parallel::process_files_parallel(files, 2, move |_file| {
            processed_clone.fetch_add(1, Ordering::SeqCst);
            42
        });

        assert_eq!(results.len(), 3);
        assert_eq!(processed.load(Ordering::SeqCst), 3);
        assert!(results.iter().all(|&x| x == 42));
    }
}
