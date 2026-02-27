//! Integration tests for CI Guardrail Ignored Test Monitoring

use anyhow::Result;
use perl_tdd_support::governance::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[test]
fn test_ignored_test_guardian_validation() -> Result<()> {
    // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#ci-guardrail-system

    let governance = IgnoredTestGovernance {
        inventory: IgnoredTestInventory {
            total_count: 49,
            by_category: {
                let mut map = HashMap::new();
                map.insert(TestCategory::CriticalLsp, 24);
                map.insert(TestCategory::Infrastructure, 5);
                map.insert(TestCategory::AdvancedSyntax, 20);
                map
            },
            by_crate: {
                let mut map = HashMap::new();
                map.insert("perl-lsp".to_string(), 29);
                map.insert("tree-sitter-perl-rs".to_string(), 20);
                map
            },
            by_priority: {
                let mut map = HashMap::new();
                map.insert(1, 24); // Critical
                map.insert(2, 5); // Infrastructure
                map.insert(3, 20); // Advanced
                map
            },
            last_updated: SystemTime::now(),
        },
        baseline_management: BaselineManagement {
            baseline_count: 49,
            max_deviation: 5,
            deviation_threshold_percent: 10.0,
            baseline_date: SystemTime::now(),
            next_review_date: SystemTime::now() + Duration::from_secs(30 * 24 * 3600),
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
                max_ignored_per_crate: {
                    let mut map = HashMap::new();
                    map.insert("perl-lsp".to_string(), 30);
                    map.insert("tree-sitter-perl-rs".to_string(), 25);
                    map
                },
                min_quality_score: 70.0,
            },
            metrics_tracking: MetricsTracking {
                track_trend: true,
                trend_window_days: 90,
                alert_on_negative_trend: true,
            },
        },
        reporting: ReportingConfiguration {
            daily_reports: false,
            weekly_trends: true,
            monthly_summaries: true,
            output_formats: vec![ReportFormat::Json, ReportFormat::Markdown],
        },
    };

    let guardian = IgnoredTestGuardian::new(governance);

    // Test validation of well-documented ignored test
    let good_test = IgnoredTestMetadata {
        test_id: "test_good_example".to_string(),
        file_path: PathBuf::from("tests/example_test.rs"),
        test_name: "test_good_example".to_string(),
        category: TestCategory::CriticalLsp,
        priority: 1,
        ignore_reason: "Requires implementation of enhanced error handling system (issue #144)"
            .to_string(),
        complexity: ComplexityLevel::Medium,
        target_timeline: Duration::from_secs(14 * 24 * 3600), // 2 weeks
        dependencies: vec!["error_context_system".to_string()],
        success_criteria: vec![
            "Error responses include enhanced context".to_string(),
            "Malformed frame recovery implemented".to_string(),
            "Performance requirements met (<5ms)".to_string(),
        ],
        workflow_integration: LspWorkflowStage::Parse,
        performance_requirements: Some(PerformanceRequirements {
            max_latency_ms: 5,
            max_memory_mb: 1,
            min_throughput: None,
        }),
        last_assessed: SystemTime::now(),
    };

    let validation_result = guardian.validate_new_ignored_test(&good_test);
    assert!(validation_result.is_valid, "Well-documented test should pass validation");
    assert!(
        validation_result.quality_score >= 70.0,
        "Quality score should be high for well-documented test"
    );
    assert!(validation_result.errors.is_empty(), "Should have no validation errors");

    // Test validation of poorly documented ignored test
    let poor_test = IgnoredTestMetadata {
        test_id: "test_poor_example".to_string(),
        file_path: PathBuf::from("tests/poor_test.rs"),
        test_name: "test_poor_example".to_string(),
        category: TestCategory::EdgeCases,
        priority: 4,
        ignore_reason: "TODO".to_string(), // Poor documentation
        complexity: ComplexityLevel::Low,
        target_timeline: Duration::ZERO, // Missing timeline
        dependencies: vec![],
        success_criteria: vec![], // Missing criteria
        workflow_integration: LspWorkflowStage::CrossCutting,
        performance_requirements: None,
        last_assessed: SystemTime::now() - Duration::from_secs(120 * 24 * 3600), // Old
    };

    let poor_validation = guardian.validate_new_ignored_test(&poor_test);
    assert!(!poor_validation.is_valid, "Poorly documented test should fail validation");
    assert!(
        poor_validation.quality_score < 50.0,
        "Quality score should be low for poorly documented test"
    );
    assert!(!poor_validation.errors.is_empty(), "Should have validation errors");

    // Verify specific validation errors
    assert!(
        poor_validation.errors.iter().any(|e| e.contains("issue")),
        "Should require issue reference"
    );
    assert!(
        poor_validation.errors.iter().any(|e| e.contains("timeline")),
        "Should require timeline"
    );
    assert!(
        poor_validation.errors.iter().any(|e| e.contains("success criteria")),
        "Should require success criteria"
    );

    Ok(())
}

