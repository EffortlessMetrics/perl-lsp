//! Additional test coverage for perl-tdd-support helper utilities.
//!
//! Focuses on: must/must_some/must_err edge cases, panic message formatting,
//! #[track_caller] behavior, governance quality scoring edge cases,
//! TDD workflow state transitions, and coverage tracker boundaries.

use perl_tdd_support::governance::*;
use perl_tdd_support::tdd_basic::{
    DiagnosticSeverity, RefactoringAnalyzer, RefactoringCategory, TddState, TddWorkflow,
};
use perl_tdd_support::tdd_workflow::{
    LineCoverage, TddConfig, TddWorkflow as FullTddWorkflow, WorkflowState,
};
use perl_tdd_support::test_runner::TestStatus;
use perl_tdd_support::{Node, NodeKind, SourceLocation};
use perl_tdd_support::{must, must_err, must_some};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

// ===========================================================================
// must() — edge cases and panic message formatting
// ===========================================================================

#[test]
fn test_must_ok_with_unit_type() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<(), &str> = Ok(());
    must(val);
    Ok(())
}

#[test]
fn test_must_ok_with_zero() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<i64, String> = Ok(0);
    assert_eq!(must(val), 0);
    Ok(())
}

#[test]
fn test_must_ok_with_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<String, String> = Ok(String::new());
    assert_eq!(must(val), "");
    Ok(())
}

#[test]
fn test_must_ok_with_empty_vec() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<Vec<u8>, &str> = Ok(vec![]);
    assert!(must(val).is_empty());
    Ok(())
}

#[test]
fn test_must_ok_with_option_inside() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<Option<i32>, &str> = Ok(None);
    assert!(must(val).is_none());
    Ok(())
}

#[test]
fn test_must_ok_with_nested_result() -> Result<(), Box<dyn std::error::Error>> {
    let inner: Result<i32, &str> = Ok(99);
    let outer: Result<Result<i32, &str>, &str> = Ok(inner);
    let extracted = must(outer);
    assert_eq!(must(extracted), 99);
    Ok(())
}

#[test]
#[should_panic(expected = "unexpected Err")]
fn test_must_panics_with_io_error() {
    let val: Result<(), std::io::Error> =
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "file missing"));
    must(val);
}

#[test]
#[should_panic(expected = "file missing")]
fn test_must_panic_message_contains_error_debug_repr() {
    let val: Result<(), std::io::Error> =
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "file missing"));
    must(val);
}

#[test]
#[should_panic(expected = "unexpected Err: \"detailed error message\"")]
fn test_must_panic_message_includes_string_error() {
    let val: Result<i32, String> = Err("detailed error message".to_string());
    let _ = must(val);
}

// ===========================================================================
// must_some() — edge cases and panic message formatting
// ===========================================================================

#[test]
#[allow(unused_must_use)]
fn test_must_some_with_unit_type() -> Result<(), Box<dyn std::error::Error>> {
    let val: Option<()> = Some(());
    must_some(val);
    Ok(())
}

#[test]
fn test_must_some_with_zero() -> Result<(), Box<dyn std::error::Error>> {
    let val: Option<i32> = Some(0);
    assert_eq!(must_some(val), 0);
    Ok(())
}

#[test]
fn test_must_some_with_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    let val: Option<String> = Some(String::new());
    assert_eq!(must_some(val), "");
    Ok(())
}

#[test]
fn test_must_some_with_false() -> Result<(), Box<dyn std::error::Error>> {
    let val: Option<bool> = Some(false);
    assert!(!must_some(val));
    Ok(())
}

#[test]
fn test_must_some_with_nested_option() -> Result<(), Box<dyn std::error::Error>> {
    let val: Option<Option<i32>> = Some(None);
    assert!(must_some(val).is_none());
    Ok(())
}

#[test]
#[should_panic(expected = "unexpected None")]
fn test_must_some_panic_message_is_exact() {
    let val: Option<String> = None;
    let _ = must_some(val);
}

// ===========================================================================
// must_err() — edge cases and panic message formatting
// ===========================================================================

#[test]
#[allow(unused_must_use)]
fn test_must_err_with_unit_error() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<i32, ()> = Err(());
    must_err(val);
    Ok(())
}

#[test]
fn test_must_err_with_io_error() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<(), std::io::Error> =
        Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied"));
    let err = must_err(val);
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    Ok(())
}

#[test]
fn test_must_err_with_tuple_error() -> Result<(), Box<dyn std::error::Error>> {
    let val: Result<(), (i32, String)> = Err((404, "not found".to_string()));
    let (code, msg) = must_err(val);
    assert_eq!(code, 404);
    assert_eq!(msg, "not found");
    Ok(())
}

#[test]
#[should_panic(expected = "expected Err, got Ok")]
fn test_must_err_panic_message_contains_ok_debug() {
    let val: Result<Vec<i32>, &str> = Ok(vec![1, 2, 3]);
    let _ = must_err(val);
}

