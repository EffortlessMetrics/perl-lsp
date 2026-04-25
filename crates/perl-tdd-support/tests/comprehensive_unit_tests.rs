//! Comprehensive unit tests for perl-tdd-support crate.
//!
//! Tests cover: must/must_some/must_err helpers, governance structs,
//! TDD workflow, test generation, test runner, and refactoring analysis.
#![allow(clippy::field_reassign_with_default)]

use perl_tdd_support::governance::*;
use perl_tdd_support::tdd_basic::{
    Diagnostic, DiagnosticSeverity, RefactoringAnalyzer, RefactoringCategory, TddState, TddWorkflow,
};
use perl_tdd_support::test_generator::{
    Priority, RefactoringCategory as GenRefactoringCategory, RefactoringSuggester, TestFramework,
    TestGenerator, TestGeneratorOptions, TestResults, TestRunner as GenTestRunner,
};
use perl_tdd_support::test_runner::{TestItem, TestKind, TestRange, TestResult, TestStatus};
use perl_tdd_support::{Node, NodeKind, SourceLocation};
use perl_tdd_support::{must, must_err, must_some};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

// ---------------------------------------------------------------------------
// must / must_some / must_err helpers
// ---------------------------------------------------------------------------

#[test]
fn must_extracts_ok_value() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<i32, &str> = Ok(42);
    let extracted = must(val);
    assert_eq!(extracted, 42);
    Ok(())
}

#[test]
fn must_some_extracts_some_value() -> Result<(), Box<dyn std::error::Error>> {
    let val: Option<&str> = Some("hello");
    let extracted = must_some(val);
    assert_eq!(extracted, "hello");
    Ok(())
}

#[test]
fn must_err_extracts_err_value() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<i32, String> = Err("bad".to_string());
    let extracted = must_err(val);
    assert_eq!(extracted, "bad");
    Ok(())
}

#[test]
#[should_panic(expected = "unexpected Err")]
fn must_panics_on_err() {
    let val: Result<i32, &str> = Err("oops");
    let _ = must(val);
}

#[test]
#[should_panic(expected = "unexpected None")]
fn must_some_panics_on_none() {
    let val: Option<i32> = None;
    let _ = must_some(val);
}

#[test]
#[should_panic(expected = "expected Err")]
fn must_err_panics_on_ok() {
    let val: Result<i32, &str> = Ok(1);
    let _ = must_err(val);
}

#[test]
fn must_works_with_string_error() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<String, String> = Ok("success".to_string());
    let extracted = must(val);
    assert_eq!(extracted, "success");
    Ok(())
}

#[test]
fn must_some_works_with_complex_type() -> Result<(), Box<dyn std::error::Error>> {
    let val: Option<Vec<i32>> = Some(vec![1, 2, 3]);
    let extracted = must_some(val);
    assert_eq!(extracted.len(), 3);
    Ok(())
}

#[test]
fn must_err_returns_error_variant() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<(), Vec<String>> = Err(vec!["a".to_string(), "b".to_string()]);
    let errs = must_err(val);
    assert_eq!(errs.len(), 2);
    Ok(())
}

// ---------------------------------------------------------------------------
// TestStatus
// ---------------------------------------------------------------------------

#[test]
fn test_status_as_str_passed() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(TestStatus::Passed.as_str(), "passed");
    Ok(())
}

#[test]
fn test_status_as_str_failed() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(TestStatus::Failed.as_str(), "failed");
    Ok(())
}

#[test]
fn test_status_as_str_skipped() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(TestStatus::Skipped.as_str(), "skipped");
    Ok(())
}

#[test]
fn test_status_as_str_errored() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(TestStatus::Errored.as_str(), "errored");
    Ok(())
}

// ---------------------------------------------------------------------------
// TestItem JSON serialization
// ---------------------------------------------------------------------------

#[test]
fn test_item_to_json_basic() -> Result<(), Box<dyn std::error::Error>> {
    let item = TestItem {
        id: "file.t::test_one".to_string(),
        label: "test_one".to_string(),
        uri: "file.t".to_string(),
        range: TestRange { start_line: 0, start_character: 0, end_line: 5, end_character: 1 },
        kind: TestKind::Test,
        children: vec![],
    };
    let json = item.to_json();
    assert_eq!(json["id"], "file.t::test_one");
    assert_eq!(json["label"], "test_one");
    assert_eq!(json["canResolveChildren"], false);
    Ok(())
}

