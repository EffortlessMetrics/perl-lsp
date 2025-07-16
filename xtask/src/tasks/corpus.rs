//! Corpus test task implementation

use crate::types::ScannerType;
use color_eyre::eyre::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::fs;
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
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            errors: Vec::new(),
        }
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
    
    for line in content.lines() {
        if line.starts_with("================================================================================") {
            // Save previous test case if we have one
            if !current_name.is_empty() && !current_source.is_empty() && !current_expected.is_empty() {
                test_cases.push(CorpusTestCase {
                    name: current_name.clone(),
                    source: current_source.clone(),
                    expected: current_expected.clone(),
                });
            }
            
            // Start new test case
            current_name.clear();
            current_source.clear();
            current_expected.clear();
            in_source = false;
            in_expected = false;
        } else if line.starts_with("----") {
            // Transition from source to expected
            in_source = false;
            in_expected = true;
        } else if in_source {
            current_source.push_str(line);
            current_source.push('\n');
        } else if in_expected {
            current_expected.push_str(line);
            current_expected.push('\n');
        } else if !line.trim().is_empty() && !line.starts_with("=") {
            // This is the test case name
            current_name = line.trim().to_string();
            in_source = true;
        }
    }
    
    // Add the last test case
    if !current_name.is_empty() && !current_source.is_empty() && !current_expected.is_empty() {
        test_cases.push(CorpusTestCase {
            name: current_name,
            source: current_source,
            expected: current_expected,
        });
    }
    
    Ok(test_cases)
}

fn normalize_sexp(s: &str) -> String {
    s.lines()
        .map(|line| line.trim_end())
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Run a single corpus test case
fn run_corpus_test_case(test_case: &CorpusTestCase, scanner: &Option<ScannerType>) -> Result<bool> {
    // Parse the source code using tree-sitter-perl
    let tree = match scanner {
        Some(ScannerType::C) => {
            // TODO: Implement C scanner parsing when C scanner is available
            // For now, use the default (Rust scanner)
            tree_sitter_perl::parse(&test_case.source)?
        }
        Some(ScannerType::Rust) => {
            tree_sitter_perl::parse(&test_case.source)?
        }
        Some(ScannerType::Both) => {
            // TODO: Test both scanners and compare results
            tree_sitter_perl::parse(&test_case.source)?
        }
        None => {
            tree_sitter_perl::parse(&test_case.source)?
        }
    };
    
    let actual = normalize_sexp(&tree.root_node().to_sexp());
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

pub fn run(path: PathBuf, scanner: Option<ScannerType>) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );

    spinner.set_message("Running corpus tests");

    // Find all corpus test files
    let corpus_path = if path.exists() { path } else { PathBuf::from("test/corpus") };
    
    if !corpus_path.exists() {
        spinner.finish_with_message("❌ Corpus directory not found");
        return Err(color_eyre::eyre::eyre!("Corpus directory not found: {}", corpus_path.display()));
    }

    let mut results = CorpusTestResults::new();
    
    // Process each corpus file
    for entry in WalkDir::new(&corpus_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        let file_name = file_path.file_name().unwrap().to_string_lossy();
        
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
                        }
                        Err(e) => {
                            results.add_failed(format!("{}: {} - Error: {}", file_name, test_case.name, e));
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