#[test]
#[should_panic(expected = "[1, 2, 3]")]
fn test_must_err_panic_message_shows_ok_value() {
    let val: Result<Vec<i32>, &str> = Ok(vec![1, 2, 3]);
    let _ = must_err(val);
}

#[test]
#[should_panic(expected = "expected Err, got Ok(42)")]
fn test_must_err_panic_message_shows_numeric_ok() {
    let val: Result<i32, &str> = Ok(42);
    let _ = must_err(val);
}

// ===========================================================================
// #[track_caller] — panics report the CALLER's location, not must.rs
// ===========================================================================

#[test]
fn test_must_track_caller_reports_test_file_location() {
    let result = std::panic::catch_unwind(|| {
        let val: Result<(), &str> = Err("boom");
        must(val);
    });
    assert!(result.is_err());
}

#[test]
fn test_must_some_track_caller_reports_test_file_location() {
    let result = std::panic::catch_unwind(|| {
        let val: Option<i32> = None;
        let _ = must_some(val);
    });
    assert!(result.is_err());
}

#[test]
fn test_must_err_track_caller_reports_test_file_location() {
    let result = std::panic::catch_unwind(|| {
        let val: Result<i32, &str> = Ok(99);
        let _ = must_err(val);
    });
    assert!(result.is_err());
}

// ===========================================================================
// tdd_basic::TddWorkflow — message content and state transitions
// ===========================================================================

#[test]
fn test_tdd_workflow_start_cycle_message_contains_name() -> Result<(), Box<dyn std::error::Error>> {
    let mut wf = TddWorkflow::new("Test::More");
    let result = wf.start_cycle("parse_heredoc");
    assert!(result.message.contains("parse_heredoc"));
    assert_eq!(result.state, TddState::Red);
    Ok(())
}

#[test]
fn test_tdd_workflow_run_tests_failing_message() -> Result<(), Box<dyn std::error::Error>> {
    let mut wf = TddWorkflow::new("Test::More");
    wf.start_cycle("foo");
    let result = wf.run_tests(false);
    assert!(result.message.contains("failing") || result.message.contains("fix"));
    assert_eq!(result.state, TddState::Red);
    Ok(())
}

#[test]
fn test_tdd_workflow_run_tests_passing_message() -> Result<(), Box<dyn std::error::Error>> {
    let mut wf = TddWorkflow::new("Test::More");
    wf.start_cycle("foo");
    let result = wf.run_tests(true);
    assert!(result.message.contains("passing") || result.message.contains("refactor"));
    assert_eq!(result.state, TddState::Green);
    Ok(())
}

#[test]
fn test_tdd_workflow_refactor_message() -> Result<(), Box<dyn std::error::Error>> {
    let mut wf = TddWorkflow::new("Test::More");
    wf.start_cycle("foo");
    wf.run_tests(true);
    let result = wf.start_refactor();
    assert!(result.message.contains("Refactor") || result.message.contains("refactor"));
    assert_eq!(result.state, TddState::Refactor);
    Ok(())
}

#[test]
fn test_tdd_workflow_complete_cycle_message() -> Result<(), Box<dyn std::error::Error>> {
    let mut wf = TddWorkflow::new("Test::More");
    wf.start_cycle("foo");
    wf.run_tests(true);
    wf.start_refactor();
    let result = wf.complete_cycle();
    assert!(result.message.contains("complete") || result.message.contains("Complete"));
    assert_eq!(result.state, TddState::Idle);
    Ok(())
}

#[test]
fn test_tdd_workflow_coverage_diagnostics_empty() -> Result<(), Box<dyn std::error::Error>> {
    let wf = TddWorkflow::new("Test::More");
    let diags = wf.get_coverage_diagnostics(&[]);
    assert!(diags.is_empty());
    Ok(())
}

#[test]
fn test_tdd_workflow_coverage_diagnostics_severity_and_code()
-> Result<(), Box<dyn std::error::Error>> {
    let wf = TddWorkflow::new("Test::More");
    let diags = wf.get_coverage_diagnostics(&[5, 10, 15]);
    assert_eq!(diags.len(), 3);
    for d in &diags {
        assert!(matches!(d.severity, DiagnosticSeverity::Warning));
        assert_eq!(d.code.as_deref(), Some("tdd.uncovered"));
        assert!(d.message.contains("not covered") || d.message.contains("Not covered"));
    }
    assert_eq!(diags[0].range, (5, 5));
    assert_eq!(diags[1].range, (10, 10));
    assert_eq!(diags[2].range, (15, 15));
    Ok(())
}

#[test]
fn test_tdd_workflow_repeated_cycles() -> Result<(), Box<dyn std::error::Error>> {
    let mut wf = TddWorkflow::new("Test::More");

    // First cycle
    wf.start_cycle("first");
    wf.run_tests(true);
    wf.start_refactor();
    wf.complete_cycle();

    // Second cycle — verify workflow resets cleanly
    let r = wf.start_cycle("second");
    assert_eq!(r.state, TddState::Red);
    assert!(r.message.contains("second"));
    wf.run_tests(true);
    wf.complete_cycle();

    Ok(())
}

