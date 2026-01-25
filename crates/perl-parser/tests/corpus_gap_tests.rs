// Test harness for corpus gap coverage
// These tests ensure the parser handles real-world Perl features missing from original corpus

use std::fs;
use std::path::Path;
use perl_parser::Parser;

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
    fn test_source_filters() {
        test_corpus_file("source_filters.pl").expect("source filters test failed");
    }

    #[test]
    fn test_xs_inline_ffi() {
        test_corpus_file("xs_inline_ffi.pl").expect("xs/ffi test failed");
    }

    #[test]
    fn test_modern_perl_features() {
        test_corpus_file("modern_perl_features.pl").expect("modern perl test failed");
    }

    #[test]
    fn test_advanced_regex() {
        test_corpus_file("advanced_regex.pl").expect("advanced regex test failed");
    }

    #[test]
    fn test_data_end_sections() {
        test_corpus_file("data_end_sections.pl").expect("data/end sections test failed");
    }

    #[test]
    fn test_packages_versions() {
        test_corpus_file("packages_versions.pl").expect("packages/versions test failed");
    }

    #[test]
    fn test_legacy_syntax() {
        test_corpus_file("legacy_syntax.pl").expect("legacy syntax test failed");
    }

    #[test]
    fn test_continue_redo_statements() {
        test_corpus_file("continue_redo_statements.pl").expect("continue/redo test failed");
    }

    #[test]
    fn test_format_statements() {
        test_corpus_file("format_statements.pl").expect("format statements test failed");
    }

    #[test]
    fn test_glob_expressions() {
        test_corpus_file("glob_expressions.pl").expect("glob expressions test failed");
    }

    #[test]
    fn test_tie_interface() {
        test_corpus_file("tie_interface.pl").expect("tie interface test failed");
    }

    // Property-based test for delimiters
    #[test]
    fn test_arbitrary_delimiters() {
        let delimiters = vec![
            ('!', '!'), ('{', '}'), ('[', ']'), 
            ('(', ')'), ('<', '>'), ('|', '|'),
            ('#', '#'), ('/', '/'), ('@', '@'),
        ];
        
        for (open, close) in delimiters {
            let code = format!("m{open}pattern{close}", open=open, close=close);
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
            let content = fs::read_to_string(&path).expect("Failed to read file");
            
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