#[test]
fn test_item_to_json_with_children() -> Result<(), Box<dyn std::error::Error>> {
    let child = TestItem {
        id: "child".to_string(),
        label: "child_test".to_string(),
        uri: "file.t".to_string(),
        range: TestRange { start_line: 1, start_character: 0, end_line: 3, end_character: 0 },
        kind: TestKind::Test,
        children: vec![],
    };
    let parent = TestItem {
        id: "parent".to_string(),
        label: "parent_suite".to_string(),
        uri: "file.t".to_string(),
        range: TestRange { start_line: 0, start_character: 0, end_line: 10, end_character: 0 },
        kind: TestKind::Suite,
        children: vec![child],
    };
    let json = parent.to_json();
    assert_eq!(json["canResolveChildren"], true);
    let children = json["children"].as_array();
    assert!(children.is_some());
    assert_eq!(must_some(children).len(), 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// TestResult JSON serialization
// ---------------------------------------------------------------------------

#[test]
fn test_result_to_json_passed() -> Result<(), Box<dyn std::error::Error>> {
    let result = TestResult {
        test_id: "test_1".to_string(),
        status: TestStatus::Passed,
        message: None,
        duration: Some(42),
    };
    let json = result.to_json();
    assert_eq!(json["testId"], "test_1");
    assert_eq!(json["state"], "passed");
    assert_eq!(json["duration"], 42);
    assert!(json.get("message").is_none());
    Ok(())
}

#[test]
fn test_result_to_json_failed_with_message() -> Result<(), Box<dyn std::error::Error>> {
    let result = TestResult {
        test_id: "test_2".to_string(),
        status: TestStatus::Failed,
        message: Some("assertion failed".to_string()),
        duration: None,
    };
    let json = result.to_json();
    assert_eq!(json["state"], "failed");
    assert!(json.get("message").is_some());
    Ok(())
}

#[test]
fn test_result_to_json_no_duration() -> Result<(), Box<dyn std::error::Error>> {
    let result = TestResult {
        test_id: "test_3".to_string(),
        status: TestStatus::Skipped,
        message: None,
        duration: None,
    };
    let json = result.to_json();
    assert_eq!(json["state"], "skipped");
    assert!(json.get("duration").is_none());
    Ok(())
}

// ---------------------------------------------------------------------------
// test_runner::TestRunner — discover_tests
// ---------------------------------------------------------------------------

fn make_test_file_ast() -> Node {
    Node::new(
        NodeKind::Program {
            statements: vec![
                Node::new(
                    NodeKind::Subroutine {
                        name: Some("test_basic".to_string()),
                        name_span: None,
                        prototype: None,
                        signature: None,
                        attributes: vec![],
                        body: Box::new(Node::new(
                            NodeKind::Block { statements: vec![] },
                            SourceLocation { start: 10, end: 20 },
                        )),
                    },
                    SourceLocation { start: 0, end: 20 },
                ),
                Node::new(
                    NodeKind::Subroutine {
                        name: Some("helper".to_string()),
                        name_span: None,
                        prototype: None,
                        signature: None,
                        attributes: vec![],
                        body: Box::new(Node::new(
                            NodeKind::Block { statements: vec![] },
                            SourceLocation { start: 30, end: 40 },
                        )),
                    },
                    SourceLocation { start: 25, end: 40 },
                ),
                Node::new(
                    NodeKind::Subroutine {
                        name: Some("test_another".to_string()),
                        name_span: None,
                        prototype: None,
                        signature: None,
                        attributes: vec![],
                        body: Box::new(Node::new(
                            NodeKind::Block { statements: vec![] },
                            SourceLocation { start: 50, end: 60 },
                        )),
                    },
                    SourceLocation { start: 45, end: 60 },
                ),
            ],
        },
        SourceLocation { start: 0, end: 60 },
    )
}

#[test]
fn test_runner_discovers_test_functions_in_t_file() -> Result<(), Box<dyn std::error::Error>> {
    let source = " ".repeat(60);
    let runner = perl_tdd_support::test_runner::TestRunner::new(source, "my_test.t".to_string());
    let ast = make_test_file_ast();
    let items = runner.discover_tests(&ast);

    // .t file → wrapped in a File-level item
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, TestKind::File);
    // Only test_ prefixed subs are discovered
    assert_eq!(items[0].children.len(), 2);
    Ok(())
}

#[test]
fn test_runner_discovers_test_functions_in_non_test_file() -> Result<(), Box<dyn std::error::Error>>
{
    let source = " ".repeat(60);
    let runner = perl_tdd_support::test_runner::TestRunner::new(source, "lib/Foo.pm".to_string());
    let ast = make_test_file_ast();
    let items = runner.discover_tests(&ast);
    // Non-test file → individual test functions only
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|i| i.kind == TestKind::Test));
    Ok(())
}

// ---------------------------------------------------------------------------
// tdd_basic::TestGenerator (simple)
// ---------------------------------------------------------------------------

