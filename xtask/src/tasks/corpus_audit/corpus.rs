//! Corpus file parsing and inventory
//!
//! This module handles discovery and parsing of corpus files across all layers:
//! - tree-sitter corpus
//! - highlight fixtures
//! - test corpus
//! - perl-corpus generators

use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Corpus layer classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CorpusLayer {
    /// Tree-sitter corpus files (test/corpus/*.txt)
    TreeSitter,
    /// Highlight test fixtures (test/highlight/*.txt)
    Highlight,
    /// Test corpus files (test_corpus/**/*.pl)
    TestCorpus,
    /// Perl-corpus generator files (crates/perl-corpus/src/generators/*.rs)
    PerlCorpus,
}

impl CorpusLayer {
    /// Get the directory path for this corpus layer
    pub fn directory(&self) -> &'static str {
        match self {
            CorpusLayer::TreeSitter => "c/test/corpus",
            CorpusLayer::Highlight => "c/test/highlight",
            CorpusLayer::TestCorpus => "test_corpus",
            CorpusLayer::PerlCorpus => "crates/perl-corpus/src/generators",
        }
    }

    /// Get the file extension for this corpus layer
    pub fn extension(&self) -> &'static str {
        match self {
            CorpusLayer::TreeSitter => "txt",
            CorpusLayer::Highlight => "txt",
            CorpusLayer::TestCorpus => "pl",
            CorpusLayer::PerlCorpus => "rs",
        }
    }
}

/// A single corpus file with its metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusFile {
    /// Path to the corpus file
    pub path: PathBuf,
    /// Corpus layer this file belongs to
    pub layer: CorpusLayer,
    /// File content
    pub content: String,
    /// File size in bytes
    pub size_bytes: usize,
    /// Number of lines in the file
    pub line_count: usize,
}

/// Corpus inventory summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusInventory {
    /// Total number of corpus files
    pub total_files: usize,
    /// Number of files per layer
    pub files_by_layer: Vec<LayerCount>,
    /// Total size of all corpus files in bytes
    pub total_size_bytes: usize,
    /// Total number of lines across all corpus files
    pub total_line_count: usize,
}

/// Count of files in a specific layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerCount {
    /// Corpus layer
    pub layer: CorpusLayer,
    /// Number of files in this layer
    pub count: usize,
}

/// Parse all corpus files from the given directory
///
/// This function discovers and parses corpus files from all layers:
/// - tree-sitter corpus files
/// - highlight test fixtures
/// - test corpus files
/// - perl-corpus generator files
pub fn parse_corpus_files(corpus_path: &Path) -> Result<Vec<CorpusFile>> {
    let mut files = Vec::new();

    // Process each corpus layer
    for layer in [
        CorpusLayer::TreeSitter,
        CorpusLayer::Highlight,
        CorpusLayer::TestCorpus,
        CorpusLayer::PerlCorpus,
    ] {
        let layer_path = corpus_path.join(layer.directory());

        if layer_path.exists() {
            let layer_files = parse_corpus_layer(&layer_path, layer)?;
            files.extend(layer_files);
        }
    }

    Ok(files)
}

/// Parse corpus files from a specific layer directory
fn parse_corpus_layer(layer_path: &Path, layer: CorpusLayer) -> Result<Vec<CorpusFile>> {
    let mut files = Vec::new();

    // Walk the directory and collect files
    for entry in WalkDir::new(layer_path)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        // Skip files that are clearly not corpus files
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if file_name.starts_with('_')
            || file_name.starts_with('.')
            || file_name.ends_with(".md")
            || file_name == "README"
        {
            continue;
        }

        // Check file extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && ext != layer.extension()
        {
            continue;
        }

        // Read file content
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read corpus file: {}", path.display()))?;

        // Create corpus file entry
        files.push(CorpusFile {
            path: path.to_path_buf(),
            layer,
            size_bytes: content.len(),
            line_count: content.lines().count(),
            content,
        });
    }

    Ok(files)
}

/// Generate corpus inventory from a list of corpus files
pub fn generate_inventory(files: &[CorpusFile]) -> CorpusInventory {
    let total_files = files.len();
    let total_size_bytes = files.iter().map(|f| f.size_bytes).sum();
    let total_line_count = files.iter().map(|f| f.line_count).sum();

    // Count files per layer
    let mut layer_counts: std::collections::HashMap<CorpusLayer, usize> =
        std::collections::HashMap::new();
    for file in files {
        *layer_counts.entry(file.layer).or_insert(0) += 1;
    }

    let files_by_layer =
        layer_counts.into_iter().map(|(layer, count)| LayerCount { layer, count }).collect();

    CorpusInventory { total_files, files_by_layer, total_size_bytes, total_line_count }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corpus_layer_directory() {
        assert_eq!(CorpusLayer::TreeSitter.directory(), "c/test/corpus");
        assert_eq!(CorpusLayer::Highlight.directory(), "c/test/highlight");
        assert_eq!(CorpusLayer::TestCorpus.directory(), "test_corpus");
        assert_eq!(CorpusLayer::PerlCorpus.directory(), "crates/perl-corpus/src/generators");
    }

    #[test]
    fn test_corpus_layer_extension() {
        assert_eq!(CorpusLayer::TreeSitter.extension(), "txt");
        assert_eq!(CorpusLayer::Highlight.extension(), "txt");
        assert_eq!(CorpusLayer::TestCorpus.extension(), "pl");
        assert_eq!(CorpusLayer::PerlCorpus.extension(), "rs");
    }
}