// ===========================================================================
// tdd_basic::RefactoringAnalyzer — additional edge cases
// ===========================================================================

#[test]
fn test_refactoring_analyzer_empty_program() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = RefactoringAnalyzer::default();
    let ast =
        Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 });
    let suggestions = analyzer.analyze(&ast, "");
    assert!(suggestions.is_empty());
    Ok(())
}

#[test]
fn test_refactoring_analyzer_exactly_at_param_limit() -> Result<(), Box<dyn std::error::Error>> {
    // Default max_params = 5. At exactly 5 params, no suggestion should fire.
    let analyzer = RefactoringAnalyzer::default();
    let params: Vec<Node> = (0..5)
        .map(|i| {
            Node::new(
                NodeKind::MandatoryParameter {
                    variable: Box::new(Node::new(
                        NodeKind::Variable { sigil: "$".to_string(), name: format!("p{}", i) },
                        SourceLocation { start: 0, end: 0 },
                    )),
                },
                SourceLocation { start: 0, end: 0 },
            )
        })
        .collect();

    let ast = Node::new(
        NodeKind::Subroutine {
            name: Some("exact_limit".to_string()),
            name_span: None,
            prototype: None,
            signature: Some(Box::new(Node::new(
                NodeKind::Signature { parameters: params },
                SourceLocation { start: 0, end: 0 },
            ))),
            attributes: vec![],
            body: Box::new(Node::new(
                NodeKind::Block { statements: vec![] },
                SourceLocation { start: 0, end: 0 },
            )),
        },
        SourceLocation { start: 0, end: 0 },
    );

    let suggestions = analyzer.analyze(&ast, "sub exact_limit($p0, $p1, $p2, $p3, $p4) {}");
    // At max_params (5) — should NOT trigger TooManyParameters
    let too_many: Vec<_> = suggestions
        .iter()
        .filter(|s| s.category == RefactoringCategory::TooManyParameters)
        .collect();
    assert!(too_many.is_empty());
    Ok(())
}

#[test]
fn test_refactoring_analyzer_one_over_param_limit() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = RefactoringAnalyzer::default();
    let params: Vec<Node> = (0..6)
        .map(|i| {
            Node::new(
                NodeKind::MandatoryParameter {
                    variable: Box::new(Node::new(
                        NodeKind::Variable { sigil: "$".to_string(), name: format!("p{}", i) },
                        SourceLocation { start: 0, end: 0 },
                    )),
                },
                SourceLocation { start: 0, end: 0 },
            )
        })
        .collect();

    let ast = Node::new(
        NodeKind::Subroutine {
            name: Some("over_limit".to_string()),
            name_span: None,
            prototype: None,
            signature: Some(Box::new(Node::new(
                NodeKind::Signature { parameters: params },
                SourceLocation { start: 0, end: 0 },
            ))),
            attributes: vec![],
            body: Box::new(Node::new(
                NodeKind::Block { statements: vec![] },
                SourceLocation { start: 0, end: 0 },
            )),
        },
        SourceLocation { start: 0, end: 0 },
    );

    let suggestions = analyzer.analyze(&ast, "sub over_limit($p0,$p1,$p2,$p3,$p4,$p5) {}");
    let too_many: Vec<_> = suggestions
        .iter()
        .filter(|s| s.category == RefactoringCategory::TooManyParameters)
        .collect();
    assert!(!too_many.is_empty());
    assert!(too_many[0].title.contains("over_limit"));
    assert!(too_many[0].description.contains('6'));
    Ok(())
}

#[test]
fn test_refactoring_analyzer_anonymous_sub() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = RefactoringAnalyzer::default();
    // Anonymous subroutine (name = None) with too many params
    let params: Vec<Node> = (0..7)
        .map(|i| {
            Node::new(
                NodeKind::MandatoryParameter {
                    variable: Box::new(Node::new(
                        NodeKind::Variable { sigil: "$".to_string(), name: format!("x{}", i) },
                        SourceLocation { start: 0, end: 0 },
                    )),
                },
                SourceLocation { start: 0, end: 0 },
            )
        })
        .collect();

    let ast = Node::new(
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: Some(Box::new(Node::new(
                NodeKind::Signature { parameters: params },
                SourceLocation { start: 0, end: 0 },
            ))),
            attributes: vec![],
            body: Box::new(Node::new(
                NodeKind::Block { statements: vec![] },
                SourceLocation { start: 0, end: 0 },
            )),
        },
        SourceLocation { start: 0, end: 0 },
    );

    let suggestions = analyzer.analyze(&ast, "sub ($x0,$x1,$x2,$x3,$x4,$x5,$x6) {}");
    // Should mention "anonymous" since name is None
    let too_many: Vec<_> = suggestions
        .iter()
        .filter(|s| s.category == RefactoringCategory::TooManyParameters)
        .collect();
    assert!(!too_many.is_empty());
    assert!(too_many[0].title.contains("anonymous"));
    Ok(())
}

// ===========================================================================
// tdd_workflow::TddWorkflow (full) — coverage tracker boundary cases
// ===========================================================================

