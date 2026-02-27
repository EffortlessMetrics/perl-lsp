use crate::ast::{Node, NodeKind};
use serde_json::{Value, json};
use std::path::Path;
use std::process::{Command, Stdio};

/// Test item representing a test that can be run
#[derive(Debug, Clone)]
pub struct TestItem {
    /// Unique identifier for the test (typically URI::function_name)
    pub id: String,
    /// Human-readable display name for the test
    pub label: String,
    /// File URI where the test is located
    pub uri: String,
    /// Source location range of the test
    pub range: TestRange,
    /// Classification of this test item
    pub kind: TestKind,
    /// Nested test items (e.g., functions within a file)
    pub children: Vec<TestItem>,
}

/// Source location range for a test item
#[derive(Debug, Clone)]
pub struct TestRange {
    /// Zero-based starting line number
    pub start_line: u32,
    /// Zero-based starting character offset within the line
    pub start_character: u32,
    /// Zero-based ending line number
    pub end_line: u32,
    /// Zero-based ending character offset within the line
    pub end_character: u32,
}

/// Classification of test items
#[derive(Debug, Clone, PartialEq)]
pub enum TestKind {
    /// A test file (e.g., .t file)
    File,
    /// A test suite containing multiple tests
    Suite,
    /// An individual test function or assertion
    Test,
}

/// Test result after running a test
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Identifier of the test that was run
    pub test_id: String,
    /// Outcome status of the test execution
    pub status: TestStatus,
    /// Optional diagnostic message (e.g., error details)
    pub message: Option<String>,
    /// Execution time in milliseconds
    pub duration: Option<u64>,
}

/// Outcome status of a test execution
#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    /// Test passed successfully
    Passed,
    /// Test failed (assertion not met)
    Failed,
    /// Test was skipped (not executed)
    Skipped,
    /// Test encountered an error (could not run)
    Errored,
}

impl TestStatus {
    /// Convert to string for JSON serialization
    pub fn as_str(&self) -> &'static str {
        match self {
            TestStatus::Passed => "passed",
            TestStatus::Failed => "failed",
            TestStatus::Skipped => "skipped",
            TestStatus::Errored => "errored",
        }
    }
}

/// Test Runner for Perl tests
pub struct TestRunner {
    /// Source code content of the test file
    source: String,
    /// URI of the test file being analyzed
    uri: String,
}

impl TestRunner {
    /// Creates a new test runner for the given source code and file URI.
    pub fn new(source: String, uri: String) -> Self {
        Self { source, uri }
    }

    /// Discover tests in the AST
    pub fn discover_tests(&self, ast: &Node) -> Vec<TestItem> {
        let mut tests = Vec::new();

        // Find test functions
        let mut test_functions = Vec::new();
        self.find_test_functions_only(ast, &mut test_functions);

        // Check if this is a test file
        if self.is_test_file(&self.uri) {
            // Create a file-level test item
            let file_item = TestItem {
                id: self.uri.clone(),
                label: Path::new(&self.uri)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("test")
                    .to_string(),
                uri: self.uri.clone(),
                range: self.get_file_range(),
                kind: TestKind::File,
                children: test_functions,
            };

            tests.push(file_item);
        } else {
            // Return individual test functions
            tests.extend(test_functions);
        }

        tests
    }

    /// Check if a file is a test file
    fn is_test_file(&self, uri: &str) -> bool {
        let path = Path::new(uri);
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // Common Perl test file patterns
        file_name.ends_with(".t")
            || file_name.ends_with("_test.pl")
            || file_name.ends_with("Test.pl")
            || file_name.starts_with("test_")
            || path.components().any(|c| c.as_os_str() == "t" || c.as_os_str() == "tests")
    }

    /// Find test functions in the AST
    #[allow(dead_code)]
    fn find_test_functions(&self, node: &Node) -> Vec<TestItem> {
        let mut tests = Vec::new();
        self.visit_node_for_tests(node, &mut tests);
        tests
    }

