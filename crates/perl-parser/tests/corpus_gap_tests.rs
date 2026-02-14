// Test harness for corpus gap coverage
// These tests ensure the parser handles real-world Perl features missing from original corpus

use perl_parser::Parser;
use std::fs;
use std::path::Path;

#[cfg(test)]
mod corpus_gap_tests {
    use super::*;

    // Helper to test a corpus file doesn't crash the parser
    fn test_corpus_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut path = Path::new("test_corpus").join(filename);
        if !path.exists() {
            // Try parent directory if run from within a crate
            path = Path::new("../../test_corpus").join(filename);
        }
        let content = fs::read_to_string(&path)?;

        let mut parser = Parser::new(&content);
        let result = parser.parse();

        assert!(result.is_ok(), "Failed to parse {}: {:?}", filename, result.err());
        Ok(())
    }

    #[test]
    fn test_source_filters() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("source_filters.pl")
    }

    #[test]
    fn test_xs_inline_ffi() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("xs_inline_ffi.pl")
    }

    #[test]
    fn test_modern_perl_features() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("modern_perl_features.pl")
    }

    #[test]
    fn test_advanced_regex() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("advanced_regex.pl")
    }

    #[test]
    fn test_data_end_sections() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("data_end_sections.pl")
    }

    #[test]
    fn test_packages_versions() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("packages_versions.pl")
    }

    #[test]
    fn test_legacy_syntax() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("legacy_syntax.pl")
    }

    #[test]
    fn test_continue_redo_statements() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("continue_redo_statements.pl")
    }

    #[test]
    fn test_format_statements() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("format_statements.pl")
    }

    #[test]
    fn test_glob_expressions() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("glob_expressions.pl")
    }

    #[test]
    fn test_tie_interface() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("tie_interface.pl")
    }

    // Property-based test for delimiters
    #[test]
    fn test_arbitrary_delimiters() {
        let delimiters = vec![
            ('!', '!'),
            ('{', '}'),
            ('[', ']'),
            ('(', ')'),
            ('<', '>'),
            ('|', '|'),
            ('#', '#'),
            ('/', '/'),
            ('@', '@'),
        ];

        for (open, close) in delimiters {
            let code = format!("m{open}pattern{close}", open = open, close = close);
            let mut parser = Parser::new(&code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse m{}{}", open, close);
        }
    }

    // Benchmark corpus files (optional, run with --release)
    #[test]
    #[ignore] // Run with: cargo test --ignored --release
    fn bench_corpus_files() {
        use std::time::Instant;

        let files = vec![
            "source_filters.pl",
            "xs_inline_ffi.pl",
            "modern_perl_features.pl",
            "advanced_regex.pl",
            "data_end_sections.pl",
            "packages_versions.pl",
            "legacy_syntax.pl",
            "continue_redo_statements.pl",
            "format_statements.pl",
            "glob_expressions.pl",
            "tie_interface.pl",
        ];

        for file in files {
            let path = Path::new("test_corpus").join(file);
            let content_res = fs::read_to_string(&path);
            assert!(content_res.is_ok(), "Failed to read file {}: {:?}", file, content_res.err());
            let content = content_res.unwrap_or_else(|_| unreachable!());

            let start = Instant::now();
            for _ in 0..100 {
                let mut parser = Parser::new(&content);
                let _ = parser.parse();
            }
            let duration = start.elapsed();

            println!("{}: {:?} per parse", file, duration / 100);
        }
    }
}