#[test]
fn test_full_tdd_workflow_start_cycle_state() -> Result<(), Box<dyn std::error::Error>> {
    let config = TddConfig::default();
    let mut wf = FullTddWorkflow::new(config);
    let result = wf.start_cycle("my_feature");
    assert!(result.message.contains("my_feature"));
    assert_eq!(result.phase, "Red");
    Ok(())
}

#[test]
fn test_full_tdd_workflow_coverage_threshold_no_data() -> Result<(), Box<dyn std::error::Error>> {
    let config = TddConfig::default();
    let wf = FullTddWorkflow::new(config);
    // No coverage data => total_coverage is 0.0, threshold is 80.0
    assert!(!wf.check_coverage_threshold());
    Ok(())
}

#[test]
fn test_full_tdd_workflow_coverage_threshold_met() -> Result<(), Box<dyn std::error::Error>> {
    let config = TddConfig { coverage_threshold: 50.0, ..TddConfig::default() };
    let mut wf = FullTddWorkflow::new(config);

    // 3 covered out of 4 lines = 75% > 50% threshold
    let coverage = vec![
        LineCoverage { line: 1, hits: 1, covered: true },
        LineCoverage { line: 2, hits: 5, covered: true },
        LineCoverage { line: 3, hits: 0, covered: false },
        LineCoverage { line: 4, hits: 2, covered: true },
    ];
    wf.update_coverage(PathBuf::from("test.pl"), coverage);
    assert!(wf.check_coverage_threshold());
    Ok(())
}

#[test]
fn test_full_tdd_workflow_coverage_threshold_not_met() -> Result<(), Box<dyn std::error::Error>> {
    let config = TddConfig { coverage_threshold: 90.0, ..TddConfig::default() };
    let mut wf = FullTddWorkflow::new(config);

    // 1 covered out of 4 = 25% < 90%
    let coverage = vec![
        LineCoverage { line: 1, hits: 0, covered: false },
        LineCoverage { line: 2, hits: 0, covered: false },
        LineCoverage { line: 3, hits: 0, covered: false },
        LineCoverage { line: 4, hits: 1, covered: true },
    ];
    wf.update_coverage(PathBuf::from("test.pl"), coverage);
    assert!(!wf.check_coverage_threshold());
    Ok(())
}

#[test]
fn test_full_tdd_workflow_inline_coverage_uncovered_lines() -> Result<(), Box<dyn std::error::Error>>
{
    let config = TddConfig::default();
    let mut wf = FullTddWorkflow::new(config);

    let coverage = vec![
        LineCoverage { line: 1, hits: 3, covered: true },
        LineCoverage { line: 2, hits: 0, covered: false },
        LineCoverage { line: 3, hits: 0, covered: false },
        LineCoverage { line: 4, hits: 1, covered: true },
    ];
    let path = PathBuf::from("my_module.pm");
    wf.update_coverage(path.clone(), coverage);

    let annotations = wf.get_inline_coverage(&path);
    assert_eq!(annotations.len(), 2);
    assert_eq!(annotations[0].line, 2);
    assert_eq!(annotations[1].line, 3);
    Ok(())
}

#[test]
fn test_full_tdd_workflow_inline_coverage_no_file() -> Result<(), Box<dyn std::error::Error>> {
    let config = TddConfig::default();
    let wf = FullTddWorkflow::new(config);
    let annotations = wf.get_inline_coverage(&PathBuf::from("nonexistent.pm"));
    assert!(annotations.is_empty());
    Ok(())
}

#[test]
fn test_full_tdd_workflow_status_idle() -> Result<(), Box<dyn std::error::Error>> {
    let config = TddConfig::default();
    let wf = FullTddWorkflow::new(config);
    let status = wf.get_status();
    assert_eq!(status.state, WorkflowState::Idle);
    assert!(status.tests_passing); // no test results = vacuously passing
    Ok(())
}

#[test]
fn test_full_tdd_workflow_coverage_diagnostics_for_file() -> Result<(), Box<dyn std::error::Error>>
{
    let config = TddConfig::default();
    let mut wf = FullTddWorkflow::new(config);

    let coverage = vec![
        LineCoverage { line: 10, hits: 0, covered: false },
        LineCoverage { line: 11, hits: 5, covered: true },
        LineCoverage { line: 12, hits: 0, covered: false },
    ];
    let path = PathBuf::from("target.pl");
    wf.update_coverage(path.clone(), coverage);

    let diags = wf.generate_coverage_diagnostics(&path);
    assert_eq!(diags.len(), 2);
    assert_eq!(diags[0].range, (10, 10));
    assert_eq!(diags[1].range, (12, 12));
    Ok(())
}

#[test]
fn test_full_tdd_workflow_coverage_diagnostics_empty_file() -> Result<(), Box<dyn std::error::Error>>
{
    let config = TddConfig::default();
    let wf = FullTddWorkflow::new(config);
    let diags = wf.generate_coverage_diagnostics(&PathBuf::from("missing.pl"));
    assert!(diags.is_empty());
    Ok(())
}

// ===========================================================================
// tdd_workflow::TddConfig — default values
// ===========================================================================