#[test]
fn basic_test_generator_test_more() -> Result<(), Box<dyn std::error::Error>> {
    let generator = perl_tdd_support::tdd_basic::TestGenerator::new("Test::More");
    let code = generator.generate_test("my_sub", 2);
    assert!(code.contains("Test::More"));
    assert!(code.contains("my_sub"));
    assert!(code.contains("arg1"));
    assert!(code.contains("arg2"));
    assert!(code.contains("done_testing"));
    Ok(())
}

#[test]
fn basic_test_generator_test2() -> Result<(), Box<dyn std::error::Error>> {
    let generator = perl_tdd_support::tdd_basic::TestGenerator::new("Test2::V0");
    let code = generator.generate_test("process", 0);
    assert!(code.contains("Test2::V0"));
    assert!(code.contains("process"));
    Ok(())
}

#[test]
fn basic_test_generator_default_framework() -> Result<(), Box<dyn std::error::Error>> {
    let generator = perl_tdd_support::tdd_basic::TestGenerator::new("Unknown");
    let code = generator.generate_test("foo", 1);
    // Falls back to Test::More
    assert!(code.contains("Test::More"));
    Ok(())
}

#[test]
fn basic_test_generator_zero_params() -> Result<(), Box<dyn std::error::Error>> {
    let generator = perl_tdd_support::tdd_basic::TestGenerator::new("Test::More");
    let code = generator.generate_test("run", 0);
    assert!(code.contains("run()"));
    Ok(())
}

// ---------------------------------------------------------------------------
// tdd_basic::TddWorkflow (simple state machine)
// ---------------------------------------------------------------------------

#[test]
fn tdd_workflow_basic_cycle() -> Result<(), Box<dyn std::error::Error>> {
    let mut wf = TddWorkflow::new("Test::More");

    let r = wf.start_cycle("my_test");
    assert_eq!(r.state, TddState::Red);
    assert!(r.message.contains("my_test"));

    let r = wf.run_tests(false);
    assert_eq!(r.state, TddState::Red);

    let r = wf.run_tests(true);
    assert_eq!(r.state, TddState::Green);

    let r = wf.start_refactor();
    assert_eq!(r.state, TddState::Refactor);

    let r = wf.complete_cycle();
    assert_eq!(r.state, TddState::Idle);
    Ok(())
}

#[test]
fn tdd_workflow_generate_test_delegates() -> Result<(), Box<dyn std::error::Error>> {
    let wf = TddWorkflow::new("Test::More");
    let code = wf.generate_test("add", 2);
    assert!(code.contains("add"));
    assert!(code.contains("arg1"));
    Ok(())
}

#[test]
fn tdd_workflow_coverage_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let wf = TddWorkflow::new("Test::More");
    let diags = wf.get_coverage_diagnostics(&[3, 7, 12]);
    assert_eq!(diags.len(), 3);
    for d in &diags {
        assert_eq!(d.code.as_deref(), Some("tdd.uncovered"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// tdd_basic::RefactoringAnalyzer
// ---------------------------------------------------------------------------

#[test]
fn refactoring_analyzer_default() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = RefactoringAnalyzer::default();
    let source = "sub foo { 1 }";
    let ast = Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::Subroutine {
                    name: Some("foo".to_string()),
                    name_span: None,
                    prototype: None,
                    signature: None,
                    attributes: vec![],
                    body: Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        SourceLocation { start: 10, end: 13 },
                    )),
                },
                SourceLocation { start: 0, end: 13 },
            )],
        },
        SourceLocation { start: 0, end: 13 },
    );
    let suggestions = analyzer.analyze(&ast, source);
    // Simple sub should yield no refactoring suggestions
    assert!(suggestions.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// tdd_basic::Diagnostic
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_severity_variants() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticSeverity::Error, DiagnosticSeverity::Error);
    assert_eq!(DiagnosticSeverity::Warning, DiagnosticSeverity::Warning);
    assert_eq!(DiagnosticSeverity::Information, DiagnosticSeverity::Information);
    assert_eq!(DiagnosticSeverity::Hint, DiagnosticSeverity::Hint);
    Ok(())
}

#[test]
fn diagnostic_construction() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (1, 5),
        severity: DiagnosticSeverity::Error,
        code: Some("E001".to_string()),
        message: "something wrong".to_string(),
        related_information: vec!["see also".to_string()],
        tags: vec!["deprecated".to_string()],
    };
    assert_eq!(d.range, (1, 5));
    assert_eq!(d.message, "something wrong");
    assert_eq!(d.related_information.len(), 1);
    assert_eq!(d.tags.len(), 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// test_generator::TestGenerator (full)
// ---------------------------------------------------------------------------

fn make_sub_ast(name: &str) -> Node {
    Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::Subroutine {
                    name: Some(name.to_string()),
                    name_span: None,
                    prototype: None,
                    signature: None,
                    attributes: vec![],
                    body: Box::new(Node::new(
                        NodeKind::Block { statements: vec![] },
                        SourceLocation { start: 10, end: 20 },
                    )),
                },
                SourceLocation { start: 0, end: 20 },
            )],
        },
        SourceLocation { start: 0, end: 20 },
    )
}

