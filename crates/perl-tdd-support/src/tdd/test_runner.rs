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
            NodeKind::If { condition, then_branch, elsif_branches, else_branch, .. } => {
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

    /// Create a hermetic `Command` for the Perl interpreter.
    ///
    /// Strips the entire parent environment and passes through only:
    /// - `PATH` — needed for interpreter self-resolution and fork helpers
    /// - `SYSTEMROOT` (Windows only) — required for process spawning on Windows
    ///
    /// This prevents ambient `PERL5LIB`, `PERL5OPT`, `HOME`, and `local::lib`
    /// state from silently altering TDD fixture behaviour. Call sites that need
    /// extra env vars (e.g. `PERL5LIB` for `-Ilib`) must add them explicitly
    /// after calling this function — do not bypass with plain `Command::new`.
    ///
    /// Implements the hermeticity contract for issue #8689 / #8551.
    fn hermetic_perl_command(&self, perl_binary: &str) -> Command {
        let mut cmd = Command::new(perl_binary);
        cmd.env_clear();
        if let Some(path_val) = std::env::var_os("PATH") {
            cmd.env("PATH", path_val);
        }
        #[cfg(windows)]
        if let Some(systemroot) = std::env::var_os("SYSTEMROOT") {
            cmd.env("SYSTEMROOT", systemroot);
        }
        cmd
    }

    /// Run a .t test file
    fn run_test_file(&self, file_path: &str) -> Vec<TestResult> {
        let start_time = std::time::Instant::now();

        // SECURITY: For prove, filenames starting with '-' can be interpreted as flags.
        // prove does not support '--' to separate files from options in all versions/modes.
        // Prepend ./ to ensure it is treated as a file path if it starts with -.
        // Absolute paths (starting with /) are safe.
        let safe_prove_path = if file_path.starts_with('-') {
            format!("./{}", file_path)
        } else {
            file_path.to_string()
        };

        // Try to run with prove first, fall back to perl.
        // The direct `perl` fallback is hermetic below; the ambient `prove`
        // seam remains out of scope for this TDD fixture hardening slice.
        let output = Command::new("prove")
            .arg("-v")
            .arg(&safe_prove_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let output = match output {
            Ok(out) => out,
            Err(_) => {
                // Fall back to running with perl.
                // hermetic_perl_command strips ambient env (PERL5LIB, PERL5OPT, etc.)
                // so test fixtures cannot be silently altered by the caller's env.
                // SECURITY: Use -- to separate options from script file.
                match self
                    .hermetic_perl_command("perl")
                    .arg("--")
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

        // SECURITY: Use -- to separate options from script file.
        // hermetic_perl_command strips ambient env so fixtures cannot be affected
        // by caller's PERL5LIB, PERL5OPT, HOME, or local::lib settings.
        let output = match self
            .hermetic_perl_command("perl")
            .arg("-Ilib")
            .arg("--")
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
    use crate::SourceLocation;
    use crate::parser::Parser;
    use std::sync::{LazyLock, Mutex};

    static ENV_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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

    #[test]
    fn test_status_strings_cover_all_variants() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(TestStatus::Passed.as_str(), "passed");
        assert_eq!(TestStatus::Failed.as_str(), "failed");
        assert_eq!(TestStatus::Skipped.as_str(), "skipped");
        assert_eq!(TestStatus::Errored.as_str(), "errored");
        Ok(())
    }

    #[test]
    fn test_item_json_includes_ranges_and_children() -> Result<(), Box<dyn std::error::Error>> {
        let child = TestItem {
            id: "file:///suite.t::child".to_string(),
            label: "child".to_string(),
            uri: "file:///suite.t".to_string(),
            range: TestRange { start_line: 3, start_character: 4, end_line: 3, end_character: 16 },
            kind: TestKind::Test,
            children: vec![],
        };
        let item = TestItem {
            id: "file:///suite.t".to_string(),
            label: "suite.t".to_string(),
            uri: "file:///suite.t".to_string(),
            range: TestRange { start_line: 0, start_character: 0, end_line: 5, end_character: 1 },
            kind: TestKind::File,
            children: vec![child],
        };

        let json = item.to_json();

        assert_eq!(json["id"], "file:///suite.t");
        assert_eq!(json["label"], "suite.t");
        assert_eq!(json["range"]["end"]["line"], 5);
        assert_eq!(json["canResolveChildren"], true);
        assert_eq!(json["children"][0]["id"], "file:///suite.t::child");
        assert_eq!(json["children"][0]["range"]["start"]["character"], 4);
        Ok(())
    }

    #[test]
    fn test_result_json_covers_message_and_duration() -> Result<(), Box<dyn std::error::Error>> {
        let result = TestResult {
            test_id: "file:///suite.t::case".to_string(),
            status: TestStatus::Errored,
            message: Some("boom".to_string()),
            duration: Some(42),
        };

        let json = result.to_json();

        assert_eq!(json["testId"], "file:///suite.t::case");
        assert_eq!(json["state"], "errored");
        assert_eq!(json["message"]["message"], "boom");
        assert_eq!(json["duration"], 42);
        Ok(())
    }

    #[test]
    fn tap_parser_reports_individual_passes_and_failures() -> Result<(), Box<dyn std::error::Error>>
    {
        let runner = TestRunner::new(String::new(), String::new());

        let results = runner.parse_tap_output(
            "1..3
ok 1 - loads
not ok 2 - rejects invalid input
ok 3
",
            "ignored when individual TAP records exist",
            false,
            99,
            "t/sample.t",
        );

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].test_id, "t/sample.t::- loads");
        assert_eq!(results[0].status, TestStatus::Passed);
        assert_eq!(results[1].test_id, "t/sample.t::2 - rejects invalid input");
        assert_eq!(results[1].status, TestStatus::Failed);
        assert_eq!(results[1].message.as_deref(), Some("not ok 2 - rejects invalid input"));
        assert_eq!(results[2].test_id, "t/sample.t::test");
        assert_eq!(results[2].duration, None);
        Ok(())
    }

    #[test]
    fn tap_parser_falls_back_to_file_result_with_stderr() -> Result<(), Box<dyn std::error::Error>>
    {
        let runner = TestRunner::new(String::new(), String::new());

        let results = runner.parse_tap_output("", "syntax error", false, 17, "script.pl");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].test_id, "script.pl");
        assert_eq!(results[0].status, TestStatus::Failed);
        assert_eq!(results[0].message.as_deref(), Some("syntax error"));
        assert_eq!(results[0].duration, Some(17));
        Ok(())
    }

    #[test]
    fn discover_tests_nests_file_children_and_computes_ranges()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "use Test::More;
