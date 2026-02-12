//! Corpus test task implementation

use crate::types::ScannerType;
use color_eyre::eyre::{Context, Result};
use similar::{ChangeTag, TextDiff};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Corpus test case containing input code and expected S-expression
#[derive(Debug)]
struct CorpusTestCase {
    name: String,
    source: String,
    expected: String,
}

/// Results of running corpus tests
#[derive(Debug)]
struct CorpusTestResults {
    total: usize,
    passed: usize,
    failed: usize,
    errors: Vec<String>,
}

impl CorpusTestResults {
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
        println!("\n📊 Corpus Test Summary:");
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

/// Parse a corpus test file into individual test cases
fn parse_corpus_file(path: &PathBuf) -> Result<Vec<CorpusTestCase>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read corpus file: {}", path.display()))?;

    let mut test_cases = Vec::new();
    let mut current_name = String::new();
    let mut current_source = String::new();
    let mut current_expected = String::new();
    let mut in_source = false;
    let mut in_expected = false;

    let mut seen_closing_separator = false;

    for line in content.lines() {
        if line.starts_with(
            "================================================================================",
        ) {
            if seen_closing_separator {
                // Save previous test case if we have one
                if !current_name.is_empty()
                    && !current_source.is_empty()
                    && !current_expected.is_empty()
                {
                    test_cases.push(CorpusTestCase {
                        name: current_name.clone(),
                        source: current_source.trim().to_string(),
                        expected: current_expected.trim().to_string(),
                    });
                }

                // Reset for new test case
                current_name.clear();
                current_source.clear();
                current_expected.clear();
                in_source = false;
                in_expected = false;
                seen_closing_separator = false;
            } else if !current_name.is_empty() {
                // This is the closing separator for current test case
                seen_closing_separator = true;
                in_source = true;
                in_expected = false;
            }
            // First separator or opening separator - just continue to look for name
        } else if line.starts_with("----") {
            // Transition from source to expected
            in_source = false;
            in_expected = true;
        } else if !current_name.is_empty() && in_source {
            // We're collecting source code
            if !current_source.is_empty() {
                current_source.push('\n');
            }
            current_source.push_str(line);
        } else if in_expected {
            // We're collecting expected output
            if !current_expected.is_empty() {
                current_expected.push('\n');
            }
            current_expected.push_str(line);
        } else if current_name.is_empty() && !line.trim().is_empty() && !line.starts_with("=") {
            // This is the test case name
            current_name = line.trim().to_string();
        }
    }

    // Add the last test case
    if !current_name.is_empty() && !current_source.is_empty() && !current_expected.is_empty() {
        test_cases.push(CorpusTestCase {
            name: current_name,
            source: current_source.trim().to_string(),
            expected: current_expected.trim().to_string(),
        });
    }

    Ok(test_cases)
}

