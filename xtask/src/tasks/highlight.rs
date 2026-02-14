//! Highlight test task implementation

use crate::types::ScannerType;
use color_eyre::eyre::{Context, Result, eyre};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use walkdir::WalkDir;

/// Highlight expectation from test file comments
#[derive(Clone, Debug)]
#[allow(dead_code)] // Fields retained for future test expansion and compatibility with new test harness
struct HighlightExpectation {
    // Remove unused fields if not referenced in new logic
    // line: usize,
    // column: usize,
    expected_scope: String,
}

/// Highlight test case containing source code and expected highlights
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields retained for future test expansion and compatibility with new test harness
struct HighlightTestCase {
    name: String,
    source: String,
    expectations: Vec<HighlightExpectation>,
}

/// Results of running highlight tests
#[derive(Debug)]
struct HighlightTestResults {
    total: usize,
    passed: usize,
    failed: usize,
    errors: Vec<String>,
}

impl HighlightTestResults {
    fn new() -> Self {
        Self { total: 0, passed: 0, failed: 0, errors: Vec::new() }
    }

    fn add_passed(&mut self) {
        self.total += 1;
        self.passed += 1;
    }

    fn add_failed(&mut self, error: String) {
        self.total += 1;
        self.failed += 1;
        self.errors.push(error);
    }

    fn print_summary(&self) {
        println!("\n📊 Highlight Test Summary:");
        println!("   Total: {}", self.total);
        println!("   Passed: {} ✅", self.passed);
        println!("   Failed: {} ❌", self.failed);

        if !self.errors.is_empty() {
            println!("\n❌ Failed Tests:");
            for error in &self.errors {
                println!("   {}", error);
            }
        }
    }
}

/// Parse a highlight test file into test cases
fn parse_highlight_file(path: &PathBuf) -> Result<Vec<HighlightTestCase>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read highlight file: {}", path.display()))?;

    let mut test_cases = Vec::new();
    let mut current_source = String::new();
    let mut current_expectations = Vec::new();
    let mut test_case_count = 0;

    for (line_num, line) in content.lines().enumerate() {
        if line.starts_with("#") {
            // Parse highlight expectation comment
            if let Some(expectation) = parse_highlight_expectation(line, line_num + 1) {
                current_expectations.push(expectation);
            }
        } else if !line.trim().is_empty() {
            // This is source code
            current_source.push_str(line);
            current_source.push('\n');
        } else if !current_source.trim().is_empty() {
            // Empty line after source code - end of test case
            test_case_count += 1;
            test_cases.push(HighlightTestCase {
                name: format!("Test case {}", test_case_count),
                source: current_source.clone(),
                expectations: current_expectations.clone(),
            });

            // Reset for next test case
            current_source.clear();
            current_expectations.clear();
        }
    }

    // Add the last test case if there's source code
    if !current_source.trim().is_empty() {
        test_case_count += 1;
        test_cases.push(HighlightTestCase {
            name: format!("Test case {}", test_case_count),
            source: current_source,
            expectations: current_expectations,
        });
    }

    Ok(test_cases)
}

/// Parse a highlight expectation from a comment line
fn parse_highlight_expectation(line: &str, _line_num: usize) -> Option<HighlightExpectation> {
    // Parse patterns like:
    // # ^ keyword.operator
    // # <- variable.hash
    // #        ^^^^^^^^^ type

    let line = line.trim_start_matches('#').trim();

    if line.is_empty() {
        return None;
    }

    // Find the position marker (^ or <-)
    let _column;
    let mut expected_scope;

    if let Some(pos) = line.find("<-") {
        // Format: # <- scope
        _column = 1; // Default to first column for <- format
        expected_scope = line[pos + 2..].trim().to_string();
    } else if let Some(pos) = line.find('^') {
        // Format: # ^ scope or #        ^^^^^^^^^ scope
        _column = pos + 1; // +1 because we're counting from 1
        expected_scope = line[pos + 1..].trim().to_string();

        // Remove any repeated ^ characters
        expected_scope = expected_scope.trim_start_matches('^').trim().to_string();
    } else {
        return None;
    }

    if expected_scope.is_empty() {
        return None;
    }

    Some(HighlightExpectation { expected_scope })
}

/// Run a single highlight test case
fn run_highlight_test_case(
    test_case: &HighlightTestCase,
    _scanner: &Option<ScannerType>,
) -> Result<bool> {
    // Parse the source code using tree-sitter-perl
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_perl::language())
        .context("Failed to load tree-sitter-perl language")?;
    let tree = parser
        .parse(&test_case.source, None)
        .ok_or_else(|| eyre!("Failed to parse test source"))?;

    // Load and compile the highlight query
    let query_source =
        fs::read_to_string("queries/highlights.scm").context("Failed to read highlight query")?;
    let query = Query::new(&tree_sitter_perl::language(), &query_source)
        .context("Failed to compile highlight query")?;

    // Execute the query and collect actual capture names
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(&query, tree.root_node(), test_case.source.as_bytes());
    let mut actual_counts: HashMap<String, usize> = HashMap::new();
    while let Some((_match_, capture_index)) = captures.next() {
        let name = query.capture_names()[*capture_index].to_string();
        *actual_counts.entry(name).or_insert(0) += 1;
    }

    // Verify that each expected scope appears at least as many times as specified
    let mut success = true;
    for expectation in &test_case.expectations {
        match actual_counts.get_mut(&expectation.expected_scope) {
            Some(count) if *count > 0 => {
                *count -= 1;
            }
            _ => {
                success = false;
            }
        }
    }

    Ok(success)
}

pub fn run(path: PathBuf, scanner: Option<ScannerType>) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner().template("{spinner:.green} {wide_msg}")?,
    );

    spinner.set_message("Running highlight tests");

    // Find all highlight test files
    let highlight_path =
        if path.exists() { path } else { PathBuf::from("crates/tree-sitter-perl/test/highlight") };

    if !highlight_path.exists() {
        spinner.finish_with_message("❌ Highlight directory not found");
        return Err(color_eyre::eyre::eyre!(
            "Highlight directory not found: {}",
            highlight_path.display()
        ));
    }

    let mut results = HighlightTestResults::new();

    // Process each highlight file
    for entry in WalkDir::new(&highlight_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "pm"))
    {
        let file_path = entry.path();
        let file_name = file_path.file_name().map_or_else(|| "unknown".into(), |n| n.to_string_lossy());

        spinner.set_message(format!("Processing {}", file_name));

        match parse_highlight_file(&file_path.to_path_buf()) {
            Ok(test_cases) => {
                for test_case in test_cases {
                    match run_highlight_test_case(&test_case, &scanner) {
                        Ok(true) => {
                            results.add_passed();
                        }
                        Ok(false) => {
                            results.add_failed(format!("{}: {}", file_name, test_case.name));
                        }
                        Err(e) => {
                            results.add_failed(format!(
                                "{}: {} - Error: {}",
                                file_name, test_case.name, e
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                results.add_failed(format!("{} - Parse error: {}", file_name, e));
            }
        }
    }

    spinner.finish_with_message("✅ Highlight tests completed");

    // Print summary
    results.print_summary();

    if results.failed > 0 {
        Err(color_eyre::eyre::eyre!("{} highlight tests failed", results.failed))
    } else {
        Ok(())
    }
}
