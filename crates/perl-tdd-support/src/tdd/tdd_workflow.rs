//! TDD (Test-Driven Development) workflow integration for LSP
//!
//! Provides a complete red-green-refactor cycle support with
//! automatic test generation, continuous testing, and refactoring suggestions.

use crate::ast::Node;

// Re-use Diagnostic from tdd_basic to avoid duplication
use crate::tdd_basic::{Diagnostic, DiagnosticSeverity};
use crate::test_generator::{CoverageReport, TestResults, TestRunner};
use crate::test_generator::{RefactoringSuggester, RefactoringSuggestion};
use crate::test_generator::{TestCase, TestFramework, TestGenerator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// TDD workflow manager
pub struct TddWorkflow {
    /// Test generator
    generator: TestGenerator,
    /// Test runner
    runner: TestRunner,
    /// Refactoring suggester
    suggester: RefactoringSuggester,
    /// Current workflow state
    state: WorkflowState,
    /// Test results cache
    test_cache: HashMap<PathBuf, TestResults>,
    /// Coverage tracking
    coverage_tracker: CoverageTracker,
    /// Configuration
    config: TddConfig,
}

/// Represents the current phase of the TDD (Test-Driven Development) workflow cycle.
///
/// Tracks the developer's position in the red-green-refactor cycle, enabling context-aware
/// suggestions and automation during test development and implementation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkflowState {
    /// Writing test (Red phase)
    Red,
    /// Making test pass (Green phase)
    Green,
    /// Refactoring code (Refactor phase)
    Refactor,
    /// Not in TDD cycle
    Idle,
}

/// Configuration options for TDD workflow automation and behavior.
///
/// Customizes how the TDD workflow manager generates tests, runs them, and provides
/// feedback. All settings have sensible defaults suitable for typical Perl development.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TddConfig {
    /// Automatically generate tests for new code
    pub auto_generate_tests: bool,
    /// Run tests on save
    pub test_on_save: bool,
    /// Show coverage inline
    pub show_inline_coverage: bool,
    /// Test framework preference
    pub test_framework: String,
    /// Test file naming pattern
    pub test_file_pattern: String,
    /// Minimum coverage threshold
    pub coverage_threshold: f64,
    /// Enable continuous testing
    pub continuous_testing: bool,
    /// Auto-suggest refactorings after green
    pub auto_suggest_refactorings: bool,
}

impl Default for TddConfig {
    fn default() -> Self {
        Self {
            auto_generate_tests: true,
            test_on_save: true,
            show_inline_coverage: true,
            test_framework: "Test::More".to_string(),
            test_file_pattern: "t/{name}.t".to_string(),
            coverage_threshold: 80.0,
            continuous_testing: true,
            auto_suggest_refactorings: true,
        }
    }
}

/// Coverage tracking for TDD
pub struct CoverageTracker {
    /// Line coverage data
    line_coverage: HashMap<PathBuf, Vec<LineCoverage>>,
    /// Branch coverage data (architectural placeholder for future implementation)
    #[allow(dead_code)]
    branch_coverage: HashMap<PathBuf, Vec<BranchCoverage>>,
    /// Overall coverage percentage
    total_coverage: f64,
}

/// Line-level code coverage information
#[derive(Debug, Clone)]
pub struct LineCoverage {
    /// Line number in the source file
    pub line: usize,
    /// Number of times this line was executed
    pub hits: usize,
    /// Whether this line is considered covered
    pub covered: bool,
}

/// Branch-level code coverage information
#[derive(Debug, Clone)]
pub struct BranchCoverage {
    /// Line number where the branch occurs
    pub line: usize,
    /// Unique identifier for this branch within the line
    pub branch_id: usize,
    /// Whether this branch was taken at least once
    pub taken: bool,
    /// Number of times this branch was executed
    pub hits: usize,
}

impl TddWorkflow {
    /// Create a new TDD workflow manager with the given configuration
    pub fn new(config: TddConfig) -> Self {
        let framework = match config.test_framework.as_str() {
            "Test2::V0" => TestFramework::Test2V0,
            "Test::Simple" => TestFramework::TestSimple,
            "Test::Class" => TestFramework::TestClass,
            _ => TestFramework::TestMore,
        };

        Self {
            generator: TestGenerator::new(framework),
            runner: TestRunner::new(),
            suggester: RefactoringSuggester::new(),
            state: WorkflowState::Idle,
            test_cache: HashMap::new(),
            coverage_tracker: CoverageTracker::new(),
            config,
        }
    }

