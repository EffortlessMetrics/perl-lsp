//! Corpus test loader
//!
//! Loads test cases from the tree-sitter corpus format

use crate::TestCase;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Parse a tree-sitter corpus file into test cases
pub fn parse_corpus_file(content: &str) -> Result<Vec<TestCase>> {
    let mut tests = Vec::new();
    let mut current_test: Option<TestCase> = None;
    let mut state = ParseState::SeekingTestName;
    let mut input_lines = Vec::new();
    let mut output_lines = Vec::new();

    for line in content.lines() {
        match state {
            ParseState::SeekingTestName => {
                if line.starts_with("===") {
                    state = ParseState::ReadingTestName;
                }
            }
            ParseState::ReadingTestName => {
                if line.starts_with("===") {
                    // If no test name was found, create a test with empty name
                    if current_test.is_none() {
                        current_test = Some(TestCase {
                            name: String::new(),
                            input: String::new(),
                            description: None,
                            should_parse: true,
                            expected_sexp: None,
                        });
                    }
                    state = ParseState::ReadingInput;
                } else if !line.trim().is_empty() {
                    current_test = Some(TestCase {
                        name: line.trim().to_string(),
                        input: String::new(),
                        description: None,
                        should_parse: true,
                        expected_sexp: None,
                    });
                }
            }
            ParseState::ReadingInput => {
                if line.starts_with("---") {
                    state = ParseState::ReadingOutput;
                } else {
                    input_lines.push(line);
                }
            }
            ParseState::ReadingOutput => {
                if line.starts_with("===") {
                    // End of current test, start of new test
                    if let Some(mut test) = current_test.take() {
                        test.input = input_lines.join("\n");
                        test.expected_sexp =
                            if output_lines.is_empty() { None } else { Some(output_lines.join("\n")) };
                        tests.push(test);
                    }

                    // Reset for new test
                    input_lines.clear();
                    output_lines.clear();

                    let name = line.trim_start_matches('=').trim();
                    current_test = Some(TestCase {
                        name: name.to_string(),
                        input: String::new(),
                        description: None,
                        should_parse: true,
                        expected_sexp: None,
                    });
                    state = ParseState::ReadingTestName;
                } else if !line.trim().is_empty() {
                    output_lines.push(line);
                }
            }
        }
    }

    // Handle last test
    if let Some(mut test) = current_test {
        test.input = input_lines.join("\n");
        test.expected_sexp =
            if output_lines.is_empty() { None } else { Some(output_lines.join("\n")) };
        tests.push(test);
    }

    Ok(tests)
}

#[derive(Debug, Clone, Copy)]
enum ParseState {
    SeekingTestName,
    ReadingTestName,
    ReadingInput,
    ReadingOutput,
}

/// Load all corpus tests from a directory
pub fn load_corpus_directory(dir: &Path) -> Result<Vec<TestCase>> {
    let mut all_tests = Vec::new();

    for entry in fs::read_dir(dir).context("Failed to read corpus directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("txt") {
            let content =
                fs::read_to_string(&path).with_context(|| format!("Failed to read {:?}", path))?;

            let mut tests = parse_corpus_file(&content)
                .with_context(|| format!("Failed to parse {:?}", path))?;

            // Add filename to test names for uniqueness
            let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

            for test in &mut tests {
                test.name = format!("{}::{}", filename, test.name);
            }

            all_tests.extend(tests);
        }
    }

    Ok(all_tests)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_corpus_file() {
        // Format matches actual tree-sitter corpus files
        let content = r#"================================================================================
simple test
================================================================================
my $x = 42;
--------------------------------------------------------------------------------

(source_file
  (expression_statement
    (assignment_expression)))
"#;

        let tests = parse_corpus_file(content).expect("Failed to parse corpus");
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "simple test");
        assert_eq!(tests[0].input, "my $x = 42;");
        assert!(tests[0].expected_sexp.is_some());
        assert!(tests[0].expected_sexp.as_ref().unwrap().contains("source_file"));
    }

    #[test]
    fn test_parse_multiple_tests() {
        let content = r#"================================================================================
test one
================================================================================
print "hello";
--------------------------------------------------------------------------------

(source_file (expression_statement))

================================================================================
test two
================================================================================
my $var;
--------------------------------------------------------------------------------

(source_file (variable_declaration))
"#;

        let tests = parse_corpus_file(content).expect("Failed to parse corpus");
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].name, "test one");
        assert_eq!(tests[0].input, "print \"hello\";");
        assert_eq!(tests[1].name, "test two");
        assert_eq!(tests[1].input, "my $var;");
    }

    #[test]
    fn test_parse_test_without_expected_output() {
        let content = r#"================================================================================
no output test
================================================================================
# just a comment
--------------------------------------------------------------------------------
"#;

        let tests = parse_corpus_file(content).expect("Failed to parse corpus");
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "no output test");
        assert_eq!(tests[0].input, "# just a comment");
        assert!(tests[0].expected_sexp.is_none());
    }

    #[test]
    fn test_parse_empty_test_name() {
        let content = r#"================================================================================
================================================================================
my $x;
--------------------------------------------------------------------------------

(source_file)
"#;

        let tests = parse_corpus_file(content).expect("Failed to parse corpus");
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "");
        assert_eq!(tests[0].input, "my $x;");
    }

    #[test]
    fn test_parse_multiline_input() {
        let content = r#"================================================================================
multiline test
================================================================================
if ($condition) {
    print "yes";
}
--------------------------------------------------------------------------------

(source_file (if_statement))
"#;

        let tests = parse_corpus_file(content).expect("Failed to parse corpus");
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "multiline test");
        assert_eq!(tests[0].input, "if ($condition) {\n    print \"yes\";\n}");
    }

    #[test]
    fn test_parse_empty_file() {
        let content = "";
        let tests = parse_corpus_file(content).expect("Failed to parse empty corpus");
        assert_eq!(tests.len(), 0);
    }

    #[test]
    fn test_parse_malformed_separator() {
        // Test with lines starting with === but not test separators
        let content = r#"This is not a test
===== this is not a separator =====
some content
"#;

        let tests = parse_corpus_file(content).expect("Failed to parse corpus");
        // Should handle gracefully - might not parse as expected but shouldn't panic
        assert!(tests.len() <= 1);
    }
}