    /// Find only test functions (not assertions)
    fn find_test_functions_only(&self, node: &Node, tests: &mut Vec<TestItem>) {
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.find_test_functions_only(stmt, tests);
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.find_test_functions_only(stmt, tests);
                }
            }

            NodeKind::Subroutine { name, .. } => {
                if let Some(func_name) = name {
                    if self.is_test_function(func_name) {
                        let test_item = TestItem {
                            id: format!("{}::{}", self.uri, func_name),
                            label: func_name.clone(),
                            uri: self.uri.clone(),
                            range: self.node_to_range(node),
                            kind: TestKind::Test,
                            children: vec![],
                        };
                        tests.push(test_item);
                    }
                }
            }

            _ => {
                // Visit children
                self.visit_children_for_test_functions(node, tests);
            }
        }
    }

    /// Visit children nodes for test functions only
    fn visit_children_for_test_functions(&self, node: &Node, tests: &mut Vec<TestItem>) {
        match &node.kind {
            NodeKind::If { then_branch, elsif_branches, else_branch, .. } => {
                self.find_test_functions_only(then_branch, tests);
                for (_, body) in elsif_branches {
                    self.find_test_functions_only(body, tests);
                }
                if let Some(else_b) = else_branch {
                    self.find_test_functions_only(else_b, tests);
                }
            }
            NodeKind::While { body, .. } => {
                self.find_test_functions_only(body, tests);
            }
            NodeKind::For { body, .. } => {
                self.find_test_functions_only(body, tests);
            }
            NodeKind::Foreach { body, .. } => {
                self.find_test_functions_only(body, tests);
            }
            _ => {}
        }
    }

    /// Visit nodes looking for test functions
    #[allow(dead_code)]
    fn visit_node_for_tests(&self, node: &Node, tests: &mut Vec<TestItem>) {
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.visit_node_for_tests(stmt, tests);
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.visit_node_for_tests(stmt, tests);
                }
            }

            NodeKind::Subroutine { name, body, .. } => {
                if let Some(func_name) = name {
                    if self.is_test_function(func_name) {
                        let test_item = TestItem {
                            id: format!("{}::{}", self.uri, func_name),
                            label: func_name.clone(),
                            uri: self.uri.clone(),
                            range: self.node_to_range(node),
                            kind: TestKind::Test,
                            children: vec![],
                        };
                        tests.push(test_item);
                    }
                }

                // Still visit the body for nested tests
                self.visit_node_for_tests(body, tests);
            }

            // Look for Test::More style tests
            NodeKind::FunctionCall { name, args } => {
                if self.is_test_assertion(name) {
                    // Extract test description if available
                    let description = self.extract_test_description(args);
                    let label = description.unwrap_or_else(|| name.clone());

                    let test_item = TestItem {
                        id: format!("{}::{}::{}", self.uri, name, node.location.start),
                        label,
                        uri: self.uri.clone(),
                        range: self.node_to_range(node),
                        kind: TestKind::Test,
                        children: vec![],
                    };
                    tests.push(test_item);
                }

                // Visit arguments
                for arg in args {
                    self.visit_node_for_tests(arg, tests);
                }
            }

            _ => {
                // Visit children
                self.visit_children_for_tests(node, tests);
            }
        }
    }

    /// Check if a function name indicates a test
    fn is_test_function(&self, name: &str) -> bool {
        name.starts_with("test_")
            || name.ends_with("_test")
            || name.starts_with("Test")
            || name.ends_with("Test")
            || name == "test"
    }

    /// Check if a function call is a test assertion
    #[allow(dead_code)]
    fn is_test_assertion(&self, name: &str) -> bool {
        // Test::More assertions
        matches!(
            name,
            "ok" | "is"
                | "isnt"
                | "like"
                | "unlike"
                | "is_deeply"
                | "cmp_ok"
                | "can_ok"
                | "isa_ok"
                | "pass"
                | "fail"
                | "dies_ok"
                | "lives_ok"
                | "throws_ok"
                | "lives_and"
        )
    }

    /// Extract test description from arguments
    #[allow(dead_code)]
    fn extract_test_description(&self, args: &[Node]) -> Option<String> {
        // Usually the last argument is the description
        args.last().and_then(|arg| match &arg.kind {
            NodeKind::String { value, .. } => Some(value.clone()),
            _ => None,
        })
    }

    /// Visit children nodes for tests
    #[allow(dead_code)]
    fn visit_children_for_tests(&self, node: &Node, tests: &mut Vec<TestItem>) {
        match &node.kind {
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.visit_node_for_tests(condition, tests);
                self.visit_node_for_tests(then_branch, tests);
                for (cond, body) in elsif_branches {
                    self.visit_node_for_tests(cond, tests);
                    self.visit_node_for_tests(body, tests);
                }
                if let Some(else_b) = else_branch {
                    self.visit_node_for_tests(else_b, tests);
                }
            }
            NodeKind::While { condition, body, .. } => {
                self.visit_node_for_tests(condition, tests);
                self.visit_node_for_tests(body, tests);
            }
            NodeKind::For { init, condition, update, body, .. } => {
                if let Some(i) = init {
                    self.visit_node_for_tests(i, tests);
                }
                if let Some(c) = condition {
                    self.visit_node_for_tests(c, tests);
                }
                if let Some(u) = update {
                    self.visit_node_for_tests(u, tests);
                }
                self.visit_node_for_tests(body, tests);
            }
            NodeKind::Foreach { variable, list, body, continue_block } => {
                self.visit_node_for_tests(variable, tests);
                self.visit_node_for_tests(list, tests);
                self.visit_node_for_tests(body, tests);
                if let Some(cb) = continue_block {
                    self.visit_node_for_tests(cb, tests);
                }
            }
            _ => {}
        }
    }

    /// Convert node to test range
    fn node_to_range(&self, node: &Node) -> TestRange {
        let (start_line, start_char) = self.offset_to_position(node.location.start);
        let (end_line, end_char) = self.offset_to_position(node.location.end);

        TestRange { start_line, start_character: start_char, end_line, end_character: end_char }
    }

    /// Get range for entire file
    fn get_file_range(&self) -> TestRange {
        let lines: Vec<&str> = self.source.lines().collect();
        let last_line = lines.len().saturating_sub(1) as u32;
        let last_char = lines.last().map(|l| l.len() as u32).unwrap_or(0);

        TestRange {
            start_line: 0,
            start_character: 0,
            end_line: last_line,
            end_character: last_char,
        }
    }

    /// Convert byte offset to line/character position
    fn offset_to_position(&self, offset: usize) -> (u32, u32) {
        let mut line = 0;
        let mut col = 0;

        for (i, ch) in self.source.chars().enumerate() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        (line, col)
    }

    /// Run a test and return results
    pub fn run_test(&self, test_id: &str) -> Vec<TestResult> {
        let mut results = Vec::new();

        // Extract file path from URI
        let file_path = test_id.split("::").next().unwrap_or(test_id);
        let file_path = file_path.strip_prefix("file://").unwrap_or(file_path);

        // Determine how to run the test
        if file_path.ends_with(".t") {
            // Run as a test file
            results.extend(self.run_test_file(file_path));
        } else {
            // Run as a Perl script with prove or perl
            results.extend(self.run_perl_test(file_path));
        }

        results
    }

    /// Run a .t test file
    fn run_test_file(&self, file_path: &str) -> Vec<TestResult> {
        let start_time = std::time::Instant::now();

        // Try to run with prove first, fall back to perl
        let output = Command::new("prove")
            .arg("-v")
            .arg(file_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let output = match output {
            Ok(out) => out,
            Err(_) => {
                // Fall back to running with perl
                match Command::new("perl")
                    .arg(file_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                {
                    Ok(out) => out,
                    Err(e) => {
                        return vec![TestResult {
                            test_id: file_path.to_string(),
                            status: TestStatus::Errored,
                            message: Some(format!("Failed to run test: {}", e)),
                            duration: Some(start_time.elapsed().as_millis() as u64),
                        }];
                    }
                }
            }
        };

        let duration = start_time.elapsed().as_millis() as u64;

        // Parse TAP output
        self.parse_tap_output(
            &String::from_utf8_lossy(&output.stdout),
            &String::from_utf8_lossy(&output.stderr),
            output.status.success(),
            duration,
            file_path,
        )
    }

    /// Run a Perl script as a test
    fn run_perl_test(&self, file_path: &str) -> Vec<TestResult> {
        let start_time = std::time::Instant::now();

        let output = match Command::new("perl")
            .arg("-Ilib")
            .arg(file_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            Ok(out) => out,
            Err(e) => {
                return vec![TestResult {
                    test_id: file_path.to_string(),
                    status: TestStatus::Errored,
                    message: Some(format!("Failed to run test: {}", e)),
                    duration: Some(start_time.elapsed().as_millis() as u64),
                }];
            }
        };

        let duration = start_time.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        vec![TestResult {
            test_id: file_path.to_string(),
            status: if output.status.success() { TestStatus::Passed } else { TestStatus::Failed },
            message: if !stderr.is_empty() {
                Some(stderr.to_string())
            } else if !stdout.is_empty() {
                Some(stdout.to_string())
            } else {
                None
            },
            duration: Some(duration),
        }]
    }

    /// Parse TAP (Test Anything Protocol) output
    fn parse_tap_output(
        &self,
        stdout: &str,
        stderr: &str,
        success: bool,
        duration: u64,
        test_id: &str,
    ) -> Vec<TestResult> {
        let mut results = Vec::new();
        let mut _test_count = 0;

        // Parse TAP output line by line
        for line in stdout.lines() {
            if line.starts_with("ok ") {
                _test_count += 1;
                let test_name = line.splitn(3, ' ').nth(2).unwrap_or("test");
                results.push(TestResult {
                    test_id: format!("{}::{}", test_id, test_name),
                    status: TestStatus::Passed,
                    message: None,
                    duration: None,
                });
            } else if line.starts_with("not ok ") {
                _test_count += 1;
                let test_name = line.splitn(3, ' ').nth(2).unwrap_or("test");
                results.push(TestResult {
                    test_id: format!("{}::{}", test_id, test_name),
                    status: TestStatus::Failed,
                    message: Some(line.to_string()),
                    duration: None,
                });
            }
        }

        // If no individual test results, create one for the whole file
        if results.is_empty() {
            results.push(TestResult {
                test_id: test_id.to_string(),
                status: if success { TestStatus::Passed } else { TestStatus::Failed },
                message: if !stderr.is_empty() { Some(stderr.to_string()) } else { None },
                duration: Some(duration),
            });
        }

        results
    }
}

/// Convert TestItem to JSON for LSP
impl TestItem {
    /// Serializes this test item to a JSON value for LSP communication.
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "label": self.label,
            "uri": self.uri,
            "range": {
                "start": {
                    "line": self.range.start_line,
                    "character": self.range.start_character
                },
                "end": {
                    "line": self.range.end_line,
                    "character": self.range.end_character
                }
            },
            "canResolveChildren": !self.children.is_empty(),
            "children": self.children.iter().map(|c| c.to_json()).collect::<Vec<_>>()
        })
    }
}