    /// Start a new TDD cycle
    pub fn start_cycle(&mut self, test_name: &str) -> TddCycleResult {
        self.state = WorkflowState::Red;

        TddCycleResult {
            phase: "Red".to_string(),
            message: format!("Starting TDD cycle for '{}'", test_name),
            actions: vec![
                TddAction::GenerateTest(test_name.to_string()),
                TddAction::CreateTestFile(self.get_test_file_path(test_name)),
            ],
        }
    }

    /// Generate tests for the given code
    pub fn generate_tests(&self, ast: &Node, source: &str) -> Vec<TestCase> {
        self.generator.generate_tests(ast, source)
    }

    /// Generate a specific test type
    pub fn generate_test_for_function(
        &self,
        function_name: &str,
        params: &[String],
        test_type: TestType,
    ) -> TestCase {
        let test_name = format!("test_{}_{:?}", function_name, test_type);
        let description = format!("{:?} test for {}", test_type, function_name);

        let code = match test_type {
            TestType::Basic => self.generate_basic_test(function_name, params),
            TestType::EdgeCase => self.generate_edge_case_test(function_name, params),
            TestType::ErrorHandling => self.generate_error_test(function_name, params),
            TestType::Performance => self.generate_performance_test(function_name),
            TestType::Integration => self.generate_integration_test(function_name, params),
        };

        TestCase {
            name: test_name,
            description,
            code,
            is_todo: matches!(test_type, TestType::Integration | TestType::Performance),
        }
    }