fn normalize_sexp(s: &str) -> String {
    // Normalize by removing all whitespace - we only care about structural equality
    // This handles differences between single-line and multi-line S-expression formats
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Run a single corpus test case
fn run_corpus_test_case(test_case: &CorpusTestCase, scanner: &Option<ScannerType>) -> Result<bool> {
    // Parse the source code using tree-sitter-perl
    let actual_sexp = match scanner {
        Some(ScannerType::C) => {
            // Use the C-based tree-sitter parser
            let tree = tree_sitter_perl::parse(&test_case.source)?;
            tree.root_node().to_sexp()
        }
        Some(ScannerType::Rust) => {
            // Use the pure-rust parser
            let mut parser = tree_sitter_perl::PureRustPerlParser::new();
            match parser.parse(&test_case.source) {
                Ok(ast) => parser.to_sexp(&ast),
                Err(e) => {
                    // Return an error node for failed parses
                    format!("(ERROR {})", e)
                }
            }
        }
        Some(ScannerType::V3) => {
            // Use the perl-parser v3 native parser
            let mut parser = perl_parser::Parser::new(&test_case.source);
            match parser.parse() {
                Ok(ast) => ast.to_sexp(),
                Err(e) => {
                    // Return an error node for failed parses
                    format!("(ERROR {})", e)
                }
            }
        }
        Some(ScannerType::Both) => {
            // Parse using both C and V3 scanners and compare results
            let c_tree = tree_sitter_perl::parse(&test_case.source)?;
            let c_raw = c_tree.root_node().to_sexp();
            let c_sexp = normalize_sexp(&c_raw);

            let mut v3_parser = perl_parser::Parser::new(&test_case.source);
            let v3_raw = match v3_parser.parse() {
                Ok(ast) => ast.to_sexp(),
                Err(e) => format!("(ERROR {})", e),
            };
            let v3_sexp = normalize_sexp(&v3_raw);

            if c_sexp == v3_sexp {
                return Ok(c_sexp == normalize_sexp(test_case.expected.trim()));
            }

            println!("\n❌ Test failed: {}", test_case.name);
            let diff = TextDiff::from_lines(&c_raw, &v3_raw);
            println!("Diff between C and V3:");
            for change in diff.iter_all_changes() {
                match change.tag() {
                    ChangeTag::Equal => print!("{}", change),
                    ChangeTag::Delete => print!("\x1b[91m-{}\x1b[0m", change),
                    ChangeTag::Insert => print!("\x1b[92m+{}\x1b[0m", change),
                }
            }
            println!();
            return Ok(false);
        }
        None => {
            // Default to V3 parser for this branch
            let mut parser = perl_parser::Parser::new(&test_case.source);
            match parser.parse() {
                Ok(ast) => ast.to_sexp(),
                Err(e) => {
                    // Return an error node for failed parses
                    format!("(ERROR {})", e)
                }
            }
        }
    };

    let actual = normalize_sexp(&actual_sexp);
    let expected = normalize_sexp(test_case.expected.trim());

    if actual == expected {
        Ok(true)
    } else {
        println!("\n❌ Test failed: {}", test_case.name);
        println!("Expected:");
        println!("{}", expected);
        println!("Actual:");
        println!("{}", actual);
        Ok(false)
    }
}

/// Diagnostic function to analyze differences between expected and actual S-expressions
fn diagnose_parse_differences(
    test_case: &CorpusTestCase,
    scanner: &Option<ScannerType>,
) -> Result<()> {
    println!("\n🔍 DIAGNOSTIC: {}", test_case.name);
    println!("Input Perl code:");
    println!("```perl");
    println!("{}", test_case.source.trim());
    println!("```");

    // Parse with current parser(s)
    if let Some(ScannerType::Both) = scanner {
        let c_tree = tree_sitter_perl::parse(&test_case.source)?;
        let c_sexp = normalize_sexp(&c_tree.root_node().to_sexp());

        let mut v3_parser = perl_parser::Parser::new(&test_case.source);
        let v3_sexp = match v3_parser.parse() {
            Ok(ast) => normalize_sexp(&ast.to_sexp()),
            Err(e) => format!("(ERROR {})", e),
        };

        println!("\n📊 C scanner S-expression:\n{}", c_sexp);
        println!("\n📊 V3 scanner S-expression:\n{}", v3_sexp);

        println!("\n🔍 STRUCTURAL ANALYSIS:");
        let c_nodes = count_nodes(&c_sexp);
        let v3_nodes = count_nodes(&v3_sexp);
        println!("C scanner nodes: {}", c_nodes);
        println!("V3 scanner nodes: {}", v3_nodes);

        let missing = find_missing_nodes(&c_sexp, &v3_sexp);
        if !missing.is_empty() {
            println!("❌ Nodes missing in V3 output:");
            for node in missing {
                println!("  - {}", node);
            }
        }

        let extra = find_extra_nodes(&c_sexp, &v3_sexp);
        if !extra.is_empty() {
            println!("➕ Extra nodes in V3 output:");
            for node in extra {
                println!("  - {}", node);
            }
        }

        if c_sexp == v3_sexp {
            println!("✅ Parsers produce identical S-expressions");
        } else {
            println!("❌ Parsers differ");
        }

        return Ok(());
    }

    let actual_sexp = match scanner {
        Some(ScannerType::C) => {
            let tree = tree_sitter_perl::parse(&test_case.source)?;
            tree.root_node().to_sexp()
        }
        Some(ScannerType::Rust) => {
            let mut parser = tree_sitter_perl::PureRustPerlParser::new();
            match parser.parse(&test_case.source) {
                Ok(ast) => parser.to_sexp(&ast),
                Err(e) => format!("(ERROR {})", e),
            }
        }
        Some(ScannerType::V3) | None => {
            let mut parser = perl_parser::Parser::new(&test_case.source);
            match parser.parse() {
                Ok(ast) => ast.to_sexp(),
                Err(e) => format!("(ERROR {})", e),
            }
        }
        Some(ScannerType::Both) => unreachable!(),
    };

    let actual = normalize_sexp(&actual_sexp);
    let expected = normalize_sexp(test_case.expected.trim());

    println!("\n📊 COMPARISON:");
    println!("Expected S-expression:");
    println!("{}", expected);
    println!("\nActual S-expression:");
    println!("{}", actual);

    println!("\n🔍 STRUCTURAL ANALYSIS:");

    let expected_nodes = count_nodes(&expected);
    let actual_nodes = count_nodes(&actual);
    println!("Expected nodes: {}", expected_nodes);
    println!("Actual nodes: {}", actual_nodes);

    let missing_nodes = find_missing_nodes(&expected, &actual);
    if !missing_nodes.is_empty() {
        println!("❌ Missing nodes in actual output:");
        for node in missing_nodes {
            println!("  - {}", node);
        }
    }

    let extra_nodes = find_extra_nodes(&expected, &actual);
    if !extra_nodes.is_empty() {
        println!("➕ Extra nodes in actual output:");
        for node in extra_nodes {
            println!("  - {}", node);
        }
    }

    if actual == expected {
        println!("✅ Parse trees match exactly");
    } else {
        println!("❌ Parse trees differ structurally");
    }

    Ok(())
}

/// Count the number of nodes in an S-expression
fn count_nodes(sexp: &str) -> usize {
    sexp.chars().filter(|&c| c == '(').count()
}

/// Find nodes that are in expected but not in actual
fn find_missing_nodes(expected: &str, actual: &str) -> Vec<String> {
    let expected_nodes = extract_node_types(expected);
    let actual_nodes = extract_node_types(actual);

    expected_nodes.iter().filter(|node| !actual_nodes.contains(node)).cloned().collect()
}

/// Find nodes that are in actual but not in expected
fn find_extra_nodes(expected: &str, actual: &str) -> Vec<String> {
    let expected_nodes = extract_node_types(expected);
    let actual_nodes = extract_node_types(actual);

    actual_nodes.iter().filter(|node| !expected_nodes.contains(node)).cloned().collect()
}

/// Extract node types from S-expression
fn extract_node_types(sexp: &str) -> Vec<String> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let mut in_paren = false;

    for ch in sexp.chars() {
        match ch {
            '(' => {
                in_paren = true;
                current.clear();
            }
            ')' => {
                if in_paren && !current.trim().is_empty() {
                    nodes.push(current.trim().to_string());
                }
                in_paren = false;
            }
            ' ' | '\n' | '\t' => {
                if in_paren && !current.trim().is_empty() {
                    nodes.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => {
                if in_paren {
                    current.push(ch);
                }
            }
        }
    }

    nodes
}

/// Test function to verify current parser behavior
fn test_current_parser() -> Result<()> {
    println!("\n🧪 TESTING CURRENT PARSER BEHAVIOR:");

    let test_cases = vec![
        "1 + 1",
        "2 * 3",
        "!3",
        "true",
        "# comment",
        // Add the exact failing test cases
        "1 + 1;",
        "# split across\n# multiple lines",
        "",
        "1 ",
        "1 + 2 __END__ this is ignored too",
        "!3;",
        "true;",
    ];

    for source in test_cases {
        println!("\nInput: '{}'", source);
        match tree_sitter_perl::parse(source) {
            Ok(tree) => {
                let sexp = normalize_sexp(&tree.root_node().to_sexp());
                println!("Output: {}", sexp);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    Ok(())
}

pub fn run(path: PathBuf, scanner: Option<ScannerType>, diagnose: bool, test: bool) -> Result<()> {
    // If test mode is requested, run the current parser test
    if test {
        return test_current_parser();
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner().template("{spinner:.green} {wide_msg}").unwrap(),
    );

    spinner.set_message("Running corpus tests");

    // Find all corpus test files
    let corpus_path = if path.exists() { path } else { PathBuf::from("test/corpus") };

    if !corpus_path.exists() {
        spinner.finish_with_message("❌ Corpus directory not found");
        return Err(color_eyre::eyre::eyre!(
            "Corpus directory not found: {}",
            corpus_path.display()
        ));
    }

    let mut results = CorpusTestResults::new();
    let mut diagnostic_run = false;

    // Process each corpus file
    for entry in WalkDir::new(&corpus_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        let file_name = file_path.file_name().unwrap().to_string_lossy();

        // Skip files that are clearly not corpus files
        if file_name.starts_with('_')
            || file_name.ends_with(".md")
            || file_name.starts_with("README")
        {
            continue;
        }

        spinner.set_message(format!("Processing {}", file_name));

        match parse_corpus_file(&file_path.to_path_buf()) {
            Ok(test_cases) => {
                for test_case in test_cases {
                    match run_corpus_test_case(&test_case, &scanner) {
                        Ok(true) => {
                            results.add_passed();
                        }
                        Ok(false) => {
                            results.add_failed(format!("{}: {}", file_name, test_case.name));

                            // Run diagnostic on first failing test if requested
                            if diagnose && !diagnostic_run {
                                if let Err(e) = diagnose_parse_differences(&test_case, &scanner) {
                                    println!("Diagnostic error: {}", e);
                                }
                                diagnostic_run = true;
                            }
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

    spinner.finish_with_message("✅ Corpus tests completed");

    // Print summary
    results.print_summary();

    if results.failed > 0 {
        Err(color_eyre::eyre::eyre!("{} corpus tests failed", results.failed))
    } else {
        Ok(())
    }
}