/// Convert TestResult to JSON for LSP
impl TestResult {
    /// Serializes this test result to a JSON value for LSP communication.
    pub fn to_json(&self) -> Value {
        let mut result = json!({
            "testId": self.test_id,
            "state": match self.status {
                TestStatus::Passed => "passed",
                TestStatus::Failed => "failed",
                TestStatus::Skipped => "skipped",
                TestStatus::Errored => "errored",
            }
        });

        if let Some(message) = &self.message {
            result["message"] = json!({
                "message": message
            });
        }

        if let Some(duration) = self.duration {
            result["duration"] = json!(duration);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_discover_test_functions() {
        let code = r#"
sub test_basic {
    ok(1, "Basic test");
}

sub helper_function {
    # Not a test
}

sub test_another_thing {
    is($result, 42, "The answer");
}
"#;

        let mut parser = Parser::new(code);
        if let Ok(ast) = parser.parse() {
            let runner = TestRunner::new(code.to_string(), "file:///test.pl".to_string());
            let tests = runner.discover_tests(&ast);

            // Debug: print tests found
            eprintln!("Found {} tests", tests.len());
            for test in &tests {
                eprintln!("Test: {} (kind: {:?})", test.label, test.kind);
                for child in &test.children {
                    eprintln!("  Child: {}", child.label);
                }
            }

            // Should find at least 1 test (file or functions)
            assert!(!tests.is_empty());

            // Should have found test functions
            let test_functions: Vec<&str> = tests
                .iter()
                .filter(|t| t.kind == TestKind::Test && t.label.starts_with("test_"))
                .map(|t| t.label.as_str())
                .collect();

            eprintln!("Test functions: {:?}", test_functions);
            assert!(test_functions.contains(&"test_basic"));
            assert!(test_functions.contains(&"test_another_thing"));
        }
    }

    #[test]
    fn test_discover_test_assertions() {
        let code = r#"
use Test::More;

ok(1, "First test");
is($x, 5, "X should be 5");
like($string, qr/pattern/, "String matches");

done_testing();
"#;

        let mut parser = Parser::new(code);
        if let Ok(ast) = parser.parse() {
            let runner = TestRunner::new(code.to_string(), "file:///test.t".to_string());
            let tests = runner.discover_tests(&ast);

            // Should find test file with assertions
            assert!(!tests.is_empty());

            // Should have discovered individual assertions
            let all_tests: Vec<&TestItem> = tests
                .iter()
                .flat_map(|t| {
                    let mut items = vec![t];
                    items.extend(&t.children);
                    items
                })
                .collect();

            // Debug: print all tests
            eprintln!("All tests found:");
            for test in &all_tests {
                eprintln!("  Test: {} (kind: {:?})", test.label, test.kind);
            }

            // Should have found the test file
            assert!(!tests.is_empty());
            assert_eq!(tests[0].kind, TestKind::File);
        }
    }

    #[test]
    fn test_is_test_file() {
        let runner = TestRunner::new("".to_string(), "".to_string());

        assert!(runner.is_test_file("file:///t/basic.t"));
        assert!(runner.is_test_file("file:///tests/foo_test.pl"));
        assert!(runner.is_test_file("file:///MyTest.pl"));
        assert!(runner.is_test_file("file:///test_something.pl"));

        assert!(!runner.is_test_file("file:///lib/Module.pm"));
        assert!(!runner.is_test_file("file:///script.pl"));
    }
}