#[test]
fn test_generator_test_more() -> Result<(), Box<dyn std::error::Error>> {
    let generator = TestGenerator::new(TestFramework::TestMore);
    let ast = make_sub_ast("compute");
    let tests = generator.generate_tests(&ast, "sub compute { 1 }  ");
    assert!(!tests.is_empty());
    let first = &tests[0];
    assert!(first.code.contains("Test::More"));
    assert!(first.name.contains("compute"));
    Ok(())
}

#[test]
fn test_generator_test2() -> Result<(), Box<dyn std::error::Error>> {
    let generator = TestGenerator::new(TestFramework::Test2V0);
    let ast = make_sub_ast("process");
    let tests = generator.generate_tests(&ast, "sub process { 1 }  ");
    assert!(!tests.is_empty());
    assert!(tests[0].code.contains("Test2::V0"));
    Ok(())
}

#[test]
fn test_generator_test_simple() -> Result<(), Box<dyn std::error::Error>> {
    let generator = TestGenerator::new(TestFramework::TestSimple);
    let ast = make_sub_ast("run");
    let tests = generator.generate_tests(&ast, "sub run { 1 }          ");
    assert!(!tests.is_empty());
    assert!(tests[0].code.contains("Test::Simple"));
    Ok(())
}

#[test]
fn test_generator_test_class() -> Result<(), Box<dyn std::error::Error>> {
    let generator = TestGenerator::new(TestFramework::TestClass);
    let ast = make_sub_ast("init");
    let tests = generator.generate_tests(&ast, "sub init { 1 }         ");
    assert!(!tests.is_empty());
    assert!(tests[0].code.contains("Test::Class"));
    Ok(())
}

#[test]
fn test_generator_skips_private_by_default() -> Result<(), Box<dyn std::error::Error>> {
    let generator = TestGenerator::new(TestFramework::TestMore);
    let ast = make_sub_ast("_private");
    let tests = generator.generate_tests(&ast, "sub _private { 1 }     ");
    // Private subs skipped by default
    assert!(tests.is_empty());
    Ok(())
}

#[test]
fn test_generator_includes_private_when_enabled() -> Result<(), Box<dyn std::error::Error>> {
    let mut opts = TestGeneratorOptions::default();
    opts.test_private = true;
    let generator = TestGenerator::with_options(TestFramework::TestMore, opts);
    let ast = make_sub_ast("_internal");
    let tests = generator.generate_tests(&ast, "sub _internal { 1 }    ");
    assert!(!tests.is_empty());
    Ok(())
}

#[test]
fn test_generator_options_default() -> Result<(), Box<dyn std::error::Error>> {
    let opts = TestGeneratorOptions::default();
    assert!(!opts.test_private);
    assert!(opts.edge_cases);
    assert!(opts.use_mocks);
    assert!(opts.data_driven);
    assert!(!opts.perf_tests);
    Ok(())
}

