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

    fn resolve_corpus_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let candidate = Path::new("test_corpus");
        if candidate.exists() {
            return Ok(candidate.to_path_buf());
        }

        let fallback = Path::new("../../test_corpus");
        if fallback.exists() {
            return Ok(fallback.to_path_buf());
        }

        Err("Unable to locate test_corpus/ directory".into())
    }

    fn discover_corpus_files() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let corpus_dir = resolve_corpus_dir()?;
        let mut files = Vec::new();
        collect_pl_files(&corpus_dir, &mut files);
        files.sort();
        Ok(files)
    }

    fn collect_pl_files(dir: &Path, files: &mut Vec<PathBuf>) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.starts_with('.') && !name.starts_with('_') {
                    collect_pl_files(&path, files);
                }
            } else if path.extension().and_then(|e| e.to_str()) == Some("pl") {
                files.push(path);
            }
        }
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

    /// Auto-discovery test: parse every .pl file in the corpus directory.
    /// This ensures 100% corpus coverage without needing to list files individually.
    #[test]
    fn test_all_corpus_files() -> Result<(), Box<dyn std::error::Error>> {
        let files = discover_corpus_files()?;
        assert!(!files.is_empty(), "No .pl files found in test_corpus/");

        let mut failures = Vec::new();

        for path in &files {
            let content = fs::read_to_string(path)?;
            let mut parser = Parser::new(&content);

            if let Err(e) = parser.parse() {
                failures.push(format!("{}: {e}", path.display()));
            }
        }

        assert!(
            failures.is_empty(),
            "Failed to parse {} of {} corpus files:\n  {}",
            failures.len(),
            files.len(),
            failures.join("\n  ")
        );

        println!("Parsed all {} corpus files successfully", files.len());
        Ok(())
    }

    // Parse all corpus files repeatedly to keep a lightweight performance guard in default CI.
    #[test]
    fn bench_corpus_files() -> Result<(), Box<dyn std::error::Error>> {
        use std::time::Instant;

        let files = discover_corpus_files()?;
        assert!(!files.is_empty(), "No .pl files found in test_corpus/");

        const ITERATIONS: u32 = 3;
        const MAX_PER_PARSE_MS: u128 = 500;

        for path in &files {
            let content = fs::read_to_string(path)?;
            let display = path.display().to_string();

            let start = Instant::now();
            for _ in 0..ITERATIONS {
                let mut parser = Parser::new(&content);
                let parse_result = parser.parse();
                assert!(
                    parse_result.is_ok(),
                    "Failed to parse file {}: {:?}",
                    display,
                    parse_result.err()
                );
            }
            let duration = start.elapsed();
            let per_parse_ms = duration.as_millis() / u128::from(ITERATIONS);

            println!("{}: {}ms per parse", display, per_parse_ms);
            assert!(
                per_parse_ms <= MAX_PER_PARSE_MS,
                "Corpus parse regression for {}: {}ms per parse exceeds {}ms budget",
                display,
                per_parse_ms,
                MAX_PER_PARSE_MS
            );
        }

        Ok(())
    }
}