#[test]
fn test_baseline_regression_detection() -> Result<()> {
    // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#ci-guardrail-system

    let governance = IgnoredTestGovernance {
        inventory: IgnoredTestInventory {
            total_count: 49,
            by_category: HashMap::new(),
            by_crate: HashMap::new(),
            by_priority: HashMap::new(),
            last_updated: SystemTime::now(),
        },
        baseline_management: BaselineManagement {
            baseline_count: 49,
            max_deviation: 5,                  // Allow max 5 new ignored tests
            deviation_threshold_percent: 11.0, // Allow max 11% increase (5/49 is 10.2%)
            baseline_date: SystemTime::now(),
            next_review_date: SystemTime::now() + Duration::from_secs(30 * 24 * 3600),
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
                trend_window_days: 90,
                alert_on_negative_trend: true,
            },
        },
        reporting: ReportingConfiguration {
            daily_reports: false,
            weekly_trends: true,
            monthly_summaries: true,
            output_formats: vec![ReportFormat::Json],
        },
    };

    let guardian = IgnoredTestGuardian::new(governance);

    // Test cases for regression detection
    let test_cases = vec![
        // (current_count, should_be_regression, description)
        (49, false, "Same count as baseline - no regression"),
        (52, false, "Small increase within absolute limit - no regression"),
        (54, false, "At absolute limit - no regression"),
        (55, true, "Exceeds absolute limit - regression"),
        (60, true, "Significant increase - regression"),
        (45, false, "Decrease - improvement, not regression"),
        (54, false, "At percentage threshold (10%) - no regression"),
        (55, true, "Above percentage threshold - regression"),
    ];

    for (current_count, should_be_regression, description) in test_cases {
        let regression_result = guardian.check_baseline_regression(current_count);

        assert_eq!(
            regression_result.is_regression, should_be_regression,
            "Regression detection failed for: {} (count: {})",
            description, current_count
        );

        assert_eq!(
            regression_result.current_count, current_count,
            "Current count should match input"
        );
        assert_eq!(
            regression_result.baseline_count, 49,
            "Baseline count should match configuration"
        );

        if should_be_regression {
            assert!(
                regression_result.threshold_exceeded.is_some(),
                "Regression should include threshold exceeded message"
            );
        } else {
            assert!(
                regression_result.threshold_exceeded.is_none(),
                "Non-regression should not have threshold exceeded message"
            );
        }

        // Validate percentage calculation
        let expected_percentage = if regression_result.baseline_count > 0 {
            (regression_result.absolute_increase as f64 / regression_result.baseline_count as f64)
                * 100.0
        } else {
            0.0
        };

        assert!(
            (regression_result.percentage_increase - expected_percentage).abs() < 0.01,
            "Percentage calculation should be accurate"
        );
    }

    Ok(())
}