#[test]
fn test_generator_with_perf_tests() -> Result<(), Box<dyn std::error::Error>> {
    let mut opts = TestGeneratorOptions::default();
    opts.perf_tests = true;
    let generator = TestGenerator::with_options(TestFramework::TestMore, opts);
    let ast = make_sub_ast("hot_path");
    let tests = generator.generate_tests(&ast, "sub hot_path { 1 }     ");
    let perf_tests: Vec<_> = tests.iter().filter(|t| t.name.contains("performance")).collect();
    assert!(!perf_tests.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// test_generator::TestRunner (prove runner)
// ---------------------------------------------------------------------------

#[test]
fn gen_test_runner_new() -> Result<(), Box<dyn std::error::Error>> {
    let runner = GenTestRunner::new();
    let results = runner.run_tests(&[]);
    assert_eq!(results.total, 0);
    assert_eq!(results.passed, 0);
    Ok(())
}

#[test]
fn gen_test_runner_with_command() -> Result<(), Box<dyn std::error::Error>> {
    let runner = GenTestRunner::with_command("prove -v -l".to_string());
    let results = runner.run_tests(&["t/basic.t".to_string()]);
    assert_eq!(results.total, 1);
    Ok(())
}

#[test]
fn gen_test_runner_watch_disabled() -> Result<(), Box<dyn std::error::Error>> {
    let runner = GenTestRunner::new();
    let result = runner.watch(&[]);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn gen_test_runner_coverage_disabled() -> Result<(), Box<dyn std::error::Error>> {
    let runner = GenTestRunner::new();
    assert!(runner.get_coverage().is_none());
    Ok(())
}

// ---------------------------------------------------------------------------
// test_generator::TestResults
// ---------------------------------------------------------------------------

#[test]
fn test_results_default() -> Result<(), Box<dyn std::error::Error>> {
    let r = TestResults::default();
    assert_eq!(r.total, 0);
    assert_eq!(r.passed, 0);
    assert_eq!(r.failed, 0);
    assert_eq!(r.skipped, 0);
    assert_eq!(r.todo, 0);
    assert!(r.errors.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// test_generator::RefactoringSuggester
// ---------------------------------------------------------------------------

#[test]
fn refactoring_suggester_empty_program() -> Result<(), Box<dyn std::error::Error>> {
    let mut suggester = RefactoringSuggester::new();
    let ast =
        Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 });
    let suggestions = suggester.analyze(&ast, "");
    assert!(suggestions.is_empty());
    Ok(())
}

#[test]
fn refactoring_suggester_default() -> Result<(), Box<dyn std::error::Error>> {
    let suggester = RefactoringSuggester::default();
    // Just verify it constructs without error
    drop(suggester);
    Ok(())
}

// ---------------------------------------------------------------------------
// test_generator::Priority ordering
// ---------------------------------------------------------------------------

#[test]
fn priority_ordering() -> Result<(), Box<dyn std::error::Error>> {
    assert!(Priority::Low < Priority::Medium);
    assert!(Priority::Medium < Priority::High);
    assert!(Priority::High < Priority::Critical);
    Ok(())
}

// ---------------------------------------------------------------------------
// test_generator::TestFramework equality
// ---------------------------------------------------------------------------

#[test]
fn test_framework_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(TestFramework::TestMore, TestFramework::TestMore);
    assert_eq!(TestFramework::Test2V0, TestFramework::Test2V0);
    assert_ne!(TestFramework::TestMore, TestFramework::TestSimple);
    Ok(())
}

// ---------------------------------------------------------------------------
// test_generator::RefactoringCategory equality
// ---------------------------------------------------------------------------

#[test]
fn gen_refactoring_category_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(GenRefactoringCategory::DuplicateCode, GenRefactoringCategory::DuplicateCode);
    assert_ne!(GenRefactoringCategory::LongMethod, GenRefactoringCategory::DeadCode);
    Ok(())
}

// ---------------------------------------------------------------------------
// tdd_basic::RefactoringCategory
// ---------------------------------------------------------------------------

#[test]
fn basic_refactoring_category_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(RefactoringCategory::HighComplexity, RefactoringCategory::HighComplexity);
    assert_ne!(RefactoringCategory::LongMethod, RefactoringCategory::TooManyParameters);
    Ok(())
}

// ---------------------------------------------------------------------------
// governance types — construction & serialization roundtrip
// ---------------------------------------------------------------------------

fn make_governance() -> IgnoredTestGovernance {
    IgnoredTestGovernance {
        inventory: IgnoredTestInventory {
            total_count: 10,
            by_category: HashMap::new(),
            by_crate: HashMap::new(),
            by_priority: HashMap::new(),
            last_updated: SystemTime::now(),
        },
        baseline_management: BaselineManagement {
            baseline_count: 10,
            max_deviation: 3,
            deviation_threshold_percent: 20.0,
            baseline_date: SystemTime::now(),
            next_review_date: SystemTime::now(),
        },
        quality_gates: QualityGates {
            pre_commit: PreCommitValidation {
                require_justification: true,
                max_new_ignored_per_commit: 2,
                documentation_requirements: DocumentationRequirements {
                    require_issue_reference: true,
                    require_timeline: true,
                    require_success_criteria: true,
                    require_complexity_assessment: true,
                },
            },
            ci_validation: CiValidation {
                block_on_count_increase: true,
                max_ignored_per_crate: HashMap::new(),
                min_quality_score: 70.0,
            },
            metrics_tracking: MetricsTracking {
                track_trend: true,
                trend_window_days: 30,
                alert_on_negative_trend: true,
            },
        },
        reporting: ReportingConfiguration {
            daily_reports: true,
            weekly_trends: true,
            monthly_summaries: true,
            output_formats: vec![ReportFormat::Json, ReportFormat::Markdown],
        },
    }
}

fn make_test_metadata() -> IgnoredTestMetadata {
    IgnoredTestMetadata {
        test_id: "test_001".to_string(),
        file_path: PathBuf::from("crates/foo/tests/bar.rs"),
        test_name: "test_something".to_string(),
        category: TestCategory::Infrastructure,
        priority: 2,
        ignore_reason: "Requires feature flag, see issue #123".to_string(),
        complexity: ComplexityLevel::Medium,
        target_timeline: Duration::from_secs(7 * 24 * 3600),
        dependencies: vec!["dep_1".to_string()],
        success_criteria: vec!["passes locally".to_string(), "passes CI".to_string()],
        workflow_integration: LspWorkflowStage::Parse,
        performance_requirements: None,
        last_assessed: SystemTime::now(),
    }
}

