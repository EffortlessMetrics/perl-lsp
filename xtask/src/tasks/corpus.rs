//! Corpus test task implementation

use crate::types::ScannerType;
use color_eyre::eyre::{Context, Result};
use difference::Changeset;
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
    // Parse the source code using the specified scanner
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
        Some(ScannerType::V3) => {
            let mut parser = perl_parser::Parser::new(&test_case.source);
            match parser.parse() {
                Ok(ast) => ast.to_sexp(),
                Err(e) => format!("(ERROR {})", e),
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
            let diff = Changeset::new(&c_raw, &v3_raw, "\n");
            println!("Diff between C and V3:\n{}", diff);
            return Ok(false);
        }
        None => {
            let mut parser = perl_parser::Parser::new(&test_case.source);
            match parser.parse() {
                Ok(ast) => ast.to_sexp(),
                Err(e) => format!("(ERROR {})", e),
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

/// Count nodes in an S-expression
fn count_nodes(sexp: &str) -> usize {
    sexp.chars().filter(|&c| c == '(').count()
}

/// Find nodes that exist in expected but not in actual
fn find_missing_nodes(expected: &str, actual: &str) -> Vec<String> {
    let expected_tokens: Vec<&str> =
        expected.split_whitespace().filter(|s| !s.is_empty() && s.starts_with('(')).collect();

    let actual_tokens: Vec<&str> =
        actual.split_whitespace().filter(|s| !s.is_empty() && s.starts_with('(')).collect();

    expected_tokens
        .into_iter()
        .filter(|token| !actual_tokens.contains(token))
        .map(|s| s.to_string())
        .collect()
}

/// Find nodes that exist in actual but not in expected
fn find_extra_nodes(expected: &str, actual: &str) -> Vec<String> {
    find_missing_nodes(actual, expected)
}

/// Run corpus tests with a specific scanner configuration
pub fn run(path: PathBuf, scanner: Option<ScannerType>, diagnose: bool, test: bool) -> Result<()> {
    // Use provided path or default to corpus directory
    let corpus_dir = if path.to_string_lossy() == "c/test/corpus" {
        PathBuf::from("crates/perl-corpus/corpus")
    } else {
        path
    };

    if !corpus_dir.exists() && !test {
        return Err(color_eyre::eyre::eyre!(
            "Corpus directory not found: {}",
            corpus_dir.display()
        ));
    }

    let scanner_name = match &scanner {
        Some(ScannerType::C) => "C scanner",
        Some(ScannerType::Rust) => "Rust scanner",
        Some(ScannerType::V3) => "V3 parser",
        Some(ScannerType::Both) => "Both scanners (C vs V3)",
        None => "Default parser (V3)",
    };

    if test {
        println!("🧪 Running simple expression tests with {}", scanner_name);
        return run_simple_tests(scanner, diagnose);
    }

    println!("🧪 Running corpus tests with {}", scanner_name);

    let mut results = CorpusTestResults::new();
    let mut progress_bar = None;

    // Collect all test files
    let test_files: Vec<PathBuf> = WalkDir::new(&corpus_dir)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "txt" { Some(path.to_path_buf()) } else { None }
        })
        .collect();

    if !diagnose {
        progress_bar = Some(ProgressBar::new(test_files.len() as u64));
        if let Some(pb) = &progress_bar {
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{wide_bar:.cyan/blue} {pos:>7}/{len:7} {msg}")
                    .unwrap(),
            );
        }
    }

    for test_file in test_files {
        let test_cases = parse_corpus_file(&test_file)?;

        for test_case in test_cases {
            if diagnose {
                diagnose_parse_differences(&test_case, &scanner)?;
            } else {
                match run_corpus_test_case(&test_case, &scanner)? {
                    true => results.add_passed(),
                    false => results.add_failed(test_case.name),
                }
            }
        }

        if let Some(pb) = &progress_bar {
            pb.inc(1);
            pb.set_message(format!(
                "Processing {}",
                test_file.file_name().unwrap().to_string_lossy()
            ));
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_with_message("✅ Corpus tests completed");
    }

    if !diagnose {
        results.print_summary();

        if results.failed > 0 {
            return Err(color_eyre::eyre::eyre!(
                "❌ {} out of {} corpus tests failed",
                results.failed,
                results.total
            ));
        }

        println!("✅ All corpus tests passed!");
    }

    Ok(())
}

/// Run simple expression tests for quick validation
fn run_simple_tests(scanner: Option<ScannerType>, diagnose: bool) -> Result<()> {
    let test_cases = vec![
        CorpusTestCase {
            name: "Simple variable".to_string(),
            source: "$x".to_string(),
            expected: "(program (expression (variable (scalar_variable))))".to_string(),
        },
        CorpusTestCase {
            name: "String literal".to_string(),
            source: "\"hello\"".to_string(),
            expected: "(program (expression (string_literal)))".to_string(),
        },
        CorpusTestCase {
            name: "Simple assignment".to_string(),
            source: "$x = 42".to_string(),
            expected: "(program (expression (binary_expression (variable (scalar_variable)) (assignment_operator) (expression (number)))))".to_string(),
        },
    ];

    println!("Running {} simple test cases...", test_cases.len());
    let mut passed = 0;

    for test_case in test_cases {
        if diagnose {
            diagnose_parse_differences(&test_case, &scanner)?;
        } else {
            match run_corpus_test_case(&test_case, &scanner)? {
                true => {
                    passed += 1;
                    println!("✅ {}", test_case.name);
                }
                false => {
                    println!("❌ {}", test_case.name);
                }
            }
        }
    }

    if !diagnose {
        println!("\n📊 Simple Test Summary: {}/3 passed", passed);
        if passed == 3 {
            println!("✅ All simple tests passed!");
        } else {
            println!("❌ Some simple tests failed");
        }
    }

    Ok(())
}