#[test]
fn test_ignored_test_trend_reporting() -> Result<()> {
    // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#ci-guardrail-system

    let governance = IgnoredTestGovernance {
        inventory: IgnoredTestInventory {
            total_count: 49,
            by_category: HashMap::new(),
            by_crate: HashMap::new(),
            by_priority: HashMap::new(),
            last_updated: SystemTime::now(),
        },
        baseline_management: BaselineManagement {
            baseline_count: 49,
            max_deviation: 5,
            deviation_threshold_percent: 10.0,
            baseline_date: SystemTime::now(),
            next_review_date: SystemTime::now() + Duration::from_secs(30 * 24 * 3600),
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
            daily_reports: false,
            weekly_trends: true,
            monthly_summaries: true,
            output_formats: vec![ReportFormat::Json, ReportFormat::Markdown],
        },
    };

    let mut guardian = IgnoredTestGuardian::new(governance);

    // Add historical data for trend analysis
    let now = SystemTime::now();
    let historical_data = vec![
        (now - Duration::from_secs(30 * 24 * 3600), 60), // 30 days ago: 60 tests
        (now - Duration::from_secs(25 * 24 * 3600), 58), // 25 days ago: 58 tests
        (now - Duration::from_secs(20 * 24 * 3600), 55), // 20 days ago: 55 tests
        (now - Duration::from_secs(15 * 24 * 3600), 52), // 15 days ago: 52 tests
        (now - Duration::from_secs(10 * 24 * 3600), 50), // 10 days ago: 50 tests
        (now - Duration::from_secs(5 * 24 * 3600), 49),  // 5 days ago: 49 tests
        (now, 49),                                       // Today: 49 tests
    ];

    guardian.set_historical_data(historical_data);

    // Generate trend report
    let trend_report = guardian.generate_trend_report();

    // Validate trend analysis
    assert_eq!(
        trend_report.trend_direction,
        TrendDirection::Decreasing,
        "Should detect decreasing trend from 60 to 49 tests"
    );

    assert!(!trend_report.data_points.is_empty(), "Should have data points in trend report");

    assert!(trend_report.average_count > 0.0, "Should calculate average count");

    // Average should be around 54.7 for the given data
    assert!(
        trend_report.average_count >= 50.0 && trend_report.average_count <= 60.0,
        "Average count should be reasonable for the data set"
    );

    // Validate recommendations for decreasing trend
    assert!(!trend_report.recommendations.is_empty(), "Should provide recommendations");

    assert!(
        trend_report.recommendations.iter().any(|r| r.contains("progress")),
        "Should recommend acknowledging progress for decreasing trend"
    );

    // Test trend reporting with increasing trend
    let increasing_data = vec![
        (now - Duration::from_secs(20 * 24 * 3600), 40), // 20 days ago: 40 tests
        (now - Duration::from_secs(15 * 24 * 3600), 42), // 15 days ago: 42 tests
        (now - Duration::from_secs(10 * 24 * 3600), 45), // 10 days ago: 45 tests
        (now - Duration::from_secs(5 * 24 * 3600), 47),  // 5 days ago: 47 tests
        (now, 49),                                       // Today: 49 tests
    ];

    guardian.set_historical_data(increasing_data);
    let increasing_trend_report = guardian.generate_trend_report();

    assert_eq!(
        increasing_trend_report.trend_direction,
        TrendDirection::Increasing,
        "Should detect increasing trend from 40 to 49 tests"
    );

    assert!(
        increasing_trend_report.recommendations.iter().any(|r| r.contains("systematic")),
        "Should recommend systematic approach for increasing trend"
    );

    Ok(())
}