#[test]
fn governance_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let json = serde_json::to_string(&gov)?;
    let _parsed: IgnoredTestGovernance = serde_json::from_str(&json)?;
    Ok(())
}

#[test]
fn test_metadata_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let meta = make_test_metadata();
    let json = serde_json::to_string(&meta)?;
    let parsed: IgnoredTestMetadata = serde_json::from_str(&json)?;
    assert_eq!(parsed.test_id, "test_001");
    assert_eq!(parsed.category, TestCategory::Infrastructure);
    Ok(())
}

#[test]
fn report_format_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(ReportFormat::Json, ReportFormat::Json);
    assert_ne!(ReportFormat::Json, ReportFormat::Csv);
    Ok(())
}

#[test]
fn test_category_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(TestCategory::CriticalLsp, TestCategory::CriticalLsp);
    assert_ne!(TestCategory::EdgeCases, TestCategory::Infrastructure);
    Ok(())
}

#[test]
fn complexity_level_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(ComplexityLevel::Low, ComplexityLevel::Low);
    assert_ne!(ComplexityLevel::High, ComplexityLevel::Critical);
    Ok(())
}

#[test]
fn lsp_workflow_stage_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(LspWorkflowStage::Parse, LspWorkflowStage::Parse);
    assert_ne!(LspWorkflowStage::Index, LspWorkflowStage::Complete);
    Ok(())
}

// ---------------------------------------------------------------------------
// IgnoredTestGuardian — validation
// ---------------------------------------------------------------------------

#[test]
fn guardian_validates_well_formed_test() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);
    let meta = make_test_metadata();
    let result = guardian.validate_new_ignored_test(&meta);
    assert!(result.is_valid);
    assert!(result.errors.is_empty());
    Ok(())
}

#[test]
fn guardian_rejects_missing_issue_reference() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);
    let mut meta = make_test_metadata();
    meta.ignore_reason = "no reference here".to_string();
    let result = guardian.validate_new_ignored_test(&meta);
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("issue")));
    Ok(())
}

#[test]
fn guardian_rejects_missing_timeline() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);
    let mut meta = make_test_metadata();
    meta.target_timeline = Duration::from_secs(0);
    let result = guardian.validate_new_ignored_test(&meta);
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("timeline")));
    Ok(())
}

#[test]
fn guardian_rejects_missing_success_criteria() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);
    let mut meta = make_test_metadata();
    meta.success_criteria.clear();
    let result = guardian.validate_new_ignored_test(&meta);
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("success criteria")));
    Ok(())
}

#[test]
fn guardian_warns_on_low_complexity_long_timeline() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);
    let mut meta = make_test_metadata();
    meta.complexity = ComplexityLevel::Low;
    meta.target_timeline = Duration::from_secs(30 * 24 * 3600); // 30 days
    let result = guardian.validate_new_ignored_test(&meta);
    assert!(result.warnings.iter().any(|w| w.contains("shorter timeline")));
    Ok(())
}

#[test]
fn guardian_quality_score_decreases_for_short_reason() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);

    let mut meta_good = make_test_metadata();
    meta_good.ignore_reason =
        "This is a detailed reason referencing issue #456 with full context".to_string();

    let mut meta_bad = make_test_metadata();
    meta_bad.ignore_reason = "short #1".to_string();

    let score_good = guardian.validate_new_ignored_test(&meta_good).quality_score;
    let score_bad = guardian.validate_new_ignored_test(&meta_bad).quality_score;
    assert!(score_good > score_bad);
    Ok(())
}

#[test]
fn guardian_quality_score_bonus_for_many_criteria() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);

    let mut meta = make_test_metadata();
    meta.success_criteria =
        vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
    let result = guardian.validate_new_ignored_test(&meta);
    // 3+ criteria earns bonus
    assert!(result.quality_score > 0.0);
    Ok(())
}

// ---------------------------------------------------------------------------
// IgnoredTestGuardian — baseline regression
// ---------------------------------------------------------------------------

#[test]
fn guardian_no_regression_within_deviation() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);
    let result = guardian.check_baseline_regression(12); // baseline=10, max_dev=3
    assert!(!result.is_regression);
    assert_eq!(result.absolute_increase, 2);
    assert!(result.threshold_exceeded.is_none());
    Ok(())
}

#[test]
fn guardian_detects_absolute_regression() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);
    let result = guardian.check_baseline_regression(14); // 4 > max_dev=3
    assert!(result.is_regression);
    assert!(result.threshold_exceeded.is_some());
    Ok(())
}

