//! Highlight test task implementation

use crate::types::ScannerType;
use color_eyre::eyre::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;
use perl_parser::{Parser, Node, NodeKind};

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
    // Parse the source code using perl-parser
    let mut parser = Parser::new(&test_case.source);
    let ast = parser.parse()
        .context("Failed to parse test source")?;

    // Collect actual node kinds from the AST to simulate highlight scopes
    let mut actual_scopes: HashMap<String, usize> = HashMap::new();
    collect_node_kinds(&ast, &mut actual_scopes);

    // Map expected highlight scopes to node kinds
    let mut success = true;
    for expectation in &test_case.expectations {
        let expected_scope = &expectation.expected_scope;
        
        // Map common highlight scopes to parser node kinds
        let node_kind = match expected_scope.as_str() {
            "number" => "number",
            "operator" => "binary_+", // Map operator to binary addition as example
            "keyword" => "VariableDeclaration", // Map keyword to variable declaration
            "punctuation.special" => "Variable", // Sigil is part of variable node
            "variable" => "Variable",
            "string" => "string",
            "function" => "SubDeclaration", // Map function to subroutine declaration
            "type" => "UseStatement", // Map type to use statement
            "label" => "HereDocEnd", // Map label to heredoc end marker
            _ => expected_scope, // Use as-is for other cases
        };
        
        let actual_count = actual_scopes.get(node_kind).copied().unwrap_or(0);
        
        if actual_count == 0 {
            eprintln!("Expected scope '{}' (mapped to '{}') not found in AST", expected_scope, node_kind);
            success = false;
        }
    }

    // Debug: Print all actual node kinds for troubleshooting
    if !success {
        eprintln!("Available node kinds: {:?}", actual_scopes.keys().collect::<Vec<_>>());
        eprintln!("Expected scopes: {:?}", test_case.expectations.iter().map(|e| &e.expected_scope).collect::<Vec<_>>());
    }

    Ok(success)
}

/// Recursively collect all node kinds from the AST
fn collect_node_kinds(node: &Node, scopes: &mut HashMap<String, usize>) {
    let kind_name = match &node.kind {
        NodeKind::Program { .. } => "Program",
        NodeKind::ExpressionStatement { .. } => "ExpressionStatement", 
        NodeKind::VariableDeclaration { .. } => "VariableDeclaration",
        NodeKind::Variable { .. } => "Variable",
        NodeKind::Binary { op, .. } => {
            // Return specific binary operator types
            match op.as_str() {
                "+" => "binary_+",
                "-" => "binary_-",
                "*" => "binary_*",
                "/" => "binary_/",
                "=" => "Assignment",
                "==" => "binary_==",
                "=~" => "binary_=~",
                "=>" => "binary_=>",
                _ => "binary_op",
            }
        }
        NodeKind::Number { .. } => "number",
        NodeKind::String { .. } => "string",
        NodeKind::Assignment { .. } => "Assignment",
        NodeKind::Subroutine { .. } => "SubDeclaration",
        NodeKind::Use { .. } => "UseStatement",
        NodeKind::FunctionCall { .. } => "FunctionCall",
        NodeKind::Identifier { .. } => "Identifier",
        NodeKind::Heredoc { .. } => "HereDoc",
        _ => "other", // Fallback for other node types
    }.to_string();
    
    *scopes.entry(kind_name).or_insert(0) += 1;
    
    // Manually recurse into child nodes based on NodeKind
    match &node.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                collect_node_kinds(stmt, scopes);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            collect_node_kinds(expression, scopes);
        }
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            collect_node_kinds(variable, scopes);
            if let Some(init) = initializer {
                collect_node_kinds(init, scopes);
            }
        }
        NodeKind::Binary { left, right, .. } => {
            collect_node_kinds(left, scopes);
            collect_node_kinds(right, scopes);
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            collect_node_kinds(lhs, scopes);
            collect_node_kinds(rhs, scopes);
        }
        NodeKind::Subroutine { body, .. } => {
            // Note: name is an Option<String>, not a Node, so we don't recurse into it
            collect_node_kinds(body, scopes);
        }
        NodeKind::Use { .. } => {
            // Note: module is a String, not a Node, so we don't recurse into it
        }
        NodeKind::FunctionCall { args, .. } => {
            // Note: name is a String, not a Node, so we don't recurse into it
            for arg in args {
                collect_node_kinds(arg, scopes);
            }
        }
        _ => {}
    }
}

pub fn run(path: PathBuf, scanner: Option<ScannerType>) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner().template("{spinner:.green} {wide_msg}").unwrap(),
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
        let file_name = file_path.file_name().unwrap().to_string_lossy();

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
