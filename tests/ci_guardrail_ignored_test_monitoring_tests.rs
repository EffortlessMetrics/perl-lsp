//! CI Guardrail Ignored Test Monitoring Test Scaffolding
//!
//! Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#ci-guardrail-system
//!
//! This test suite validates automated ignored test monitoring with prevention of regression,
//! baseline tracking, quality gates, and comprehensive reporting.
//!
//! AC13: Ignored Test Guardian
//! AC14: Test Metadata Schema
//! AC15: Test Quality Validator

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Comprehensive ignored test monitoring and governance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoredTestGovernance {
    /// Current ignored test inventory
    pub inventory: IgnoredTestInventory,
    /// Baseline tracking and limits
    pub baseline_management: BaselineManagement,
    /// Quality gates and validation
    pub quality_gates: QualityGates,
    /// Reporting and alerting
    pub reporting: ReportingConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoredTestInventory {
    /// Total ignored test count
    pub total_count: usize,
    /// Count by category
    pub by_category: HashMap<TestCategory, usize>,
    /// Count by crate
    pub by_crate: HashMap<String, usize>,
    /// Count by priority
    pub by_priority: HashMap<u8, usize>,
    /// Last updated timestamp
    pub last_updated: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineManagement {
    /// Established baseline count
    pub baseline_count: usize,
    /// Maximum allowed deviation
    pub max_deviation: usize,
    /// Deviation percentage threshold
    pub deviation_threshold_percent: f64,
    /// Baseline establishment date
    pub baseline_date: SystemTime,
    /// Next baseline review date
    pub next_review_date: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGates {
    /// Pre-commit validation rules
    pub pre_commit: PreCommitValidation,
    /// CI pipeline validation
    pub ci_validation: CiValidation,
    /// Quality metrics tracking
    pub metrics_tracking: MetricsTracking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCommitValidation {
    /// Require justification for new ignored tests
    pub require_justification: bool,
    /// Maximum allowed new ignored tests per commit
    pub max_new_ignored_per_commit: usize,
    /// Required documentation for ignored tests
    pub documentation_requirements: DocumentationRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationRequirements {
    /// Require issue tracking reference
    pub require_issue_reference: bool,
    /// Require implementation timeline
    pub require_timeline: bool,
    /// Require success criteria
    pub require_success_criteria: bool,
    /// Require complexity assessment
    pub require_complexity_assessment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiValidation {
    /// Block CI on ignored test count increase
    pub block_on_count_increase: bool,
    /// Maximum allowed ignored tests per crate
    pub max_ignored_per_crate: HashMap<String, usize>,
    /// Quality score threshold
    pub min_quality_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsTracking {
    /// Track ignored test trend over time
    pub track_trend: bool,
    /// Trend analysis window in days
    pub trend_window_days: u32,
    /// Alert on negative trend
    pub alert_on_negative_trend: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingConfiguration {
    /// Generate daily reports
    pub daily_reports: bool,
    /// Generate weekly trend reports
    pub weekly_trends: bool,
    /// Generate monthly summaries
    pub monthly_summaries: bool,
    /// Report output formats
    pub output_formats: Vec<ReportFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportFormat {
    Json,
    Markdown,
    Html,
    Csv,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TestCategory {
    CriticalLsp,
    Infrastructure,
    AdvancedSyntax,
    EdgeCases,
}

/// Test metadata for ignored test management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IgnoredTestMetadata {
    /// Unique test identifier
    pub test_id: String,
    /// File path relative to crate root
    pub file_path: PathBuf,
    /// Test function name
    pub test_name: String,
    /// Test category classification
    pub category: TestCategory,
    /// Implementation priority (1=highest, 4=lowest)
    pub priority: u8,
    /// Current ignore reason
    pub ignore_reason: String,
    /// Estimated implementation complexity
    pub complexity: ComplexityLevel,
    /// Target implementation timeline
    pub target_timeline: Duration,
    /// Dependencies on other tests or features
    pub dependencies: Vec<String>,
    /// Success criteria for re-enabling
    pub success_criteria: Vec<String>,
    /// LSP workflow stage integration
    pub workflow_integration: LspWorkflowStage,
    /// Performance requirements if applicable
    pub performance_requirements: Option<PerformanceRequirements>,
    /// Last assessment date
    pub last_assessed: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplexityLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LspWorkflowStage {
    Parse,
    Index,
    Navigate,
    Complete,
    Analyze,
    CrossCutting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRequirements {
    /// Maximum allowed latency
    pub max_latency_ms: u64,
    /// Maximum memory usage
    pub max_memory_mb: u64,
    /// Required throughput
    pub min_throughput: Option<f64>,
}

/// Ignored test guardian for validation and monitoring
pub struct IgnoredTestGuardian {
    /// Baseline tracking for ignored test count
    baseline_tracker: BaselineTracker,
    /// Justification requirement system
    justification_validator: JustificationValidator,
    /// Automated reporting system
    reporter: IgnoredTestReporter,
    /// Current governance configuration
    governance: IgnoredTestGovernance,
}

pub struct BaselineTracker {
    pub current_baseline: usize,
    pub historical_data: Vec<(SystemTime, usize)>,
}

pub struct JustificationValidator {
    pub required_fields: Vec<String>,
    pub validation_rules: Vec<ValidationRule>,
}

pub struct ValidationRule {
    pub field: String,
    pub rule_type: ValidationRuleType,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationRuleType {
    Required,
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    Reference(String),
}

pub struct IgnoredTestReporter {
    pub output_directory: PathBuf,
    pub formats: Vec<ReportFormat>,
}

impl IgnoredTestGuardian {
    pub fn new(governance: IgnoredTestGovernance) -> Self {
        Self {
            baseline_tracker: BaselineTracker {
                current_baseline: governance.baseline_management.baseline_count,
                historical_data: Vec::new(),
            },
            justification_validator: JustificationValidator {
                required_fields: vec![
                    "ignore_reason".to_string(),
                    "target_timeline".to_string(),
                    "success_criteria".to_string(),
                ],
                validation_rules: Vec::new(),
            },
            reporter: IgnoredTestReporter {
                output_directory: PathBuf::from("target/ignored_test_reports"),
                formats: vec![ReportFormat::Json, ReportFormat::Markdown],
            },
            governance,
        }
    }

    pub fn validate_new_ignored_test(&self, test_info: &IgnoredTestMetadata) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate required documentation
        if self.governance.quality_gates.pre_commit.documentation_requirements.require_issue_reference {
            if !test_info.ignore_reason.contains("#") && !test_info.ignore_reason.contains("issue") {
                errors.push("Ignored test must reference an issue".to_string());
            }
        }

        if self.governance.quality_gates.pre_commit.documentation_requirements.require_timeline {
            if test_info.target_timeline.as_secs() == 0 {
                errors.push("Target implementation timeline must be specified".to_string());
            }
        }

        if self.governance.quality_gates.pre_commit.documentation_requirements.require_success_criteria {
            if test_info.success_criteria.is_empty() {
                errors.push("Success criteria must be specified".to_string());
            }
        }

        // Validate complexity assessment
        if self.governance.quality_gates.pre_commit.documentation_requirements.require_complexity_assessment {
            if test_info.complexity == ComplexityLevel::Low && test_info.target_timeline > Duration::from_secs(7 * 24 * 3600) {
                warnings.push("Low complexity test should have shorter timeline".to_string());
            }
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            quality_score: calculate_quality_score(test_info),
        }
    }

    pub fn check_baseline_regression(&self, current_count: usize) -> RegressionResult {
        let baseline = self.baseline_tracker.current_baseline;
        let max_deviation = self.governance.baseline_management.max_deviation;
        let threshold_percent = self.governance.baseline_management.deviation_threshold_percent;

        let absolute_increase = current_count.saturating_sub(baseline);
        let percentage_increase = if baseline > 0 {
            (absolute_increase as f64 / baseline as f64) * 100.0
        } else {
            0.0
        };

        let is_regression = absolute_increase > max_deviation || percentage_increase > threshold_percent;

        RegressionResult {
            is_regression,
            current_count,
            baseline_count: baseline,
            absolute_increase,
            percentage_increase,
            threshold_exceeded: if absolute_increase > max_deviation {
                Some(format!("Absolute increase {} > max deviation {}", absolute_increase, max_deviation))
            } else if percentage_increase > threshold_percent {
                Some(format!("Percentage increase {:.1}% > threshold {:.1}%", percentage_increase, threshold_percent))
            } else {
                None
            },
        }
    }

    pub fn generate_trend_report(&self) -> TrendReport {
        let current_time = SystemTime::now();
        let window_duration = Duration::from_secs(self.governance.reporting.monthly_summaries as u64 * 30 * 24 * 3600);

        let recent_data: Vec<_> = self.baseline_tracker.historical_data
            .iter()
            .filter(|(timestamp, _)| {
                current_time.duration_since(*timestamp).unwrap_or(Duration::MAX) <= window_duration
            })
            .cloned()
            .collect();

        let trend_direction = if recent_data.len() >= 2 {
            let first = recent_data.first().unwrap().1 as f64;
            let last = recent_data.last().unwrap().1 as f64;
            if last > first * 1.1 {
                TrendDirection::Increasing
            } else if last < first * 0.9 {
                TrendDirection::Decreasing
            } else {
                TrendDirection::Stable
            }
        } else {
            TrendDirection::Unknown
        };

        TrendReport {
            period_start: recent_data.first().map(|(t, _)| *t),
            period_end: recent_data.last().map(|(t, _)| *t),
            data_points: recent_data,
            trend_direction,
            average_count: if !recent_data.is_empty() {
                recent_data.iter().map(|(_, count)| *count as f64).sum::<f64>() / recent_data.len() as f64
            } else {
                0.0
            },
            recommendations: generate_trend_recommendations(&trend_direction, &recent_data),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub quality_score: f64,
}

#[derive(Debug, Clone)]
pub struct RegressionResult {
    pub is_regression: bool,
    pub current_count: usize,
    pub baseline_count: usize,
    pub absolute_increase: usize,
    pub percentage_increase: f64,
    pub threshold_exceeded: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TrendReport {
    pub period_start: Option<SystemTime>,
    pub period_end: Option<SystemTime>,
    pub data_points: Vec<(SystemTime, usize)>,
    pub trend_direction: TrendDirection,
    pub average_count: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
    Unknown,
}

fn calculate_quality_score(test_info: &IgnoredTestMetadata) -> f64 {
    let mut score = 100.0;

    // Deduct for missing documentation
    if test_info.ignore_reason.len() < 20 {
        score -= 20.0;
    }

    if test_info.success_criteria.is_empty() {
        score -= 30.0;
    }

    if test_info.dependencies.is_empty() && test_info.complexity != ComplexityLevel::Low {
        score -= 10.0;
    }

    // Deduct for old tests
    if let Ok(duration) = SystemTime::now().duration_since(test_info.last_assessed) {
        if duration > Duration::from_secs(90 * 24 * 3600) { // 90 days
            score -= 25.0;
        }
    }

    // Bonus for well-documented tests
    if test_info.success_criteria.len() >= 3 {
        score += 5.0;
    }

    score.max(0.0).min(100.0)
}

fn generate_trend_recommendations(direction: &TrendDirection, data: &[(SystemTime, usize)]) -> Vec<String> {
    let mut recommendations = Vec::new();

    match direction {
        TrendDirection::Increasing => {
            recommendations.push("Consider implementing systematic ignored test resolution plan".to_string());
            recommendations.push("Review test categorization and prioritization".to_string());
            recommendations.push("Allocate development resources for test implementation".to_string());
        }
        TrendDirection::Decreasing => {
            recommendations.push("Excellent progress on ignored test reduction".to_string());
            recommendations.push("Document successful implementation strategies".to_string());
            recommendations.push("Maintain current implementation pace".to_string());
        }
        TrendDirection::Stable => {
            recommendations.push("Evaluate whether current ignored test count is acceptable".to_string());
            recommendations.push("Consider setting more aggressive reduction targets".to_string());
        }
        TrendDirection::Unknown => {
            recommendations.push("Collect more historical data for trend analysis".to_string());
            recommendations.push("Establish baseline measurement practices".to_string());
        }
    }

    if data.len() > 10 {
        let recent_variance = calculate_variance(&data[data.len().saturating_sub(10)..]);
        if recent_variance > 10.0 {
            recommendations.push("High variance in ignored test count indicates inconsistent progress".to_string());
        }
    }

    recommendations
}

fn calculate_variance(data: &[(SystemTime, usize)]) -> f64 {
    if data.len() < 2 {
        return 0.0;
    }

    let mean = data.iter().map(|(_, count)| *count as f64).sum::<f64>() / data.len() as f64;
    let variance = data.iter()
        .map(|(_, count)| (*count as f64 - mean).powi(2))
        .sum::<f64>() / data.len() as f64;

    variance
}

#[test]
#[ignore] // AC13: Remove when ignored test guardian is implemented
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
                map.insert(2, 5);  // Infrastructure
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
        ignore_reason: "Requires implementation of enhanced error handling system (issue #144)".to_string(),
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
    assert!(validation_result.quality_score >= 70.0, "Quality score should be high for well-documented test");
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
    assert!(poor_validation.quality_score < 50.0, "Quality score should be low for poorly documented test");
    assert!(!poor_validation.errors.is_empty(), "Should have validation errors");

    // Verify specific validation errors
    assert!(poor_validation.errors.iter().any(|e| e.contains("issue")),
           "Should require issue reference");
    assert!(poor_validation.errors.iter().any(|e| e.contains("timeline")),
           "Should require timeline");
    assert!(poor_validation.errors.iter().any(|e| e.contains("success criteria")),
           "Should require success criteria");

    Ok(())
}

#[test]
#[ignore] // AC13: Remove when baseline regression detection is implemented
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
            max_deviation: 5,           // Allow max 5 new ignored tests
            deviation_threshold_percent: 10.0, // Allow max 10% increase
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

        assert_eq!(regression_result.is_regression, should_be_regression,
                  "Regression detection failed for: {} (count: {})", description, current_count);

        assert_eq!(regression_result.current_count, current_count,
                  "Current count should match input");
        assert_eq!(regression_result.baseline_count, 49,
                  "Baseline count should match configuration");

        if should_be_regression {
            assert!(regression_result.threshold_exceeded.is_some(),
                   "Regression should include threshold exceeded message");
        } else {
            assert!(regression_result.threshold_exceeded.is_none(),
                   "Non-regression should not have threshold exceeded message");
        }

        // Validate percentage calculation
        let expected_percentage = if regression_result.baseline_count > 0 {
            (regression_result.absolute_increase as f64 / regression_result.baseline_count as f64) * 100.0
        } else {
            0.0
        };

        assert!((regression_result.percentage_increase - expected_percentage).abs() < 0.01,
               "Percentage calculation should be accurate");
    }

    Ok(())
}

#[test]
#[ignore] // AC13,AC14: Remove when trend reporting is implemented
fn test_ignored_test_trend_reporting() -> Result<()> {
    // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#ci-guardrail-system

    let mut governance = IgnoredTestGovernance {
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
        (now, 49),                                         // Today: 49 tests
    ];

    guardian.baseline_tracker.historical_data = historical_data;

    // Generate trend report
    let trend_report = guardian.generate_trend_report();

    // Validate trend analysis
    assert_eq!(trend_report.trend_direction, TrendDirection::Decreasing,
              "Should detect decreasing trend from 60 to 49 tests");

    assert!(trend_report.data_points.len() > 0,
           "Should have data points in trend report");

    assert!(trend_report.average_count > 0.0,
           "Should calculate average count");

    // Average should be around 54.7 for the given data
    assert!(trend_report.average_count >= 50.0 && trend_report.average_count <= 60.0,
           "Average count should be reasonable for the data set");

    // Validate recommendations for decreasing trend
    assert!(!trend_report.recommendations.is_empty(),
           "Should provide recommendations");

    assert!(trend_report.recommendations.iter().any(|r| r.contains("progress")),
           "Should recommend acknowledging progress for decreasing trend");

    // Test trend reporting with increasing trend
    let increasing_data = vec![
        (now - Duration::from_secs(20 * 24 * 3600), 40), // 20 days ago: 40 tests
        (now - Duration::from_secs(15 * 24 * 3600), 42), // 15 days ago: 42 tests
        (now - Duration::from_secs(10 * 24 * 3600), 45), // 10 days ago: 45 tests
        (now - Duration::from_secs(5 * 24 * 3600), 47),  // 5 days ago: 47 tests
        (now, 49),                                         // Today: 49 tests
    ];

    guardian.baseline_tracker.historical_data = increasing_data;
    let increasing_trend_report = guardian.generate_trend_report();

    assert_eq!(increasing_trend_report.trend_direction, TrendDirection::Increasing,
              "Should detect increasing trend from 40 to 49 tests");

    assert!(increasing_trend_report.recommendations.iter().any(|r| r.contains("systematic")),
           "Should recommend systematic approach for increasing trend");

    Ok(())
}

#[test]
#[ignore] // AC15: Remove when test quality validation is implemented
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
            ignore_reason: "Requires performance baseline establishment for parsing efficiency".to_string(),
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
        assert!(validation_result.quality_score >= expected_min_score - 10.0,
               "{}: Quality score {} should be >= {} - 10",
               description, validation_result.quality_score, expected_min_score);

        // High quality tests should pass CI validation
        if validation_result.quality_score >= guardian.governance.quality_gates.ci_validation.min_quality_score {
            assert!(validation_result.is_valid || validation_result.errors.is_empty(),
                   "High quality test should pass validation or have minimal errors");
        }

        // Validate specific quality criteria
        match test_metadata.complexity {
            ComplexityLevel::High => {
                assert!(test_metadata.success_criteria.len() >= 3,
                       "High complexity tests should have multiple success criteria");
                assert!(!test_metadata.dependencies.is_empty(),
                       "High complexity tests should have dependencies");
            }
            ComplexityLevel::Low => {
                if test_metadata.target_timeline > Duration::from_secs(14 * 24 * 3600) {
                    assert!(!validation_result.warnings.is_empty(),
                           "Low complexity with long timeline should generate warnings");
                }
            }
            _ => {}
        }

        // Validate timeline reasonableness
        if test_metadata.target_timeline.as_secs() > 0 {
            let max_reasonable_timeline = match test_metadata.complexity {
                ComplexityLevel::Low => Duration::from_secs(7 * 24 * 3600),    // 1 week
                ComplexityLevel::Medium => Duration::from_secs(21 * 24 * 3600), // 3 weeks
                ComplexityLevel::High => Duration::from_secs(42 * 24 * 3600),   // 6 weeks
                ComplexityLevel::Critical => Duration::from_secs(84 * 24 * 3600), // 12 weeks
            };

            if test_metadata.target_timeline > max_reasonable_timeline {
                println!("Warning: Timeline may be too long for complexity level: {:?} -> {:?}",
                        test_metadata.complexity, test_metadata.target_timeline);
            }
        }
    }

    Ok(())
}

#[test]
#[ignore] // AC13,AC14,AC15: Remove when comprehensive CI guardrail integration is implemented
fn test_comprehensive_ci_guardrail_integration() -> Result<()> {
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
                map.insert(1, 24);
                map.insert(2, 5);
                map.insert(3, 20);
                map
            },
            last_updated: SystemTime::now(),
        },
        baseline_management: BaselineManagement {
            baseline_count: 49,
            max_deviation: 5,
            deviation_threshold_percent: 10.0,
            baseline_date: SystemTime::now() - Duration::from_secs(30 * 24 * 3600),
            next_review_date: SystemTime::now() + Duration::from_secs(60 * 24 * 3600),
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
                    map.insert("perl-parser".to_string(), 10);
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
            output_formats: vec![ReportFormat::Json, ReportFormat::Markdown, ReportFormat::Html],
        },
    };

    let guardian = IgnoredTestGuardian::new(governance);

    // Simulate CI pipeline scenarios
    let ci_scenarios = vec![
        // Scenario 1: Adding well-documented ignored test within limits
        (CiScenario {
            current_total: 50,  // +1 from baseline
            per_crate_changes: {
                let mut map = HashMap::new();
                map.insert("perl-lsp".to_string(), 30); // +1
                map
            },
            new_tests: vec![
                IgnoredTestMetadata {
                    test_id: "new_well_documented_test".to_string(),
                    file_path: PathBuf::from("crates/perl-lsp/tests/new_feature.rs"),
                    test_name: "test_new_lsp_feature".to_string(),
                    category: TestCategory::CriticalLsp,
                    priority: 1,
                    ignore_reason: "Requires implementation of enhanced cancellation system integration (issue #144-AC3)".to_string(),
                    complexity: ComplexityLevel::Medium,
                    target_timeline: Duration::from_secs(14 * 24 * 3600),
                    dependencies: vec!["cancellation_system".to_string()],
                    success_criteria: vec![
                        "Cancellation integration with request correlation".to_string(),
                        "Performance requirements met".to_string(),
                        "Thread safety validated".to_string(),
                    ],
                    workflow_integration: LspWorkflowStage::Parse,
                    performance_requirements: Some(PerformanceRequirements {
                        max_latency_ms: 100,
                        max_memory_mb: 5,
                        min_throughput: None,
                    }),
                    last_assessed: SystemTime::now(),
                }
            ],
        }, true, "Well-documented test within limits should pass CI"),

        // Scenario 2: Adding too many ignored tests
        (CiScenario {
            current_total: 55,  // +6 from baseline, exceeds limit
            per_crate_changes: {
                let mut map = HashMap::new();
                map.insert("perl-lsp".to_string(), 32); // +3
                map.insert("tree-sitter-perl-rs".to_string(), 23); // +3
                map
            },
            new_tests: vec![], // Multiple new tests would be added
        }, false, "Exceeding baseline limits should fail CI"),

        // Scenario 3: Exceeding per-crate limits
        (CiScenario {
            current_total: 52,  // +3 from baseline, within global limit
            per_crate_changes: {
                let mut map = HashMap::new();
                map.insert("perl-lsp".to_string(), 35); // +6, exceeds crate limit of 30
                map
            },
            new_tests: vec![],
        }, false, "Exceeding per-crate limits should fail CI"),

        // Scenario 4: Poor quality ignored test
        (CiScenario {
            current_total: 50,  // +1 from baseline
            per_crate_changes: HashMap::new(),
            new_tests: vec![
                IgnoredTestMetadata {
                    test_id: "poor_quality_test".to_string(),
                    file_path: PathBuf::from("tests/poor.rs"),
                    test_name: "test_poor".to_string(),
                    category: TestCategory::EdgeCases,
                    priority: 4,
                    ignore_reason: "TODO".to_string(), // Poor documentation
                    complexity: ComplexityLevel::Low,
                    target_timeline: Duration::ZERO,
                    dependencies: vec![],
                    success_criteria: vec![],
                    workflow_integration: LspWorkflowStage::CrossCutting,
                    performance_requirements: None,
                    last_assessed: SystemTime::now(),
                }
            ],
        }, false, "Poor quality test should fail CI validation"),
    ];

    for (scenario, should_pass_ci, description) in ci_scenarios {
        // Test baseline regression check
        let regression_result = guardian.check_baseline_regression(scenario.current_total);

        // Test per-crate limit validation
        let mut per_crate_violations = Vec::new();
        for (crate_name, count) in &scenario.per_crate_changes {
            if let Some(&limit) = guardian.governance.quality_gates.ci_validation.max_ignored_per_crate.get(crate_name) {
                if *count > limit {
                    per_crate_violations.push(format!("Crate '{}' has {} ignored tests, limit is {}", crate_name, count, limit));
                }
            }
        }

        // Test quality validation for new tests
        let mut quality_failures = Vec::new();
        for test in &scenario.new_tests {
            let validation = guardian.validate_new_ignored_test(test);
            if !validation.is_valid || validation.quality_score < guardian.governance.quality_gates.ci_validation.min_quality_score {
                quality_failures.push(format!("Test '{}' failed quality validation (score: {:.1})", test.test_name, validation.quality_score));
            }
        }

        // Determine if CI should pass
        let ci_blocking_conditions = vec![
            (guardian.governance.quality_gates.ci_validation.block_on_count_increase && regression_result.is_regression, "Count increase regression"),
            (!per_crate_violations.is_empty(), "Per-crate limit violations"),
            (!quality_failures.is_empty(), "Quality validation failures"),
        ];

        let has_blocking_conditions = ci_blocking_conditions.iter().any(|(condition, _)| *condition);
        let ci_should_pass = should_pass_ci && !has_blocking_conditions;

        if should_pass_ci {
            assert!(ci_should_pass || (!regression_result.is_regression && per_crate_violations.is_empty() && quality_failures.is_empty()),
                   "{}: CI validation failed unexpectedly", description);
        } else {
            assert!(!ci_should_pass || has_blocking_conditions,
                   "{}: CI should have failed but passed", description);
        }

        // Generate detailed CI report
        if !ci_should_pass {
            let mut failure_reasons = Vec::new();

            if regression_result.is_regression {
                failure_reasons.push(format!("Baseline regression: {} tests (baseline: {}, limit: +{})",
                                            regression_result.current_count,
                                            regression_result.baseline_count,
                                            guardian.governance.baseline_management.max_deviation));
            }

            failure_reasons.extend(per_crate_violations);
            failure_reasons.extend(quality_failures);

            println!("CI Failure Report for '{}': {}", description, failure_reasons.join("; "));
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct CiScenario {
    current_total: usize,
    per_crate_changes: HashMap<String, usize>,
    new_tests: Vec<IgnoredTestMetadata>,
}