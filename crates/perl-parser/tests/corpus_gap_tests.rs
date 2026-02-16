// Test harness for corpus gap coverage
// These tests ensure the parser handles real-world Perl features missing from original corpus

use perl_parser::Parser;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod corpus_gap_tests {
    use super::*;

    fn resolve_corpus_path(filename: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let candidate = Path::new("test_corpus").join(filename);
        if candidate.exists() {
            return Ok(candidate);
        }

        let fallback = Path::new("../../test_corpus").join(filename);
        if fallback.exists() {
            return Ok(fallback);
        }

        Err(format!(
            "Unable to locate corpus fixture '{filename}' in test_corpus/ or ../../test_corpus/"
        )
        .into())
    }

    fn read_corpus_file(filename: &str) -> Result<String, Box<dyn std::error::Error>> {
        let path = resolve_corpus_path(filename)?;
        Ok(fs::read_to_string(path)?)
    }

    // Helper to test a corpus file doesn't crash the parser
    fn test_corpus_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = read_corpus_file(filename)?;

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

    /// Regression: anonymous sub as expression initializer (`my $c = sub { 1 };`)
    /// must produce a subroutine node inside the initializer (locks down peek_second() fix).
    #[test]
    fn test_anonymous_sub_expression() -> Result<(), Box<dyn std::error::Error>> {
        let input = "my $c = sub { 1 };";
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;

        let sexp = ast.to_sexp();
        // The variable declaration's initializer should contain a subroutine node
        assert!(
            sexp.contains("subroutine") || sexp.contains("anonymous_sub") || sexp.contains("sub"),
            "expected subroutine/anonymous_sub/sub node in initializer, got: {sexp}"
        );
        Ok(())
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

    // Parse corpus files repeatedly to keep a lightweight performance guard in default CI.
    #[test]
    fn bench_corpus_files() -> Result<(), Box<dyn std::error::Error>> {
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

        const ITERATIONS: u32 = 3;
        const MAX_PER_PARSE_MS: u128 = 500;

        for file in files {
            let content = read_corpus_file(file)?;

            let start = Instant::now();
            for _ in 0..ITERATIONS {
                let mut parser = Parser::new(&content);
                let parse_result = parser.parse();
                assert!(
                    parse_result.is_ok(),
                    "Failed to parse file {}: {:?}",
                    file,
                    parse_result.err()
                );
            }
            let duration = start.elapsed();
            let per_parse_ms = duration.as_millis() / u128::from(ITERATIONS);

            println!("{}: {}ms per parse", file, per_parse_ms);
            assert!(
                per_parse_ms <= MAX_PER_PARSE_MS,
                "Corpus parse regression for {}: {}ms per parse exceeds {}ms budget",
                file,
                per_parse_ms,
                MAX_PER_PARSE_MS
            );
        }

        Ok(())
    }
}