#[test]
fn test_tdd_config_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let config = TddConfig::default();
    assert!(config.auto_generate_tests);
    assert!(config.test_on_save);
    assert!(config.show_inline_coverage);
    assert_eq!(config.test_framework, "Test::More");
    assert!(config.test_file_pattern.contains("{name}"));
    assert!((config.coverage_threshold - 80.0).abs() < f64::EPSILON);
    assert!(config.continuous_testing);
    assert!(config.auto_suggest_refactorings);
    Ok(())
}

// ===========================================================================
// governance::IgnoredTestGuardian — quality score edge cases
// ===========================================================================

fn make_minimal_governance() -> IgnoredTestGovernance {
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
            max_deviation: 2,
            deviation_threshold_percent: 20.0,
            baseline_date: SystemTime::now(),
            next_review_date: SystemTime::now() + Duration::from_hours(720),
        },
        quality_gates: QualityGates {
            pre_commit: PreCommitValidation {
                require_justification: true,
                max_new_ignored_per_commit: 2,
                documentation_requirements: DocumentationRequirements {
                    require_issue_reference: false,
                    require_timeline: false,
                    require_success_criteria: false,
                    require_complexity_assessment: false,
                },
            },
            ci_validation: CiValidation {
                block_on_count_increase: false,
                max_ignored_per_crate: HashMap::new(),
                min_quality_score: 50.0,
            },
            metrics_tracking: MetricsTracking {
                track_trend: true,
                trend_window_days: 30,
                alert_on_negative_trend: false,
            },
        },
        reporting: ReportingConfiguration {
            daily_reports: false,
            weekly_trends: false,
            monthly_summaries: false,
            output_formats: vec![],
        },
    }
}

fn make_test_metadata_with_reason(reason: &str) -> IgnoredTestMetadata {
    IgnoredTestMetadata {
        test_id: "test_id".to_string(),
        file_path: PathBuf::from("tests/test.rs"),
        test_name: "test_function".to_string(),
        category: TestCategory::Infrastructure,
        priority: 2,
        ignore_reason: reason.to_string(),
        complexity: ComplexityLevel::Medium,
        target_timeline: Duration::from_hours(168),
        dependencies: vec!["dep_a".to_string()],
        success_criteria: vec![
            "criterion 1".to_string(),
            "criterion 2".to_string(),
            "criterion 3".to_string(),
        ],
        workflow_integration: LspWorkflowStage::Parse,
        performance_requirements: None,
        last_assessed: SystemTime::now(),
    }
}

#[test]
fn test_quality_score_high_for_well_documented() -> Result<(), Box<dyn std::error::Error>> {
    let governance = make_minimal_governance();
    let guardian = IgnoredTestGuardian::new(governance);

    let metadata = make_test_metadata_with_reason(
        "Requires implementation of the new parser backend for heredoc support (issue #200)",
    );
    let result = guardian.validate_new_ignored_test(&metadata);
    // Well-documented: long reason, 3+ success criteria, recent assessment, has dependencies
    // Should get high quality score
    assert!(
        result.quality_score >= 80.0,
        "Expected high quality score, got {}",
        result.quality_score,
    );
    Ok(())
}

#[test]
fn test_quality_score_penalty_for_short_reason() -> Result<(), Box<dyn std::error::Error>> {
    let governance = make_minimal_governance();
    let guardian = IgnoredTestGuardian::new(governance);

    // Short reason (< 20 chars) gets -20 penalty
    let short_reason_metadata = make_test_metadata_with_reason("TODO fix");
    let result = guardian.validate_new_ignored_test(&short_reason_metadata);

    // Compare with long reason
    let long_reason_metadata = make_test_metadata_with_reason(
        "Requires a complete rewrite of the lexer to support heredocs properly",
    );
    let long_result = guardian.validate_new_ignored_test(&long_reason_metadata);

    assert!(
        long_result.quality_score > result.quality_score,
        "Short reason ({}) should score lower than long reason ({})",
        result.quality_score,
        long_result.quality_score,
    );
    Ok(())
}

#[test]
fn test_quality_score_penalty_for_empty_success_criteria() -> Result<(), Box<dyn std::error::Error>>
{
    let governance = make_minimal_governance();
    let guardian = IgnoredTestGuardian::new(governance);

    let mut metadata = make_test_metadata_with_reason(
        "This is a sufficiently long ignore reason for testing purposes",
    );
    metadata.success_criteria = vec![];

    let result = guardian.validate_new_ignored_test(&metadata);
    // Empty success criteria gets -30 penalty
    assert!(
        result.quality_score <= 75.0,
        "Expected lower score for empty success criteria, got {}",
        result.quality_score,
    );
    Ok(())
}