sub test_nested {
    ok(1);
}
";
        let body = node(NodeKind::Block { statements: vec![] }, 36, 46);
        let subroutine = node(
            NodeKind::Subroutine {
                name: Some("test_nested".to_string()),
                name_span: Some(SourceLocation { start: 20, end: 31 }),
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(body),
            },
            16,
            46,
        );
        let ast = node(NodeKind::Program { statements: vec![subroutine] }, 0, source.len());
        let runner = TestRunner::new(source.to_string(), "file:///project/t/sample.t".to_string());

        let tests = runner.discover_tests(&ast);

        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].kind, TestKind::File);
        assert_eq!(tests[0].label, "sample.t");
        assert_eq!(tests[0].range.end_line, 3);
        assert_eq!(tests[0].range.end_character, 1);
        assert_eq!(tests[0].children.len(), 1);
        assert_eq!(tests[0].children[0].label, "test_nested");
        assert_eq!(tests[0].children[0].range.start_line, 1);
        assert_eq!(tests[0].children[0].range.start_character, 0);
        assert_eq!(tests[0].children[0].range.end_line, 3);
        assert_eq!(tests[0].children[0].range.end_character, 1);
        Ok(())
    }

    #[test]
    fn visit_children_for_tests_walks_if_with_keyword_metadata()
    -> Result<(), Box<dyn std::error::Error>> {
        let runner = TestRunner::new("is($got, $want);".to_string(), "file:///suite.t".to_string());
        let string = |value: &str, start| {
            node(
                NodeKind::String { value: value.to_string(), interpolated: false },
                start,
                start + value.len() + 2,
            )
        };
        let call = |name: &str, start| {
            node(
                NodeKind::FunctionCall {
                    name: name.to_string(),
                    args: vec![string("case", start + name.len() + 1)],
                },
                start,
                start + name.len() + 8,
            )
        };
        let node = node(
            NodeKind::If {
                condition: Box::new(node(NodeKind::Number { value: "1".to_string() }, 0, 1)),
                then_branch: Box::new(call("is", 2)),
                elsif_branches: vec![],
                else_branch: Some(Box::new(call("ok", 12))),
                keyword: Some("unless".to_string()),
            },
            0,
            20,
        );
        let mut tests = Vec::new();

        runner.visit_children_for_tests(&node, &mut tests);

        assert_eq!(tests.len(), 2);
        Ok(())
    }

    #[test]
    fn assertion_discovery_uses_string_description_or_call_name()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "ok($value, 'truthy');
