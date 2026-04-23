//! Performance optimizations for large projects.
//!
//! This module is designed for large workspace scaling, including repositories
//! with tens of thousands of files where cache hit rates and bounded memory
//! usage are required to keep indexing and analysis responsive for enterprise
//! and large-file workloads.
//!
//! Previously the standalone `perl-lsp-performance` crate; absorbed into
//! `perl-lsp-rs-core::performance` in Wave G3 (#4535).

use moka::sync::Cache;
use perl_parser_core::Node;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use perl_symbol::SymbolIndex;

/// Assert that a value is `Some` and return it, panicking with `message` if `None`.
///
/// This is a test helper exported for integration tests that do `use performance::*`.
///
/// # Panics
///
/// Panics with `message` if `value` is `None`. Intended for tests only.
#[allow(clippy::panic)]
#[track_caller]
pub fn assert_some<T>(value: Option<T>, message: &str) -> T {
    match value {
        Some(v) => v,
        None => panic!("{message}"),
    }
}

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
            // Check if content hash matches (skip if content changed)
            if cached.content_hash == content_hash {
                return Some(Arc::clone(&cached.ast));
            } else {
                // Remove stale entry
                self.cache.remove(uri);
            }
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
        let (start, end) = if start <= end { (start, end) } else { (end, start) };

        // Ignore zero-length spans to keep the tracking set focused on
        // meaningful byte ranges.
        if start == end {
            return;
        }

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

/// Parallel processing utilities for large workspaces.
pub mod parallel {
    use super::Arc;
    use super::Mutex;
    use std::sync::mpsc;
    use std::thread;

    /// Parallel indexer for workspace-wide symbol indexing.
    pub struct ParallelIndexer;

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
        if files.is_empty() {
            return Vec::new();
        }

        let file_count = files.len();

        // Ensure callers cannot accidentally request zero workers and drop all work.
        // This preserves the API contract that every input file is processed once.
        let effective_workers = num_workers.max(1).min(file_count);

        let (tx, rx) = mpsc::channel();
        let work_queue = Arc::new(Mutex::new(files.into_iter().enumerate().collect::<Vec<_>>()));
        let processor = Arc::new(processor);

        let mut handles = vec![];

        for _ in 0..effective_workers {
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
                        Some((index, file)) => {
                            let result = processor(file);
                            if tx.send((index, result)).is_err() {
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

        let mut ordered_results = Vec::with_capacity(file_count);
        ordered_results.resize_with(file_count, || None);

        for (index, value) in rx {
            ordered_results[index] = Some(value);
        }

        ordered_results.into_iter().flatten().collect()
    }
}