#[test]
fn test_quality_score_penalty_for_old_assessment() -> Result<(), Box<dyn std::error::Error>> {
    let governance = make_minimal_governance();
    let guardian = IgnoredTestGuardian::new(governance);

    let mut metadata = make_test_metadata_with_reason(
        "This is a sufficiently long ignore reason for testing purposes",
    );
    // Assessed 120 days ago (>90 days threshold)
    metadata.last_assessed = SystemTime::now() - Duration::from_hours(2880);

    let result_old = guardian.validate_new_ignored_test(&metadata);

    metadata.last_assessed = SystemTime::now();
    let result_recent = guardian.validate_new_ignored_test(&metadata);

    assert!(
        result_recent.quality_score > result_old.quality_score,
        "Recent assessment ({}) should score higher than old assessment ({})",
        result_recent.quality_score,
        result_old.quality_score,
    );
    Ok(())
}

#[test]
fn test_quality_score_clamped_to_zero_minimum() -> Result<(), Box<dyn std::error::Error>> {
    let governance = make_minimal_governance();
    let guardian = IgnoredTestGuardian::new(governance);

    // Worst case: short reason (-20), empty criteria (-30), old assessment (-25),
    // no deps + non-low complexity (-10) = 100 - 85 = 15
    let metadata = IgnoredTestMetadata {
        test_id: "bad".to_string(),
        file_path: PathBuf::from("t.rs"),
        test_name: "bad".to_string(),
        category: TestCategory::EdgeCases,
        priority: 4,
        ignore_reason: "x".to_string(),
        complexity: ComplexityLevel::Critical,
        target_timeline: Duration::from_hours(1),
        dependencies: vec![],
        success_criteria: vec![],
        workflow_integration: LspWorkflowStage::CrossCutting,
        performance_requirements: None,
        last_assessed: SystemTime::now() - Duration::from_hours(8760),
    };

    let result = guardian.validate_new_ignored_test(&metadata);
    assert!(
        result.quality_score >= 0.0,
        "Quality score should never be negative, got {}",
        result.quality_score,
    );
    Ok(())
}

#[test]
fn test_quality_score_clamped_to_100_maximum() -> Result<(), Box<dyn std::error::Error>> {
    let governance = make_minimal_governance();
    let guardian = IgnoredTestGuardian::new(governance);

    // Best case: long reason, 3+ criteria (+5 bonus), deps, recent
    let metadata = make_test_metadata_with_reason(
        "Requires a comprehensive implementation of the entire error handling subsystem with full coverage",
    );

    let result = guardian.validate_new_ignored_test(&metadata);
    assert!(
        result.quality_score <= 100.0,
        "Quality score should never exceed 100, got {}",
        result.quality_score,
    );
    Ok(())
}

// ===========================================================================
// governance::IgnoredTestGuardian — baseline regression edge cases
// ===========================================================================

#[test]
fn test_baseline_regression_zero_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let mut governance = make_minimal_governance();
    governance.baseline_management.baseline_count = 0;
    governance.baseline_management.max_deviation = 0;
    let guardian = IgnoredTestGuardian::new(governance);

    // Zero baseline, zero current = no regression
    let result = guardian.check_baseline_regression(0);
    assert!(!result.is_regression);
    assert_eq!(result.absolute_increase, 0);

    // Zero baseline, current > 0 = regression (exceeds max_deviation of 0)
    let result = guardian.check_baseline_regression(1);
    assert!(result.is_regression);
    Ok(())
}

#[test]
fn test_baseline_regression_decrease_is_not_regression() -> Result<(), Box<dyn std::error::Error>> {
    let governance = make_minimal_governance();
    let guardian = IgnoredTestGuardian::new(governance);

    // Current count < baseline = improvement
    let result = guardian.check_baseline_regression(5);
    assert!(!result.is_regression);
    assert_eq!(result.absolute_increase, 0); // saturating_sub means no negative
    Ok(())
}

// ===========================================================================
// governance::IgnoredTestGuardian — trend report edge cases
// ===========================================================================

#[test]
fn test_trend_report_no_historical_data() -> Result<(), Box<dyn std::error::Error>> {
    let governance = make_minimal_governance();
    let guardian = IgnoredTestGuardian::new(governance);
    let report = guardian.generate_trend_report();

    assert_eq!(report.trend_direction, TrendDirection::Unknown);
    assert!(report.data_points.is_empty());
    assert!((report.average_count - 0.0).abs() < f64::EPSILON);
    assert!(!report.recommendations.is_empty()); // Unknown trend still gets recommendations
    Ok(())
}

#[test]
fn test_trend_report_single_data_point() -> Result<(), Box<dyn std::error::Error>> {
    let governance = make_minimal_governance();
    let mut guardian = IgnoredTestGuardian::new(governance);

    guardian.set_historical_data(vec![(SystemTime::now(), 42)]);
    let report = guardian.generate_trend_report();

    // Single data point cannot determine a trend
    assert_eq!(report.trend_direction, TrendDirection::Unknown);
    Ok(())
}

#[test]
fn test_trend_report_stable_trend() -> Result<(), Box<dyn std::error::Error>> {
    let mut governance = make_minimal_governance();
    governance.reporting.monthly_summaries = true;
    let mut guardian = IgnoredTestGuardian::new(governance);

    let now = SystemTime::now();
    // Two data points with very similar values -> stable
    guardian.set_historical_data(vec![(now - Duration::from_hours(120), 50), (now, 50)]);
    let report = guardian.generate_trend_report();
    assert_eq!(report.trend_direction, TrendDirection::Stable);
    Ok(())
}