#[test]
fn test_test_quality_validation() -> Result<()> {
    // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#ci-guardrail-system

    let governance = IgnoredTestGovernance {
        inventory: IgnoredTestInventory {
            total_count: 49,
            by_category: HashMap::new(),
            by_crate: HashMap::new(),
            by_priority: HashMap::new(),
            last_updated: SystemTime::now(),
        },
        baseline_management: BaselineManagement {
            baseline_count: 49,
            max_deviation: 5,
            deviation_threshold_percent: 10.0,
            baseline_date: SystemTime::now(),
            next_review_date: SystemTime::now() + Duration::from_secs(30 * 24 * 3600),
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
                trend_window_days: 90,
                alert_on_negative_trend: true,
            },
        },
        reporting: ReportingConfiguration {
            daily_reports: true,
            weekly_trends: true,
            monthly_summaries: true,
            output_formats: vec![ReportFormat::Json, ReportFormat::Markdown],
        },
    };

    let guardian = IgnoredTestGuardian::new(governance);

    // Test quality validation scenarios
    let quality_test_cases = vec![
        // High quality test
        (IgnoredTestMetadata {
            test_id: "high_quality_test".to_string(),
            file_path: PathBuf::from("tests/high_quality.rs"),
            test_name: "test_comprehensive_feature".to_string(),
            category: TestCategory::CriticalLsp,
            priority: 1,
            ignore_reason: "Requires implementation of enhanced LSP error handling with comprehensive context and malformed frame recovery (issue #144-AC1-AC2)".to_string(),
            complexity: ComplexityLevel::High,
            target_timeline: Duration::from_secs(21 * 24 * 3600), // 3 weeks
            dependencies: vec![
                "error_context_system".to_string(),
                "frame_recovery_mechanism".to_string(),
                "enhanced_diagnostic_publication".to_string(),
            ],
            success_criteria: vec![
                "Enhanced error responses include comprehensive context".to_string(),
                "Malformed frame recovery implemented with <10ms latency".to_string(),
                "Error response generation <5ms".to_string(),
                "Memory overhead <1MB for error context storage".to_string(),
                "All acceptance criteria AC1-AC2 validated".to_string(),
            ],
            workflow_integration: LspWorkflowStage::Parse,
            performance_requirements: Some(PerformanceRequirements {
                max_latency_ms: 5,
                max_memory_mb: 1,
                min_throughput: Some(1000.0),
            }),
            last_assessed: SystemTime::now(),
        }, 90.0, "High quality test with comprehensive documentation"),

        // Medium quality test
        (IgnoredTestMetadata {
            test_id: "medium_quality_test".to_string(),
            file_path: PathBuf::from("tests/medium_quality.rs"),
            test_name: "test_basic_feature".to_string(),
            category: TestCategory::Infrastructure,
            priority: 2,
            ignore_reason: "Requires performance baseline establishment for parsing efficiency (issue #418)".to_string(),
            complexity: ComplexityLevel::Medium,
            target_timeline: Duration::from_secs(14 * 24 * 3600), // 2 weeks
            dependencies: vec!["baseline_framework".to_string()],
            success_criteria: vec![
                "Baseline established for parsing operations".to_string(),
                "Performance requirements documented".to_string(),
            ],
            workflow_integration: LspWorkflowStage::Parse,
            performance_requirements: None,
            last_assessed: SystemTime::now() - Duration::from_secs(30 * 24 * 3600), // 30 days old
        }, 65.0, "Medium quality test with adequate documentation"),

        // Low quality test
        (IgnoredTestMetadata {
            test_id: "low_quality_test".to_string(),
            file_path: PathBuf::from("tests/low_quality.rs"),
            test_name: "test_something".to_string(),
            category: TestCategory::EdgeCases,
            priority: 4,
            ignore_reason: "Not implemented yet".to_string(),
            complexity: ComplexityLevel::Low,
            target_timeline: Duration::ZERO,
            dependencies: vec![],
            success_criteria: vec![],
            workflow_integration: LspWorkflowStage::CrossCutting,
            performance_requirements: None,
            last_assessed: SystemTime::now() - Duration::from_secs(120 * 24 * 3600), // 120 days old
        }, 30.0, "Low quality test with minimal documentation"),
    ];

    for (test_metadata, expected_min_score, description) in quality_test_cases {
        let validation_result = guardian.validate_new_ignored_test(&test_metadata);

        // Quality score should be within reasonable range of expected
        assert!(
            validation_result.quality_score >= expected_min_score - 10.0,
            "{}: Quality score {} should be >= {} - 10",
            description,
            validation_result.quality_score,
            expected_min_score
        );

        // High quality tests should pass CI validation
        if validation_result.quality_score
            >= guardian.governance.quality_gates.ci_validation.min_quality_score
        {
            assert!(
                validation_result.is_valid || validation_result.errors.is_empty(),
                "High quality test should pass validation or have minimal errors"
            );
        }

        // Validate specific quality criteria
        match test_metadata.complexity {
            ComplexityLevel::High => {
                assert!(
                    test_metadata.success_criteria.len() >= 3,
                    "High complexity tests should have multiple success criteria"
                );
                assert!(
                    !test_metadata.dependencies.is_empty(),
                    "High complexity tests should have dependencies"
                );
            }
            ComplexityLevel::Low => {
                if test_metadata.target_timeline > Duration::from_secs(14 * 24 * 3600) {
                    assert!(
                        !validation_result.warnings.is_empty(),
                        "Low complexity with long timeline should generate warnings"
                    );
                }
            }
            _ => {}
        }
    }

    Ok(())
}