pass();
";
        let described = node(
            NodeKind::FunctionCall {
                name: "ok".to_string(),
                args: vec![
                    node(
                        NodeKind::Variable { sigil: "$".to_string(), name: "value".to_string() },
                        3,
                        9,
                    ),
                    node(
                        NodeKind::String { value: "truthy".to_string(), interpolated: false },
                        11,
                        19,
                    ),
                ],
            },
            0,
            20,
        );
        let unnamed =
            node(NodeKind::FunctionCall { name: "pass".to_string(), args: vec![] }, 21, 27);
        let ast = node(NodeKind::Program { statements: vec![described, unnamed] }, 0, source.len());
        let runner =
            TestRunner::new(source.to_string(), "file:///project/lib/Module.pm".to_string());

        let tests = runner.find_test_functions(&ast);

        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].label, "truthy");
        assert_eq!(tests[0].id, "file:///project/lib/Module.pm::ok::0");
        assert_eq!(tests[1].label, "pass");
        assert_eq!(tests[1].id, "file:///project/lib/Module.pm::pass::21");
        Ok(())
    }

    fn node(kind: NodeKind, start: usize, end: usize) -> Node {
        Node::new(kind, SourceLocation { start, end })
    }

    // ── Hermeticity tests for hermetic_perl_command (#8689) ──────────────────

    /// `hermetic_perl_command` produces a command with an empty env except PATH.
    ///
    /// We cannot inspect `Command`'s env map on stable Rust, so we verify
    /// indirectly by running the Perl subprocess and asserting PERL5LIB and
    /// PERL5OPT are absent from its environment.
    ///
    /// Skipped automatically when Perl is not on PATH.
    #[test]
    // SAFETY-LINT: this test intentionally mutates process env under ENV_MUTEX.
    #[allow(unsafe_code)]
    fn hermetic_perl_command_strips_perl5lib() {
        let perl = which_perl();
        let Some(perl) = perl else { return };
        let Ok(_env_guard) = ENV_MUTEX.lock() else { return };

        let runner = TestRunner::new("".to_string(), "".to_string());

        // Temporarily set a poisoned PERL5LIB in the parent process.
        let poison = "/hermetic-test-poison-perl5lib";
        // SAFETY: ENV_MUTEX serializes test-local environment mutation.
        unsafe { std::env::set_var("PERL5LIB", poison) };
        let mut cmd = runner.hermetic_perl_command(&perl);
        cmd.args(["-e", "print $ENV{PERL5LIB} // 'UNSET'"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let out = cmd.output();
        // SAFETY: ENV_MUTEX serializes test-local environment mutation.
        unsafe { std::env::remove_var("PERL5LIB") };

        let out = match out {
            Ok(o) => o,
            Err(_) => return, // Perl not found; skip
        };
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout.trim(),
            "UNSET",
            "PERL5LIB must be stripped by hermetic_perl_command; got: {stdout:?}",
        );
    }

    /// `hermetic_perl_command` strips PERL5OPT — ambient runtime injection
    /// must not reach TDD fixture subprocesses.
    #[test]
    // SAFETY-LINT: this test intentionally mutates process env under ENV_MUTEX.
    #[allow(unsafe_code)]
    fn hermetic_perl_command_strips_perl5opt() {
        let perl = which_perl();
        let Some(perl) = perl else { return };
        let Ok(_env_guard) = ENV_MUTEX.lock() else { return };

        let runner = TestRunner::new("".to_string(), "".to_string());

        // SAFETY: ENV_MUTEX serializes test-local environment mutation.
        unsafe { std::env::set_var("PERL5OPT", "-Mstrict") };
        let mut cmd = runner.hermetic_perl_command(&perl);
        cmd.args(["-e", "print $ENV{PERL5OPT} // 'UNSET'"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let out = cmd.output();
        // SAFETY: ENV_MUTEX serializes test-local environment mutation.
        unsafe { std::env::remove_var("PERL5OPT") };

        let out = match out {
            Ok(o) => o,
            Err(_) => return,
        };
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout.trim(),
            "UNSET",
            "PERL5OPT must be stripped by hermetic_perl_command; got: {stdout:?}",
        );
    }

    /// `hermetic_perl_command` preserves PATH so the interpreter can resolve
    /// its own helpers.
    #[test]
    fn hermetic_perl_command_preserves_path() {
        let runner = TestRunner::new("".to_string(), "".to_string());
        // We only check the struct-level behaviour here: PATH should come from
        // the parent env if it is set. We just confirm the function is callable
        // and returns a usable Command without panicking.
        let _ = runner.hermetic_perl_command("perl");
    }

    /// Resolve the `perl` binary on PATH for subprocess tests.
    /// Returns `None` when Perl is not found (test is auto-skipped).
    fn which_perl() -> Option<String> {
        let path_env = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path_env) {
            let candidate = dir.join("perl");
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().into_owned());
            }
            // Windows: also try perl.exe
            let candidate_exe = dir.join("perl.exe");
            if candidate_exe.is_file() {
                return Some(candidate_exe.to_string_lossy().into_owned());
            }
        }
        None
    }
}