// ===========================================================================
// governance::ValidationResult — require_issue_reference check
// ===========================================================================

#[test]
fn test_validation_requires_issue_reference_with_hash() -> Result<(), Box<dyn std::error::Error>> {
    let mut governance = make_minimal_governance();
    governance.quality_gates.pre_commit.documentation_requirements.require_issue_reference = true;
    let guardian = IgnoredTestGuardian::new(governance);

    // Reason contains '#' -> passes issue reference check
    let mut metadata = make_test_metadata_with_reason("Depends on feature from #42");
    metadata.success_criteria = vec!["criterion".to_string()];
    let result = guardian.validate_new_ignored_test(&metadata);
    let has_issue_error = result.errors.iter().any(|e| e.contains("issue"));
    assert!(!has_issue_error, "Should accept '#' as issue reference");
    Ok(())
}

#[test]
fn test_validation_requires_issue_reference_with_word() -> Result<(), Box<dyn std::error::Error>> {
    let mut governance = make_minimal_governance();
    governance.quality_gates.pre_commit.documentation_requirements.require_issue_reference = true;
    let guardian = IgnoredTestGuardian::new(governance);

    // Reason contains 'issue' -> passes issue reference check
    let mut metadata = make_test_metadata_with_reason("Blocked by upstream issue with dep");
    metadata.success_criteria = vec!["criterion".to_string()];
    let result = guardian.validate_new_ignored_test(&metadata);
    let has_issue_error = result.errors.iter().any(|e| e.contains("issue"));
    assert!(!has_issue_error, "Should accept 'issue' keyword as reference");
    Ok(())
}

#[test]
fn test_validation_fails_without_issue_reference() -> Result<(), Box<dyn std::error::Error>> {
    let mut governance = make_minimal_governance();
    governance.quality_gates.pre_commit.documentation_requirements.require_issue_reference = true;
    let guardian = IgnoredTestGuardian::new(governance);

    // Reason has neither '#' nor 'issue'
    let metadata = make_test_metadata_with_reason("This needs to be done eventually but no ref");
    let result = guardian.validate_new_ignored_test(&metadata);
    let has_issue_error = result.errors.iter().any(|e| e.contains("issue"));
    assert!(has_issue_error, "Should require issue reference");
    assert!(!result.is_valid);
    Ok(())
}

// ===========================================================================
// governance types — enum equality and display
// ===========================================================================

#[test]
fn test_trend_direction_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(TrendDirection::Increasing, TrendDirection::Increasing);
    assert_eq!(TrendDirection::Decreasing, TrendDirection::Decreasing);
    assert_eq!(TrendDirection::Stable, TrendDirection::Stable);
    assert_eq!(TrendDirection::Unknown, TrendDirection::Unknown);
    assert_ne!(TrendDirection::Increasing, TrendDirection::Decreasing);
    Ok(())
}

#[test]
fn test_report_format_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(ReportFormat::Json, ReportFormat::Json);
    assert_eq!(ReportFormat::Markdown, ReportFormat::Markdown);
    assert_eq!(ReportFormat::Html, ReportFormat::Html);
    assert_eq!(ReportFormat::Csv, ReportFormat::Csv);
    assert_ne!(ReportFormat::Json, ReportFormat::Csv);
    Ok(())
}

#[test]
fn test_complexity_level_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(ComplexityLevel::Low, ComplexityLevel::Low);
    assert_eq!(ComplexityLevel::Medium, ComplexityLevel::Medium);
    assert_eq!(ComplexityLevel::High, ComplexityLevel::High);
    assert_eq!(ComplexityLevel::Critical, ComplexityLevel::Critical);
    assert_ne!(ComplexityLevel::Low, ComplexityLevel::Critical);
    Ok(())
}

#[test]
fn test_test_category_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(TestCategory::CriticalLsp, TestCategory::CriticalLsp);
    assert_eq!(TestCategory::Infrastructure, TestCategory::Infrastructure);
    assert_eq!(TestCategory::AdvancedSyntax, TestCategory::AdvancedSyntax);
    assert_eq!(TestCategory::EdgeCases, TestCategory::EdgeCases);
    assert_ne!(TestCategory::CriticalLsp, TestCategory::EdgeCases);
    Ok(())
}

#[test]
fn test_lsp_workflow_stage_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(LspWorkflowStage::Parse, LspWorkflowStage::Parse);
    assert_eq!(LspWorkflowStage::Index, LspWorkflowStage::Index);
    assert_eq!(LspWorkflowStage::Navigate, LspWorkflowStage::Navigate);
    assert_eq!(LspWorkflowStage::Complete, LspWorkflowStage::Complete);
    assert_eq!(LspWorkflowStage::Analyze, LspWorkflowStage::Analyze);
    assert_eq!(LspWorkflowStage::CrossCutting, LspWorkflowStage::CrossCutting);
    assert_ne!(LspWorkflowStage::Parse, LspWorkflowStage::Analyze);
    Ok(())
}

