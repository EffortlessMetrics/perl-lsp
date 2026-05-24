//! Comprehensive Perl test corpus and property-based testing infrastructure
//!
//! This crate provides a curated collection of Perl code samples for testing parser correctness,
//! edge case coverage, and LSP feature validation. It includes both manually curated test cases
//! and property-based test generators for comprehensive coverage.
//!
//! # Architecture
//!
//! The corpus is organized into several layers:
//!
//! - **Curated Test Cases**: Hand-written examples covering Perl syntax edge cases
//! - **Property-Based Generators**: Randomized code generation for fuzz testing
//! - **Real-World Samples**: Code from CPAN and production Perl projects
//! - **Metadata System**: Tag-based organization with section markers and test IDs
//!
//! # Corpus Organization
//!
//! Test cases are stored in text files with section markers and metadata:
//!
//! ```text
//! ==========================================
//! Basic Variable Declaration
//! ==========================================
//! # @id: vars.basic.my
//! # @tags: variables, declaration
//! my $x = 42;
//! ---
//! (expected AST representation)
//! ```
//!
//! Each section includes:
//! - **Title**: Human-readable test case name
//! - **Metadata**: ID, tags, Perl version requirements, flags
//! - **Body**: Perl code to parse
//! - **Expected Output**: Optional AST or error expectations (after `---`)
//!
//! # Usage
//!
//! ## Loading Corpus Files
//!
//! ```rust,ignore
//! use perl_corpus::{CorpusPaths, get_corpus_files};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let files = get_corpus_files();
//!
//! for file in files {
//!     println!("Found corpus file: {:?}", file.path);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Parsing Corpus Sections
//!
//! ```rust
//! use perl_corpus::parse_file;
//! use std::path::Path;
//!
//! # fn example() -> anyhow::Result<()> {
//! # let path = Path::new("test_corpus/variables.txt");
//! # if !path.exists() { return Ok(()); }
//! let sections = parse_file(path)?;
//!
//! for section in sections {
//!     println!("Section: {} (id: {})", section.title, section.id);
//!     println!("Tags: {:?}", section.tags);
//!     println!("Code:\n{}", section.body);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Finding Cases by Tag
//!
//! ```rust
//! use perl_corpus::{parse_dir, find_by_tag};
//! use std::path::Path;
//!
//! # fn example() -> anyhow::Result<()> {
//! # let corpus_dir = Path::new("test_corpus");
//! # if !corpus_dir.exists() { return Ok(()); }
//! let all_sections = parse_dir(corpus_dir)?;
//! let regex_tests = find_by_tag(&all_sections, "regex");
//!
//! println!("Found {} regex test cases", regex_tests.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Using Property-Based Generators
//!
//! ```rust,ignore
//! use perl_corpus::{generate_perl_code_with_seed, CodegenOptions};
//!
//! // Generate random valid Perl code
//! let code = generate_perl_code_with_seed(10, 42);
//! println!("Generated:\n{}", code);
//!
//! // Generate with specific options
//! let options = CodegenOptions::default();
//! let modern_code = generate_perl_code(&options);
//! ```
//!
//! ## Specialized Test Case Modules
//!
//! The corpus includes focused generators for specific Perl features:
//!
//! ### Complex Data Structures
//!
//! ```rust,ignore
//! use perl_corpus::{complex_data_structure_cases, find_complex_case};
//!
//! let cases = complex_data_structure_cases();
//! if let Some(nested) = find_complex_case("nested-arrays") {
//!     println!("Test: {}", nested.description);
//!     println!("Code:\n{}", nested.code);
//! }
//! ```
//!
//! ### Continue/Redo Blocks
//!
//! ```rust
//! use perl_corpus::{continue_redo_cases, valid_continue_redo_cases};
//!
//! let all_cases = continue_redo_cases();
//! let valid_only = valid_continue_redo_cases();
//! ```
//!
//! ### Format Statements
//!
//! ```rust,ignore
//! use perl_corpus::{format_statement_cases, FormatStatementGenerator};
//!
//! let cases = format_statement_cases();
//! let generator = FormatStatementGenerator::new(42);
//! ```
//!
//! ### Glob Expressions
//!
//! ```rust,ignore
//! use perl_corpus::{glob_expression_cases, GlobExpressionGenerator};
//!
//! let cases = glob_expression_cases();
//! let generator = GlobExpressionGenerator::new(42);
//! ```
//!
//! ### Tie Interface
//!
//! ```rust
//! use perl_corpus::{tie_interface_cases, tie_cases_by_tag};
//!
//! let all_tie = tie_interface_cases();
//! let scalar_tie = tie_cases_by_tag("scalar");
//! ```
//!
//! # Corpus Layers
//!
//! The corpus is organized into three layers accessible via [`CorpusLayer`]:
//!
//! - **`CorpusLayer::Main`**: Core test cases in `test_corpus/`
//! - **`CorpusLayer::TreeSitter`**: Tree-sitter grammar tests in `tree-sitter-perl/test/corpus/`
//! - **`CorpusLayer::Fuzz`**: Fuzzing inputs and edge cases in `crates/perl-corpus/fuzz/`
//!
//! ## Environment Configuration
//!
//! Override the corpus root with the `CORPUS_ROOT` environment variable:
//!
//! ```bash
//! export CORPUS_ROOT=/path/to/custom/corpus
//! cargo test
//! ```
//!
//! # Integration with Parser Testing
//!
//! The corpus integrates with `perl-parser` test suites:
//!
//! ```rust,ignore
//! use perl_parser::Parser;
//! use perl_corpus::{parse_dir, find_by_tag};
//!
//! # fn test_parser_with_corpus() -> anyhow::Result<()> {
//! # let corpus_dir = std::path::Path::new("test_corpus");
//! let sections = parse_dir(corpus_dir)?;
//! let regex_cases = find_by_tag(&sections, "regex");
//!
//! for case in regex_cases {
//!     let mut parser = Parser::new(&case.body);
//!     let result = parser.parse();
//!     assert!(result.is_ok(), "Failed to parse: {}", case.title);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Test Case Validation
//!
//! Corpus files can include validation flags:
//!
//! - **`parser-sensitive`**: Requires specific parser version
//! - **`perl-version:5.26`**: Requires Perl 5.26+ features
//! - **`expected-error`**: Test case should produce parse error
//! - **`wip`**: Work in progress, may not parse correctly yet
//!
//! # Contributing Test Cases
//!
//! To add new test cases:
//!
//! 1. Create or edit a corpus file in `test_corpus/`
//! 2. Use section markers (`====`) to separate cases
//! 3. Add metadata tags for categorization
//! 4. Include expected output after `---` separator
//! 5. Run `cargo test` to validate
//!
//! See existing corpus files for examples and conventions.
#![allow(clippy::pedantic)]
// Corpus crate - focus on core clippy lints only
// Lint enforcement: library code must use tracing, not direct stderr/stdout prints.
#![deny(clippy::print_stderr, clippy::print_stdout)]
#![cfg_attr(test, allow(clippy::print_stderr, clippy::print_stdout))]