#[test]
fn guardian_no_regression_below_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);
    let result = guardian.check_baseline_regression(5); // below baseline
    assert!(!result.is_regression);
    assert_eq!(result.absolute_increase, 0);
    Ok(())
}

#[test]
fn guardian_regression_result_fields() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);
    let result = guardian.check_baseline_regression(10);
    assert_eq!(result.current_count, 10);
    assert_eq!(result.baseline_count, 10);
    assert_eq!(result.absolute_increase, 0);
    assert!((result.percentage_increase - 0.0).abs() < f64::EPSILON);
    Ok(())
}

// ---------------------------------------------------------------------------
// IgnoredTestGuardian — trend report
// ---------------------------------------------------------------------------

#[test]
fn guardian_trend_report_no_data() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let guardian = IgnoredTestGuardian::new(gov);
    let report = guardian.generate_trend_report();
    assert_eq!(report.trend_direction, TrendDirection::Unknown);
    assert!(report.data_points.is_empty());
    assert!(!report.recommendations.is_empty());
    Ok(())
}

#[test]
fn guardian_trend_report_with_data() -> Result<(), Box<dyn std::error::Error>> {
    let gov = make_governance();
    let mut guardian = IgnoredTestGuardian::new(gov);
    let now = SystemTime::now();

    let data = vec![
        (now - Duration::from_secs(3600), 10),
        (now - Duration::from_secs(1800), 12),
        (now, 15),
    ];
    guardian.set_historical_data(data);

    let report = guardian.generate_trend_report();
    // With monthly_summaries=true, window = 30 days, all data should be included
    assert!(!report.recommendations.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// TrendDirection
// ---------------------------------------------------------------------------

#[test]
fn trend_direction_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(TrendDirection::Increasing, TrendDirection::Increasing);
    assert_eq!(TrendDirection::Decreasing, TrendDirection::Decreasing);
    assert_eq!(TrendDirection::Stable, TrendDirection::Stable);
    assert_eq!(TrendDirection::Unknown, TrendDirection::Unknown);
    assert_ne!(TrendDirection::Increasing, TrendDirection::Decreasing);
    Ok(())
}

// ---------------------------------------------------------------------------
// PerformanceRequirements
// ---------------------------------------------------------------------------

#[test]
fn performance_requirements_serde() -> Result<(), Box<dyn std::error::Error>> {
    let pr = PerformanceRequirements {
        max_latency_ms: 100,
        max_memory_mb: 512,
        min_throughput: Some(1000.0),
    };
    let json = serde_json::to_string(&pr)?;
    let parsed: PerformanceRequirements = serde_json::from_str(&json)?;
    assert_eq!(parsed.max_latency_ms, 100);
    assert_eq!(parsed.max_memory_mb, 512);
    assert!((must_some(parsed.min_throughput) - 1000.0).abs() < f64::EPSILON);
    Ok(())
}

// ---------------------------------------------------------------------------
// tdd_workflow types
// ---------------------------------------------------------------------------

#[test]
fn workflow_state_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use perl_tdd_support::tdd_workflow::WorkflowState;
    let states = vec![
        WorkflowState::Red,
        WorkflowState::Green,
        WorkflowState::Refactor,
        WorkflowState::Idle,
    ];
    for state in &states {
        let json = serde_json::to_string(state)?;
        let parsed: WorkflowState = serde_json::from_str(&json)?;
        assert_eq!(&parsed, state);
    }
    Ok(())
}

#[test]
fn tdd_config_default() -> Result<(), Box<dyn std::error::Error>> {
    use perl_tdd_support::tdd_workflow::TddConfig;
    let cfg = TddConfig::default();
    assert!(cfg.auto_generate_tests);
    assert!(cfg.test_on_save);
    assert!(cfg.show_inline_coverage);
    assert_eq!(cfg.test_framework, "Test::More");
    assert!((cfg.coverage_threshold - 80.0).abs() < f64::EPSILON);
    assert!(cfg.continuous_testing);
    assert!(cfg.auto_suggest_refactorings);
    Ok(())
}

#[test]
fn tdd_config_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use perl_tdd_support::tdd_workflow::TddConfig;
    let cfg = TddConfig::default();
    let json = serde_json::to_string(&cfg)?;
    let parsed: TddConfig = serde_json::from_str(&json)?;
    assert_eq!(parsed.test_framework, cfg.test_framework);
    Ok(())
}

// ---------------------------------------------------------------------------
// TestKind
// ---------------------------------------------------------------------------

#[test]
fn test_kind_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(TestKind::File, TestKind::File);
    assert_eq!(TestKind::Suite, TestKind::Suite);
    assert_eq!(TestKind::Test, TestKind::Test);
    assert_ne!(TestKind::File, TestKind::Test);
    Ok(())
}