    fn generate_basic_test(&self, name: &str, params: &[String]) -> String {
        let args = params
            .iter()
            .enumerate()
            .map(|(i, _)| format!("'test_value_{}'", i))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "use Test::More;\n\n\
             subtest '{}' => sub {{\n    \
             my $result = {}({});\n    \
             ok(defined $result, 'Returns defined value');\n    \
             # PENDING: Add specific assertions\n\
             }};\n\n\
             done_testing();\n",
            name, name, args
        )
    }

    fn generate_edge_case_test(&self, name: &str, _params: &[String]) -> String {
        format!(
            "use Test::More;\n\n\
             subtest '{} edge cases' => sub {{\n    \
             # Test with undef\n    \
             eval {{ {}(undef) }};\n    \
             ok(!$@, 'Handles undef');\n    \n    \
             # Test with empty values\n    \
             eval {{ {}('') }};\n    \
             ok(!$@, 'Handles empty string');\n    \n    \
             # Test with special characters\n    \
             eval {{ {}(\"\\n\\t\\0\") }};\n    \
             ok(!$@, 'Handles special characters');\n\
             }};\n\n\
             done_testing();\n",
            name, name, name, name
        )
    }

    fn generate_error_test(&self, name: &str, _params: &[String]) -> String {
        format!(
            "use Test::More;\n\
             use Test::Exception;\n\n\
             subtest '{} error handling' => sub {{\n    \
             # Test that errors are caught\n    \
             dies_ok {{ {}(undef, undef, undef) }} 'Dies on invalid input';\n    \n    \
             # Test error message\n    \
             throws_ok {{ {}() }} qr/required/, 'Correct error message';\n\
             }};\n\n\
             done_testing();\n",
            name, name, name
        )
    }

    fn generate_performance_test(&self, name: &str) -> String {
        format!(
            "use Test::More;\n\
             use Benchmark qw(timethis);\n\n\
             subtest '{} performance' => sub {{\n    \
             my $iterations = 10000;\n    \
             my $result = timethis($iterations, sub {{ {}() }});\n    \
             \n    \
             # Check performance threshold\n    \
             my $rate = $result->iters / $result->cpu_a;\n    \
             cmp_ok($rate, '>', 1000, 'Performance exceeds 1000 ops/sec');\n\
             }};\n\n\
             done_testing();\n",
            name, name
        )
    }

    fn generate_integration_test(&self, name: &str, _params: &[String]) -> String {
        format!(
            "use Test::More;\n\n\
             subtest '{} integration' => sub {{\n    \
             # PENDING: Set up test environment\n    \
             # PENDING: Call {} with real dependencies\n    \
             # PENDING: Verify integration points\n    \
             pass('Integration test placeholder');\n\
             }};\n\n\
             done_testing();\n",
            name, name
        )
    }

    /// Run tests and update state
    pub fn run_tests(&mut self, test_files: &[PathBuf]) -> TddCycleResult {
        let file_strings: Vec<String> =
            test_files.iter().map(|p| p.to_string_lossy().to_string()).collect();

        let results = self.runner.run_tests(&file_strings);

        // Cache results
        for file in test_files {
            self.test_cache.insert(file.clone(), results.clone());
        }

        // Update state based on results
        let (new_state, message) = if results.failed > 0 {
            (WorkflowState::Red, format!("{} tests failed", results.failed))
        } else if results.todo > 0 {
            (WorkflowState::Green, format!("All tests pass, {} TODOs remaining", results.todo))
        } else {
            (WorkflowState::Refactor, "All tests pass! Ready to refactor".to_string())
        };

        self.state = new_state.clone();

        let mut actions = vec![];

        // Suggest refactorings if all tests pass
        if new_state == WorkflowState::Refactor && self.config.auto_suggest_refactorings {
            actions.push(TddAction::SuggestRefactorings);
        }

        TddCycleResult { phase: format!("{:?}", new_state), message, actions }
    }

    /// Get refactoring suggestions
    pub fn get_refactoring_suggestions(
        &mut self,
        ast: &Node,
        source: &str,
    ) -> Vec<RefactoringSuggestion> {
        self.suggester.analyze(ast, source)
    }

    /// Get current test coverage
    pub fn get_coverage(&self) -> Option<CoverageReport> {
        self.runner.get_coverage()
    }

    /// Update coverage data
    pub fn update_coverage(&mut self, file: PathBuf, coverage: Vec<LineCoverage>) {
        self.coverage_tracker.line_coverage.insert(file, coverage);
        self.coverage_tracker.calculate_total_coverage();
    }

    /// Get inline coverage annotations
    pub fn get_inline_coverage(&self, file: &Path) -> Vec<CoverageAnnotation> {
        let mut annotations = Vec::new();

        if let Some(coverage) = self.coverage_tracker.line_coverage.get(file) {
            for line_cov in coverage {
                if !line_cov.covered {
                    annotations.push(CoverageAnnotation {
                        line: line_cov.line,
                        message: "Not covered by tests".to_string(),
                        severity: AnnotationSeverity::Warning,
                    });
                } else if line_cov.hits == 0 {
                    annotations.push(CoverageAnnotation {
                        line: line_cov.line,
                        message: "Never executed".to_string(),
                        severity: AnnotationSeverity::Info,
                    });
                }
            }
        }

        annotations
    }

    /// Check if coverage meets threshold
    pub fn check_coverage_threshold(&self) -> bool {
        self.coverage_tracker.total_coverage >= self.config.coverage_threshold
    }

    /// Get test file path for a given module
    fn get_test_file_path(&self, name: &str) -> PathBuf {
        let pattern = &self.config.test_file_pattern;
        let path_str = pattern.replace("{name}", name);
        PathBuf::from(path_str)
    }

    /// Get workflow status
    pub fn get_status(&self) -> WorkflowStatus {
        WorkflowStatus {
            state: self.state.clone(),
            coverage: self.coverage_tracker.total_coverage,
            tests_passing: self.test_cache.values().all(|r| r.failed == 0),
            suggestions_available: true, // Would check actual suggestions
        }
    }

    /// Generate diagnostics for uncovered code
    pub fn generate_coverage_diagnostics(&self, file: &Path) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(coverage) = self.coverage_tracker.line_coverage.get(file) {
            for line_cov in coverage {
                if !line_cov.covered {
                    diagnostics.push(Diagnostic {
                        range: (line_cov.line, line_cov.line),
                        severity: DiagnosticSeverity::Warning,
                        code: Some("tdd.uncovered".to_string()),
                        message: "Line not covered by tests".to_string(),
                        related_information: vec![],
                        tags: vec![],
                    });
                }
            }
        }

        diagnostics
    }
}

impl CoverageTracker {
    fn new() -> Self {
        Self { line_coverage: HashMap::new(), branch_coverage: HashMap::new(), total_coverage: 0.0 }
    }

    fn calculate_total_coverage(&mut self) {
        let mut total_lines = 0;
        let mut covered_lines = 0;

        for coverage in self.line_coverage.values() {
            for line in coverage {
                total_lines += 1;
                if line.covered {
                    covered_lines += 1;
                }
            }
        }

        if total_lines > 0 {
            self.total_coverage = (covered_lines as f64 / total_lines as f64) * 100.0;
        }
    }
}