pub mod api;
pub mod cases;
pub mod codegen;
pub mod concepts;
pub mod continue_redo;
pub mod files;
pub mod fixture_expectations;
pub mod format_statements;
pub mod r#gen;
pub mod glob_expressions;
pub mod gold;
pub mod index;
pub mod inventory;
pub mod lint;
pub mod loading;
pub mod meta;
pub mod metadata;
pub mod metadata_backfill;
pub mod prelude;
pub mod sidecar;
pub mod tie_interface;

pub use api::*;

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::{must, must_some};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        path.push(format!("{}_{}.txt", prefix, nanos));
        path
    }

    #[test]
    fn parse_file_strips_ast_and_generates_id() {
        let path = temp_file("perl_corpus_parse");
        let contents = r#"==========================================
Sample Section
==========================================

my $x = 1;

---
(source_file
  (expression_statement
    (assignment_expression
      (variable_declaration
        (scalar
          (varname)))
      (number))))

==========================================
Tagged Section
==========================================
# @id: custom.id
# @tags: alpha, Beta
# @flags: parser-sensitive
my $y = 2;
"#;

        must(fs::write(&path, contents));
        let sections = must(parse_file(&path));
        must(fs::remove_file(&path));

        assert_eq!(sections.len(), 2);

        // Find the sections by checking their content/ids
        let sample_section = must_some(sections.iter().find(|s| s.body.contains("my $x = 1;")));
        let tagged_section = must_some(sections.iter().find(|s| s.id == "custom.id"));

        assert_eq!(sample_section.body, "my $x = 1;");
        assert!(sample_section.explicit_id.is_none());
        assert!(sample_section.generated_id.is_some());
        assert!(sample_section.expected.is_some());
        assert!(!sample_section.body.contains("---"));
        assert_eq!(tagged_section.id, "custom.id");
        assert_eq!(tagged_section.explicit_id.as_deref(), Some("custom.id"));
        assert_eq!(tagged_section.tags, vec!["alpha".to_string(), "beta".to_string()]);
        assert_eq!(tagged_section.flags, vec!["parser-sensitive".to_string()]);
        assert_eq!(tagged_section.body, "my $y = 2;");
    }
}