// ---------------------------------------------------------------------------
// TestRange construction
// ---------------------------------------------------------------------------

#[test]
fn test_range_construction() -> Result<(), Box<dyn std::error::Error>> {
    let range = TestRange { start_line: 0, start_character: 5, end_line: 10, end_character: 20 };
    assert_eq!(range.start_line, 0);
    assert_eq!(range.end_line, 10);
    Ok(())
}

// ---------------------------------------------------------------------------
// Re-exports from perl_parser_core
// ---------------------------------------------------------------------------

#[test]
fn node_construction() -> Result<(), Box<dyn std::error::Error>> {
    let node =
        Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 });
    assert!(matches!(node.kind, NodeKind::Program { .. }));
    assert_eq!(node.location.start, 0);
    Ok(())
}

#[test]
fn source_location_fields() -> Result<(), Box<dyn std::error::Error>> {
    let loc = SourceLocation { start: 10, end: 50 };
    assert_eq!(loc.start, 10);
    assert_eq!(loc.end, 50);
    Ok(())
}

// ---------------------------------------------------------------------------
// tdd_workflow::TddWorkflow (full)
// ---------------------------------------------------------------------------

#[test]
fn full_tdd_workflow_start_cycle() -> Result<(), Box<dyn std::error::Error>> {
    use perl_tdd_support::tdd_workflow::{TddConfig, TddWorkflow};
    let mut wf = TddWorkflow::new(TddConfig::default());
    let result = wf.start_cycle("my_feature");
    assert_eq!(result.phase, "Red");
    assert!(result.message.contains("my_feature"));
    assert!(!result.actions.is_empty());
    Ok(())
}

#[test]
fn full_tdd_workflow_get_status() -> Result<(), Box<dyn std::error::Error>> {
    use perl_tdd_support::tdd_workflow::{TddConfig, TddWorkflow, WorkflowState};
    let wf = TddWorkflow::new(TddConfig::default());
    let status = wf.get_status();
    assert_eq!(status.state, WorkflowState::Idle);
    Ok(())
}

#[test]
fn full_tdd_workflow_coverage_threshold_initially_not_met() -> Result<(), Box<dyn std::error::Error>>
{
    use perl_tdd_support::tdd_workflow::{TddConfig, TddWorkflow};
    let wf = TddWorkflow::new(TddConfig::default());
    // No coverage data → 0% < threshold
    assert!(!wf.check_coverage_threshold());
    Ok(())
}

#[test]
fn full_tdd_workflow_generate_test_for_function() -> Result<(), Box<dyn std::error::Error>> {
    use perl_tdd_support::tdd_workflow::{TddConfig, TddWorkflow, TestType};
    let wf = TddWorkflow::new(TddConfig::default());
    let tc = wf.generate_test_for_function(
        "add",
        &["$a".to_string(), "$b".to_string()],
        TestType::Basic,
    );
    assert!(tc.code.contains("add"));
    assert!(!tc.is_todo);
    Ok(())
}

#[test]
fn full_tdd_workflow_generate_edge_case_test() -> Result<(), Box<dyn std::error::Error>> {
    use perl_tdd_support::tdd_workflow::{TddConfig, TddWorkflow, TestType};
    let wf = TddWorkflow::new(TddConfig::default());
    let tc = wf.generate_test_for_function("process", &[], TestType::EdgeCase);
    assert!(tc.code.contains("edge cases"));
    Ok(())
}

#[test]
fn full_tdd_workflow_generate_error_test() -> Result<(), Box<dyn std::error::Error>> {
    use perl_tdd_support::tdd_workflow::{TddConfig, TddWorkflow, TestType};
    let wf = TddWorkflow::new(TddConfig::default());
    let tc = wf.generate_test_for_function("validate", &[], TestType::ErrorHandling);
    assert!(tc.code.contains("error handling"));
    Ok(())
}

#[test]
fn full_tdd_workflow_integration_test_is_todo() -> Result<(), Box<dyn std::error::Error>> {
    use perl_tdd_support::tdd_workflow::{TddConfig, TddWorkflow, TestType};
    let wf = TddWorkflow::new(TddConfig::default());
    let tc = wf.generate_test_for_function("setup", &[], TestType::Integration);
    assert!(tc.is_todo);
    Ok(())
}

#[test]
fn full_tdd_workflow_performance_test_is_todo() -> Result<(), Box<dyn std::error::Error>> {
    use perl_tdd_support::tdd_workflow::{TddConfig, TddWorkflow, TestType};
    let wf = TddWorkflow::new(TddConfig::default());
    let tc = wf.generate_test_for_function("render", &[], TestType::Performance);
    assert!(tc.is_todo);
    Ok(())
}