/// Result of a TDD cycle action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TddCycleResult {
    /// Current phase of the TDD cycle (Red, Green, or Refactor)
    pub phase: String,
    /// Human-readable message describing the cycle state
    pub message: String,
    /// Recommended actions to take next
    pub actions: Vec<TddAction>,
}

/// Actions that can be taken during a TDD cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TddAction {
    /// Generate a test with the given name
    GenerateTest(String),
    /// Create a new test file at the specified path
    CreateTestFile(PathBuf),
    /// Execute the test suite
    RunTests,
    /// Request refactoring suggestions for the code
    SuggestRefactorings,
    /// Refresh code coverage data
    UpdateCoverage,
    /// Display test failure details
    ShowFailures,
}

/// Types of tests that can be generated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestType {
    /// Simple happy-path test with typical inputs
    Basic,
    /// Tests for boundary conditions and unusual inputs
    EdgeCase,
    /// Tests for error conditions and exception handling
    ErrorHandling,
    /// Benchmarks and performance regression tests
    Performance,
    /// Tests involving multiple components or external dependencies
    Integration,
}

/// Inline annotation for displaying coverage information in the editor
#[derive(Debug, Clone)]
pub struct CoverageAnnotation {
    /// Line number to annotate
    pub line: usize,
    /// Description of the coverage status
    pub message: String,
    /// Severity level for display styling
    pub severity: AnnotationSeverity,
}

/// Severity levels for coverage and diagnostic annotations
#[derive(Debug, Clone)]
pub enum AnnotationSeverity {
    /// Critical issue that must be addressed
    Error,
    /// Potential problem that should be reviewed
    Warning,
    /// Informational message
    Info,
    /// Subtle suggestion or hint
    Hint,
}

/// Summary of the current TDD workflow status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    /// Current phase of the TDD cycle
    pub state: WorkflowState,
    /// Overall code coverage percentage
    pub coverage: f64,
    /// Whether all tests are currently passing
    pub tests_passing: bool,
    /// Whether refactoring suggestions are available
    pub suggestions_available: bool,
}

/// LSP integration for TDD workflow
#[cfg(feature = "lsp-compat")]
pub mod lsp_integration {
    use super::*;
    use lsp_types::{
        CodeAction, CodeActionKind, Command, Diagnostic as LspDiagnostic, DiagnosticSeverity,
        Position, Range,
    };

    /// Convert TDD actions to LSP code actions
    pub fn tdd_actions_to_code_actions(
        actions: Vec<TddAction>,
        _uri: &url::Url,
    ) -> Vec<CodeAction> {
        actions
            .into_iter()
            .map(|action| match action {
                TddAction::GenerateTest(name) => CodeAction {
                    title: format!("Generate test for '{}'", name),
                    kind: Some(CodeActionKind::REFACTOR),
                    command: Some(Command {
                        title: "Generate Test".to_string(),
                        command: "perl.tdd.generateTest".to_string(),
                        arguments: Some(vec![serde_json::json!(name)]),
                    }),
                    ..Default::default()
                },
                TddAction::RunTests => CodeAction {
                    title: "Run tests".to_string(),
                    kind: Some(CodeActionKind::new("test.run")),
                    command: Some(Command {
                        title: "Run Tests".to_string(),
                        command: "perl.tdd.runTests".to_string(),
                        arguments: None,
                    }),
                    ..Default::default()
                },
                TddAction::SuggestRefactorings => CodeAction {
                    title: "Get refactoring suggestions".to_string(),
                    kind: Some(CodeActionKind::REFACTOR),
                    command: Some(Command {
                        title: "Suggest Refactorings".to_string(),
                        command: "perl.tdd.suggestRefactorings".to_string(),
                        arguments: None,
                    }),
                    ..Default::default()
                },
                _ => CodeAction {
                    title: format!("{:?}", action),
                    kind: Some(CodeActionKind::EMPTY),
                    ..Default::default()
                },
            })
            .collect()
    }

    /// Convert coverage annotations to LSP diagnostics
    pub fn coverage_to_diagnostics(annotations: Vec<CoverageAnnotation>) -> Vec<LspDiagnostic> {
        annotations
            .into_iter()
            .map(|ann| LspDiagnostic {
                range: Range {
                    start: Position { line: ann.line as u32, character: 0 },
                    end: Position { line: ann.line as u32, character: 999 },
                },
                severity: Some(match ann.severity {
                    AnnotationSeverity::Error => DiagnosticSeverity::ERROR,
                    AnnotationSeverity::Warning => DiagnosticSeverity::WARNING,
                    AnnotationSeverity::Info => DiagnosticSeverity::INFORMATION,
                    AnnotationSeverity::Hint => DiagnosticSeverity::HINT,
                }),
                code: Some(lsp_types::NumberOrString::String("coverage".to_string())),
                source: Some("TDD".to_string()),
                message: ann.message,
                ..Default::default()
            })
            .collect()
    }