// ===========================================================================
// TestStatus — comprehensive variant testing
// ===========================================================================

#[test]
fn test_test_status_debug() -> Result<(), Box<dyn std::error::Error>> {
    let debug_str = format!("{:?}", TestStatus::Passed);
    assert!(debug_str.contains("Passed"));
    let debug_str = format!("{:?}", TestStatus::Failed);
    assert!(debug_str.contains("Failed"));
    let debug_str = format!("{:?}", TestStatus::Skipped);
    assert!(debug_str.contains("Skipped"));
    let debug_str = format!("{:?}", TestStatus::Errored);
    assert!(debug_str.contains("Errored"));
    Ok(())
}

#[test]
fn test_test_status_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(TestStatus::Passed, TestStatus::Passed);
    assert_eq!(TestStatus::Failed, TestStatus::Failed);
    assert_ne!(TestStatus::Passed, TestStatus::Failed);
    assert_ne!(TestStatus::Skipped, TestStatus::Errored);
    Ok(())
}

// ===========================================================================
// tdd_basic::TestGenerator — find_subroutines in nested structures
// ===========================================================================

#[test]
fn test_find_subroutines_in_package() -> Result<(), Box<dyn std::error::Error>> {
    let generator = perl_tdd_support::tdd_basic::TestGenerator::new("Test::More");

    let ast = Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::Package {
                    name: "MyPkg".to_string(),
                    name_span: SourceLocation { start: 0, end: 5 },
                    block: Some(Box::new(Node::new(
                        NodeKind::Block {
                            statements: vec![Node::new(
                                NodeKind::Subroutine {
                                    name: Some("inside_package".to_string()),
                                    name_span: None,
                                    prototype: None,
                                    signature: None,
                                    attributes: vec![],
                                    body: Box::new(Node::new(
                                        NodeKind::Block { statements: vec![] },
                                        SourceLocation { start: 0, end: 0 },
                                    )),
                                },
                                SourceLocation { start: 0, end: 0 },
                            )],
                        },
                        SourceLocation { start: 0, end: 0 },
                    ))),
                },
                SourceLocation { start: 0, end: 0 },
            )],
        },
        SourceLocation { start: 0, end: 0 },
    );

    let subs = generator.find_subroutines(&ast);
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].name, "inside_package");
    Ok(())
}

#[test]
fn test_find_subroutines_in_if_branch() -> Result<(), Box<dyn std::error::Error>> {
    let generator = perl_tdd_support::tdd_basic::TestGenerator::new("Test::More");

    let ast = Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::If {
                    condition: Box::new(Node::new(
                        NodeKind::Number { value: "1".to_string() },
                        SourceLocation { start: 0, end: 0 },
                    )),
                    then_branch: Box::new(Node::new(
                        NodeKind::Block {
                            statements: vec![Node::new(
                                NodeKind::Subroutine {
                                    name: Some("in_then".to_string()),
                                    name_span: None,
                                    prototype: None,
                                    signature: None,
                                    attributes: vec![],
                                    body: Box::new(Node::new(
                                        NodeKind::Block { statements: vec![] },
                                        SourceLocation { start: 0, end: 0 },
                                    )),
                                },
                                SourceLocation { start: 0, end: 0 },
                            )],
                        },
                        SourceLocation { start: 0, end: 0 },
                    )),
                    elsif_branches: vec![],
                    else_branch: Some(Box::new(Node::new(
                        NodeKind::Block {
                            statements: vec![Node::new(
                                NodeKind::Subroutine {
                                    name: Some("in_else".to_string()),
                                    name_span: None,
                                    prototype: None,
                                    signature: None,
                                    attributes: vec![],
                                    body: Box::new(Node::new(
                                        NodeKind::Block { statements: vec![] },
                                        SourceLocation { start: 0, end: 0 },
                                    )),
                                },
                                SourceLocation { start: 0, end: 0 },
                            )],
                        },
                        SourceLocation { start: 0, end: 0 },
                    ))),
                },
                SourceLocation { start: 0, end: 0 },
            )],
        },
        SourceLocation { start: 0, end: 0 },
    );

    let subs = generator.find_subroutines(&ast);
    assert_eq!(subs.len(), 2);
    let names: Vec<&str> = subs.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"in_then"));
    assert!(names.contains(&"in_else"));
    Ok(())
}

// ===========================================================================
// tdd_basic::TestGenerator — Test2::V0 framework output
// ===========================================================================

#[test]
fn test_generator_test2_framework_output() -> Result<(), Box<dyn std::error::Error>> {
    let generator = perl_tdd_support::tdd_basic::TestGenerator::new("Test2::V0");
    let code = generator.generate_test("process_data", 3);
    assert!(code.contains("Test2::V0"));
    assert!(code.contains("process_data"));
    assert!(code.contains("arg1"));
    assert!(code.contains("arg2"));
    assert!(code.contains("arg3"));
    assert!(code.contains("done_testing"));
    Ok(())
}
