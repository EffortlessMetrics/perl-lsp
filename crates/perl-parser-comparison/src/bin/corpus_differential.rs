//! Corpus differential binary - walks the project's real-world Perl corpora and
//! surfaces per-parser disagreements.
//!
//! # Usage
//!
//! ```bash
//! cargo run -p perl-parser-comparison --bin corpus_differential
//! ```
//!
//! The binary resolves corpus roots relative to `CARGO_MANIFEST_DIR` (the
//! `crates/perl-parser-comparison/` directory) by walking up to the workspace
//! root and then finding the corpus directories.
//!
//! Output is a Markdown report printed to stdout.  Redirect to a file if needed:
//!
//! ```bash
//! cargo run -p perl-parser-comparison --bin corpus_differential \
//!   > target/parser-corpus-differential-report.md
//! ```

use std::path::PathBuf;

fn main() {
    let workspace_root = locate_workspace_root();

    let corpus_roots: Vec<PathBuf> = ["test_corpus", "tree-sitter-perl/test/highlight"]
        .iter()
        .map(|rel| workspace_root.join(rel))
        .filter(|p| p.exists())
        .collect();

    eprintln!("Scanning {} corpus root(s):", corpus_roots.len());
    for root in &corpus_roots {
        eprintln!("  {}", root.display());
    }

    let records = perl_parser_comparison::walk_corpora(&corpus_roots);
    let stats = perl_parser_comparison::AggregateStats::from_records(&records);
    let report = perl_parser_comparison::format_report(&records, &stats);

    println!("{report}");

    // Summary to stderr so it shows even when stdout is redirected
    eprintln!(
        "\nTotal: {} files, {} disagreements ({:.1}%)",
        stats.total,
        stats.total_disagreements(),
        if stats.total > 0 {
            stats.total_disagreements() as f64 / stats.total as f64 * 100.0
        } else {
            0.0
        }
    );
}

/// Walk from the binary's directory up to find the workspace root
/// (the directory containing `Cargo.toml` with `[workspace]`).
fn locate_workspace_root() -> PathBuf {
    // Try CARGO_MANIFEST_DIR env var first (set by cargo when running tests/bins)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest = PathBuf::from(manifest_dir);
        // manifest_dir is crates/perl-parser-comparison; walk up two levels
        if let Some(root) = manifest.parent().and_then(|p| p.parent()) {
            if root.join("Cargo.toml").exists() {
                return root.to_owned();
            }
        }
    }

    // Fallback: walk up from current working directory
    let mut dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => return PathBuf::from("."),
    };
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            // Check if it's the workspace root (has [workspace] table)
            if let Ok(content) = std::fs::read_to_string(&candidate) {
                if content.contains("[workspace]") {
                    return dir;
                }
            }
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_owned(),
            None => break,
        }
    }

    // Last resort: assume we're running from the workspace root
    PathBuf::from(".")
}