    /// Create status bar message for TDD state
    pub fn create_status_message(status: &WorkflowStatus) -> String {
        format!(
            "TDD: {:?} | Coverage: {:.1}% | Tests: {} | Refactor: {}",
            status.state,
            status.coverage,
            if status.tests_passing { "✓" } else { "✗" },
            if status.suggestions_available { "💡" } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::NodeKind;
    use crate::ast::SourceLocation;

    #[test]
    fn test_tdd_workflow_cycle() {
        let config = TddConfig::default();
        let mut workflow = TddWorkflow::new(config);

        // Start a new cycle
        let result = workflow.start_cycle("calculate_sum");
        assert_eq!(workflow.state, WorkflowState::Red);
        assert!(result.message.contains("calculate_sum"));
    }

    #[test]
    fn test_generate_tests() {
        let config = TddConfig::default();
        let workflow = TddWorkflow::new(config);

        let ast = Node::new(
            NodeKind::Subroutine {
                name: Some("multiply".to_string()),
                name_span: Some(SourceLocation { start: 4, end: 12 }),
                signature: None,
                body: Box::new(Node::new(
                    NodeKind::Block { statements: vec![] },
                    SourceLocation { start: 0, end: 0 },
                )),
                attributes: vec![],
                prototype: None,
            },
            SourceLocation { start: 0, end: 0 },
        );

        let tests = workflow.generate_tests(&ast, "sub multiply { }");
        assert!(!tests.is_empty());
    }

    #[test]
    fn test_coverage_tracking() {
        let config = TddConfig::default();
        let mut workflow = TddWorkflow::new(config);

        let coverage = vec![
            LineCoverage { line: 1, hits: 5, covered: true },
            LineCoverage { line: 2, hits: 0, covered: false },
            LineCoverage { line: 3, hits: 10, covered: true },
        ];

        workflow.update_coverage(PathBuf::from("test.pl"), coverage);

        let annotations = workflow.get_inline_coverage(&PathBuf::from("test.pl"));
        assert_eq!(annotations.len(), 1); // One uncovered line
        assert_eq!(annotations[0].line, 2);
    }

    #[test]
    fn test_refactoring_suggestions() {
        let config = TddConfig::default();
        let mut workflow = TddWorkflow::new(config);

        // Create a subroutine with 8 parameters to trigger TooManyParameters suggestion
        let parameters: Vec<Node> = (0..8)
            .map(|i| {
                Node::new(
                    NodeKind::MandatoryParameter {
                        variable: Box::new(Node::new(
                            NodeKind::Variable {
                                sigil: "$".to_string(),
                                name: format!("param{}", i),
                            },
                            SourceLocation { start: 0, end: 0 },
                        )),
                    },
                    SourceLocation { start: 0, end: 0 },
                )
            })
            .collect();

        let ast = Node::new(
            NodeKind::Subroutine {
                name: Some("complex_function".to_string()),
                name_span: Some(SourceLocation { start: 4, end: 20 }),
                signature: Some(Box::new(Node::new(
                    NodeKind::Signature { parameters },
                    SourceLocation { start: 0, end: 0 },
                ))),
                body: Box::new(Node::new(
                    NodeKind::Block { statements: vec![] },
                    SourceLocation { start: 0, end: 0 },
                )),
                attributes: vec![],
                prototype: None,
            },
            SourceLocation { start: 0, end: 0 },
        );

        let suggestions = workflow.get_refactoring_suggestions(&ast, "sub complex_function { }");

        // Should suggest refactoring for too many parameters
        assert!(
            suggestions.iter().any(
                |s| s.category == crate::test_generator::RefactoringCategory::TooManyParameters
            )
        );
    }

    #[test]
    fn test_specific_test_generation() {
        let config = TddConfig::default();
        let workflow = TddWorkflow::new(config);

        let test = workflow.generate_test_for_function(
            "validate_email",
            &["$email".to_string()],
            TestType::EdgeCase,
        );

        assert!(test.code.contains("edge cases"));
        assert!(test.code.contains("undef"));
        assert!(test.code.contains("empty"));
    }
}
