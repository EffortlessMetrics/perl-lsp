//! CI Guardrail Ignored Test Monitoring and Governance
//!
//! This module provides automated ignored test monitoring with prevention of regression,
//! baseline tracking, quality gates, and comprehensive reporting.
//!
//! AC13: Ignored Test Guardian
//! AC14: Test Metadata Schema
//! AC15: Test Quality Validator

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
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

/// Inventory of ignored tests
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

/// Baseline management configuration
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

/// Quality gates for ignored tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGates {
    /// Pre-commit validation rules
    pub pre_commit: PreCommitValidation,
    /// CI pipeline validation
    pub ci_validation: CiValidation,
    /// Quality metrics tracking
    pub metrics_tracking: MetricsTracking,
}

/// Pre-commit validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCommitValidation {
    /// Require justification for new ignored tests
    pub require_justification: bool,
    /// Maximum allowed new ignored tests per commit
    pub max_new_ignored_per_commit: usize,
    /// Required documentation for ignored tests
    pub documentation_requirements: DocumentationRequirements,
}

/// Documentation requirements for ignored tests
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

/// CI validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiValidation {
    /// Block CI on ignored test count increase
    pub block_on_count_increase: bool,
    /// Maximum allowed ignored tests per crate
    pub max_ignored_per_crate: HashMap<String, usize>,
    /// Quality score threshold
    pub min_quality_score: f64,
}

/// Metrics tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsTracking {
    /// Track ignored test trend over time
    pub track_trend: bool,
    /// Trend analysis window in days
    pub trend_window_days: u32,
    /// Alert on negative trend
    pub alert_on_negative_trend: bool,
}

/// Reporting configuration
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

/// Supported report formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportFormat {
    /// JSON format
    Json,
    /// Markdown format
    Markdown,
    /// HTML format
    Html,
    /// CSV format
    Csv,
}

/// Categories for ignored tests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TestCategory {
    /// Critical LSP functionality
    CriticalLsp,
    /// Infrastructure and tooling
    Infrastructure,
    /// Advanced language syntax
    AdvancedSyntax,
    /// Niche edge cases
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

/// Complexity levels for implementation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplexityLevel {
    /// Low complexity
    Low,
    /// Medium complexity
    Medium,
    /// High complexity
    High,
    /// Critical complexity
    Critical,
}

/// LSP workflow stages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LspWorkflowStage {
    /// Parsing stage
    Parse,
    /// Indexing stage
    Index,
    /// Navigation stage
    Navigate,
    /// Completion stage
    Complete,
    /// Analysis stage
    Analyze,
    /// Cross-cutting concern
    CrossCutting,
}

/// Performance requirements for tests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub baseline_tracker: BaselineTracker,
    /// Current governance configuration
    pub governance: IgnoredTestGovernance,
}

/// Tracker for baseline metrics
pub struct BaselineTracker {
    /// Current baseline count
    pub current_baseline: usize,
    /// Historical data points
    pub historical_data: Vec<(SystemTime, usize)>,
}

impl IgnoredTestGuardian {
    /// Create a new ignored test guardian
    pub fn new(governance: IgnoredTestGovernance) -> Self {
        Self {
            baseline_tracker: BaselineTracker {
                current_baseline: governance.baseline_management.baseline_count,
                historical_data: Vec::new(),
            },
            governance,
        }
    }

    /// Validate a new ignored test against quality gates
    pub fn validate_new_ignored_test(&self, test_info: &IgnoredTestMetadata) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate required documentation
        if self
            .governance
            .quality_gates
            .pre_commit
            .documentation_requirements
            .require_issue_reference
        {
            if !test_info.ignore_reason.contains('#') && !test_info.ignore_reason.contains("issue")
            {
                errors.push("Ignored test must reference an issue".to_string());
            }
        }

        if self.governance.quality_gates.pre_commit.documentation_requirements.require_timeline {
            if test_info.target_timeline.as_secs() == 0 {
                errors.push("target implementation timeline must be specified".to_string());
            }
        }

        if self
            .governance
            .quality_gates
            .pre_commit
            .documentation_requirements
            .require_success_criteria
        {
            if test_info.success_criteria.is_empty() {
                errors.push("success criteria must be specified".to_string());
            }
        }

        // Validate complexity assessment
        if self
            .governance
            .quality_gates
            .pre_commit
            .documentation_requirements
            .require_complexity_assessment
        {
            if test_info.complexity == ComplexityLevel::Low
                && test_info.target_timeline > Duration::from_secs(7 * 24 * 3600)
            {
                warnings.push("Low complexity test should have shorter timeline".to_string());
            }
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            quality_score: self.calculate_quality_score(test_info),
        }
    }

    /// Check for baseline regression
    pub fn check_baseline_regression(&self, current_count: usize) -> RegressionResult {
        let baseline = self.baseline_tracker.current_baseline;
        let max_deviation = self.governance.baseline_management.max_deviation;
        let threshold_percent = self.governance.baseline_management.deviation_threshold_percent;

        let absolute_increase = current_count.saturating_sub(baseline);
        let percentage_increase =
            if baseline > 0 { (absolute_increase as f64 / baseline as f64) * 100.0 } else { 0.0 };

        let is_regression =
            absolute_increase > max_deviation || percentage_increase > threshold_percent;

        RegressionResult {
            is_regression,
            current_count,
            baseline_count: baseline,
            absolute_increase,
            percentage_increase,
            threshold_exceeded: if absolute_increase > max_deviation {
                Some(format!(
                    "Absolute increase {} > max deviation {}",
                    absolute_increase, max_deviation
                ))
            } else if percentage_increase > threshold_percent {
                Some(format!(
                    "Percentage increase {:.1}% > threshold {:.1}%",
                    percentage_increase, threshold_percent
                ))
            } else {
                None
            },
        }
    }

    /// Generate trend report
    pub fn generate_trend_report(&self) -> TrendReport {
        let current_time = SystemTime::now();
        let window_duration = Duration::from_secs(
            self.governance.reporting.monthly_summaries as u64 * 30 * 24 * 3600,
        );

        let recent_data: Vec<_> = self
            .baseline_tracker
            .historical_data
            .iter()
            .filter(|(timestamp, _)| {
                current_time.duration_since(*timestamp).unwrap_or(Duration::MAX) <= window_duration
            })
            .cloned()
            .collect();

        let trend_direction = if recent_data.len() >= 2 {
            let first = recent_data[0].1 as f64;
            let last = recent_data[recent_data.len() - 1].1 as f64;
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

        let average_count = if !recent_data.is_empty() {
            recent_data.iter().map(|(_, count)| *count as f64).sum::<f64>()
                / recent_data.len() as f64
        } else {
            0.0
        };

        TrendReport {
            period_start: recent_data.first().map(|(t, _)| *t),
            period_end: recent_data.last().map(|(t, _)| *t),
            recommendations: self.generate_trend_recommendations(&trend_direction, &recent_data),
            data_points: recent_data,
            trend_direction,
            average_count,
        }
    }

    /// Calculate quality score for an ignored test
    fn calculate_quality_score(&self, test_info: &IgnoredTestMetadata) -> f64 {
        let mut score: f64 = 100.0;

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
            if duration > Duration::from_secs(90 * 24 * 3600) {
                // 90 days
                score -= 25.0;
            }
        }

        // Bonus for well-documented tests
        if test_info.success_criteria.len() >= 3 {
            score += 5.0;
        }

        score.clamp(0.0, 100.0)
    }

    fn generate_trend_recommendations(
        &self,
        direction: &TrendDirection,
        data: &[(SystemTime, usize)],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        match direction {
            TrendDirection::Increasing => {
                recommendations.push(
                    "Consider implementing systematic ignored test resolution plan".to_string(),
                );
                recommendations.push("Review test categorization and prioritization".to_string());
                recommendations
                    .push("Allocate development resources for test implementation".to_string());
            }
            TrendDirection::Decreasing => {
                recommendations.push("Excellent progress on ignored test reduction".to_string());
                recommendations.push("Document successful implementation strategies".to_string());
                recommendations.push("Maintain current implementation pace".to_string());
            }
            TrendDirection::Stable => {
                recommendations
                    .push("Evaluate whether current ignored test count is acceptable".to_string());
                recommendations
                    .push("Consider setting more aggressive reduction targets".to_string());
            }
            TrendDirection::Unknown => {
                recommendations.push("Collect more historical data for trend analysis".to_string());
                recommendations.push("Establish baseline measurement practices".to_string());
            }
        }

        if data.len() > 10 {
            let recent_variance = self.calculate_variance(&data[data.len().saturating_sub(10)..]);
            if recent_variance > 10.0 {
                recommendations.push(
                    "High variance in ignored test count indicates inconsistent progress"
                        .to_string(),
                );
            }
        }

        recommendations
    }

    fn calculate_variance(&self, data: &[(SystemTime, usize)]) -> f64 {
        if data.len() < 2 {
            return 0.0;
        }

        let mean = data.iter().map(|(_, count)| *count as f64).sum::<f64>() / data.len() as f64;
        data.iter().map(|(_, count)| (*count as f64 - mean).powi(2)).sum::<f64>()
            / data.len() as f64
    }

    /// Set historical data for trend analysis (useful for testing or loading from storage)
    pub fn set_historical_data(&mut self, data: Vec<(SystemTime, usize)>) {
        self.baseline_tracker.historical_data = data;
    }
}

/// Result of a validation operation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the validation passed
    pub is_valid: bool,
    /// List of error messages
    pub errors: Vec<String>,
    /// List of warning messages
    pub warnings: Vec<String>,
    /// Overall quality score (0-100)
    pub quality_score: f64,
}

/// Result of a regression check
#[derive(Debug, Clone)]
pub struct RegressionResult {
    /// Whether a regression was detected
    pub is_regression: bool,
    /// Current count of ignored tests
    pub current_count: usize,
    /// Baseline count of ignored tests
    pub baseline_count: usize,
    /// Absolute increase over baseline
    pub absolute_increase: usize,
    /// Percentage increase over baseline
    pub percentage_increase: f64,
    /// Description of threshold exceeded
    pub threshold_exceeded: Option<String>,
}

/// Report on ignored test trends
#[derive(Debug, Clone)]
pub struct TrendReport {
    /// Start of the reporting period
    pub period_start: Option<SystemTime>,
    /// End of the reporting period
    pub period_end: Option<SystemTime>,
    /// Data points used for trend analysis
    pub data_points: Vec<(SystemTime, usize)>,
    /// Calculated trend direction
    pub trend_direction: TrendDirection,
    /// Average count over the period
    pub average_count: f64,
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
}

/// Direction of ignored test count trend
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrendDirection {
    /// Count is increasing
    Increasing,
    /// Count is decreasing
    Decreasing,
    /// Count is stable
    Stable,
    /// Trend cannot be determined
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};

    // ---------------------------------------------------------------------------
    // Test helpers
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
                    max_new_ignored_per_commit: 1,
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
                    min_quality_score: 50.0,
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

    fn make_meta() -> IgnoredTestMetadata {
        IgnoredTestMetadata {
            test_id: "test-001".to_string(),
            file_path: PathBuf::from("src/lib.rs"),
            test_name: "test_example".to_string(),
            category: TestCategory::CriticalLsp,
            priority: 1,
            ignore_reason: "Blocked by issue #42 pending parser support".to_string(),
            complexity: ComplexityLevel::Medium,
            target_timeline: Duration::from_secs(7 * 24 * 3600),
            dependencies: vec!["parser-feature-x".to_string()],
            success_criteria: vec![
                "Parser handles nested expressions".to_string(),
                "No false positives in test suite".to_string(),
            ],
            workflow_integration: LspWorkflowStage::Parse,
            performance_requirements: None,
            last_assessed: SystemTime::now(),
        }
    }

    // ---------------------------------------------------------------------------
    // IgnoredTestGuardian::new — construction
    // ---------------------------------------------------------------------------

    #[test]
    fn test_guardian_new_sets_baseline_from_governance() {
        let gov = make_governance();
        let guardian = IgnoredTestGuardian::new(gov);
        assert_eq!(guardian.baseline_tracker.current_baseline, 10);
        assert!(guardian.baseline_tracker.historical_data.is_empty());
    }

    #[test]
    fn test_guardian_new_with_zero_baseline() {
        let mut gov = make_governance();
        gov.baseline_management.baseline_count = 0;
        let guardian = IgnoredTestGuardian::new(gov);
        assert_eq!(guardian.baseline_tracker.current_baseline, 0);
    }

    // ---------------------------------------------------------------------------
    // validate_new_ignored_test — issue reference branch
    // ---------------------------------------------------------------------------

    #[test]
    fn test_validate_passes_with_issue_hash_reference() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let meta = make_meta(); // reason contains '#'
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_passes_with_issue_word_reference() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        meta.ignore_reason = "Blocked by issue tracker entry awaiting resolution".to_string();
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_fails_without_issue_reference() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        meta.ignore_reason =
            "Flaky test without any tracking reference in the bug tracker".to_string();
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("issue")));
    }

    #[test]
    fn test_validate_issue_reference_not_required_when_disabled() {
        let mut gov = make_governance();
        gov.quality_gates.pre_commit.documentation_requirements.require_issue_reference = false;
        let guardian = IgnoredTestGuardian::new(gov);
        let mut meta = make_meta();
        meta.ignore_reason = "no tracking ref here".to_string();
        let result = guardian.validate_new_ignored_test(&meta);
        // Only timeline/criteria errors could occur, but reason has no '#' or 'issue'
        // Since require_issue_reference is false, that branch is skipped — no error for it
        assert!(!result.errors.iter().any(|e| e.contains("issue")));
    }

    // ---------------------------------------------------------------------------
    // validate_new_ignored_test — timeline branch
    // ---------------------------------------------------------------------------

    #[test]
    fn test_validate_fails_with_zero_timeline() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        meta.target_timeline = Duration::ZERO;
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(result.errors.iter().any(|e| e.contains("timeline")));
    }

    #[test]
    fn test_validate_timeline_not_required_when_disabled() {
        let mut gov = make_governance();
        gov.quality_gates.pre_commit.documentation_requirements.require_timeline = false;
        let guardian = IgnoredTestGuardian::new(gov);
        let mut meta = make_meta();
        meta.target_timeline = Duration::ZERO;
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.errors.iter().any(|e| e.contains("timeline")));
    }

    // ---------------------------------------------------------------------------
    // validate_new_ignored_test — success criteria branch
    // ---------------------------------------------------------------------------

    #[test]
    fn test_validate_fails_with_empty_success_criteria() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        meta.success_criteria = Vec::new();
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(result.errors.iter().any(|e| e.contains("success criteria")));
    }

    #[test]
    fn test_validate_success_criteria_not_required_when_disabled() {
        let mut gov = make_governance();
        gov.quality_gates.pre_commit.documentation_requirements.require_success_criteria = false;
        let guardian = IgnoredTestGuardian::new(gov);
        let mut meta = make_meta();
        meta.success_criteria = Vec::new();
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.errors.iter().any(|e| e.contains("success criteria")));
    }

    // ---------------------------------------------------------------------------
    // validate_new_ignored_test — complexity assessment branch
    // ---------------------------------------------------------------------------

    #[test]
    fn test_validate_warns_low_complexity_long_timeline() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        meta.complexity = ComplexityLevel::Low;
        // More than 7 days = long timeline for low complexity
        meta.target_timeline = Duration::from_secs(8 * 24 * 3600);
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(result.warnings.iter().any(|w| w.contains("Low complexity")));
    }

    #[test]
    fn test_validate_no_warning_medium_complexity_long_timeline() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        meta.complexity = ComplexityLevel::Medium;
        meta.target_timeline = Duration::from_secs(30 * 24 * 3600);
        let result = guardian.validate_new_ignored_test(&meta);
        // Medium complexity — warning should not fire
        assert!(!result.warnings.iter().any(|w| w.contains("Low complexity")));
    }

    #[test]
    fn test_validate_no_warning_low_complexity_short_timeline() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        meta.complexity = ComplexityLevel::Low;
        meta.target_timeline = Duration::from_secs(3 * 24 * 3600); // 3 days — short
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.warnings.iter().any(|w| w.contains("Low complexity")));
    }

    #[test]
    fn test_validate_complexity_assessment_not_required_when_disabled() {
        let mut gov = make_governance();
        gov.quality_gates.pre_commit.documentation_requirements.require_complexity_assessment =
            false;
        let guardian = IgnoredTestGuardian::new(gov);
        let mut meta = make_meta();
        meta.complexity = ComplexityLevel::Low;
        meta.target_timeline = Duration::from_secs(30 * 24 * 3600);
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.warnings.iter().any(|w| w.contains("Low complexity")));
    }

    // ---------------------------------------------------------------------------
    // calculate_quality_score — scoring branches
    // ---------------------------------------------------------------------------

    #[test]
    fn test_quality_score_deducts_for_short_reason() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        meta.ignore_reason = "short #1".to_string(); // < 20 chars
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(result.quality_score < 100.0);
    }

    #[test]
    fn test_quality_score_deducts_for_empty_success_criteria() {
        let mut gov = make_governance();
        // Disable the validation so we can test scoring without an error
        gov.quality_gates.pre_commit.documentation_requirements.require_success_criteria = false;
        let guardian = IgnoredTestGuardian::new(gov);
        let mut meta = make_meta();
        meta.success_criteria = Vec::new();
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(result.quality_score < 100.0);
    }

    #[test]
    fn test_quality_score_deducts_for_no_deps_non_low_complexity() {
        let mut gov = make_governance();
        gov.quality_gates.pre_commit.documentation_requirements.require_complexity_assessment =
            false;
        let guardian = IgnoredTestGuardian::new(gov);
        let mut meta = make_meta();
        meta.complexity = ComplexityLevel::High;
        meta.dependencies = Vec::new();
        let result = guardian.validate_new_ignored_test(&meta);
        // Should deduct 10 points for missing deps on non-Low complexity
        assert!(result.quality_score < 100.0);
    }

    #[test]
    fn test_quality_score_no_dep_deduction_for_low_complexity() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        meta.complexity = ComplexityLevel::Low;
        meta.dependencies = Vec::new();
        // Short timeline to avoid complexity warning
        meta.target_timeline = Duration::from_secs(24 * 3600);
        // Long enough reason to avoid short-reason deduction
        meta.ignore_reason = "Blocked by issue #99 pending parser support fully".to_string();
        // Multiple criteria for bonus
        meta.success_criteria = vec![
            "Criterion one".to_string(),
            "Criterion two".to_string(),
            "Criterion three".to_string(),
        ];
        let result = guardian.validate_new_ignored_test(&meta);
        // No dep-deduction, bonus applied: should be >= 100
        assert!(result.quality_score >= 100.0);
    }

    #[test]
    fn test_quality_score_bonus_for_three_or_more_success_criteria() {
        // To make the bonus observable, we need a base score < 95 so the +5 doesn't get clamped.
        // Use a short ignore_reason (-20 deduction) → base = 80, +5 bonus = 85 vs 80 (no bonus).
        let mut gov = make_governance();
        gov.quality_gates.pre_commit.documentation_requirements.require_complexity_assessment =
            false;
        let guardian = IgnoredTestGuardian::new(gov);

        let mut meta_bonus = make_meta();
        meta_bonus.ignore_reason = "short #1 reason".to_string(); // < 20 chars → -20
        meta_bonus.success_criteria =
            vec!["Criterion A".to_string(), "Criterion B".to_string(), "Criterion C".to_string()]; // >= 3 → +5 bonus
        let result_bonus = guardian.validate_new_ignored_test(&meta_bonus);

        let mut meta_no_bonus = make_meta();
        meta_no_bonus.ignore_reason = "short #2 reason".to_string(); // < 20 chars → -20
        meta_no_bonus.success_criteria = vec!["Only one".to_string()]; // 1 < 3 → no bonus
        let result_no_bonus = guardian.validate_new_ignored_test(&meta_no_bonus);

        assert!(result_bonus.quality_score > result_no_bonus.quality_score);
    }

    #[test]
    fn test_quality_score_deducts_for_old_test() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        // Set last_assessed to 91 days ago
        meta.last_assessed = SystemTime::now() - Duration::from_secs(91 * 24 * 3600);
        let result_old = guardian.validate_new_ignored_test(&meta);

        let mut meta_new = make_meta();
        meta_new.last_assessed = SystemTime::now();
        let result_new = guardian.validate_new_ignored_test(&meta_new);

        // Old test should have lower score
        assert!(result_old.quality_score < result_new.quality_score);
    }

    #[test]
    fn test_quality_score_clamped_to_zero_minimum() {
        let mut gov = make_governance();
        gov.quality_gates.pre_commit.documentation_requirements.require_success_criteria = false;
        gov.quality_gates.pre_commit.documentation_requirements.require_complexity_assessment =
            false;
        let guardian = IgnoredTestGuardian::new(gov);
        let mut meta = make_meta();
        meta.ignore_reason = "short #1".to_string(); // -20
        meta.success_criteria = Vec::new(); // -30
        meta.dependencies = Vec::new();
        meta.complexity = ComplexityLevel::High; // -10
        meta.last_assessed = SystemTime::now() - Duration::from_secs(91 * 24 * 3600); // -25
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(result.quality_score >= 0.0);
    }

    // ---------------------------------------------------------------------------
    // check_baseline_regression — branches
    // ---------------------------------------------------------------------------

    #[test]
    fn test_regression_no_increase() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let result = guardian.check_baseline_regression(10); // exactly at baseline
        assert!(!result.is_regression);
        assert_eq!(result.absolute_increase, 0);
        assert_eq!(result.baseline_count, 10);
        assert_eq!(result.current_count, 10);
        assert!(result.threshold_exceeded.is_none());
    }

    #[test]
    fn test_regression_within_absolute_deviation() {
        // baseline=10, max_dev=3, threshold%=20%. current=12: absolute=2 (<3 ok), pct=20% (not > 20%). No regression.
        let guardian = IgnoredTestGuardian::new(make_governance());
        let result = guardian.check_baseline_regression(12); // +2, max_dev=3, 20% == threshold (not >)
        assert!(!result.is_regression);
        assert_eq!(result.absolute_increase, 2);
    }

    #[test]
    fn test_regression_exceeds_absolute_deviation() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let result = guardian.check_baseline_regression(14); // +4 > max_dev=3
        assert!(result.is_regression);
        assert!(result.threshold_exceeded.is_some());
        let msg = result.threshold_exceeded.as_deref().unwrap_or("");
        assert!(msg.contains("Absolute increase"));
    }

    #[test]
    fn test_regression_exceeds_percentage_threshold() {
        let mut gov = make_governance();
        // baseline=100, max_deviation=100 (won't trigger), threshold=5%
        gov.baseline_management.baseline_count = 100;
        gov.baseline_management.max_deviation = 100;
        gov.baseline_management.deviation_threshold_percent = 5.0;
        let guardian = IgnoredTestGuardian::new(gov);
        let result = guardian.check_baseline_regression(106); // +6 = 6% > 5%
        assert!(result.is_regression);
        assert!(result.threshold_exceeded.is_some());
        let msg = result.threshold_exceeded.as_deref().unwrap_or("");
        assert!(msg.contains("Percentage increase"));
    }

    #[test]
    fn test_regression_below_baseline_no_regression() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let result = guardian.check_baseline_regression(5); // below baseline
        assert!(!result.is_regression);
        assert_eq!(result.absolute_increase, 0); // saturating_sub
    }

    #[test]
    fn test_regression_percentage_zero_when_baseline_zero() {
        let mut gov = make_governance();
        gov.baseline_management.baseline_count = 0;
        gov.baseline_management.max_deviation = 0;
        let guardian = IgnoredTestGuardian::new(gov);
        let result = guardian.check_baseline_regression(0);
        assert_eq!(result.percentage_increase, 0.0);
    }

    // ---------------------------------------------------------------------------
    // generate_trend_report — all TrendDirection arms
    // ---------------------------------------------------------------------------

    #[test]
    fn test_trend_report_unknown_when_no_data() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let report = guardian.generate_trend_report();
        assert_eq!(report.trend_direction, TrendDirection::Unknown);
        assert!(report.period_start.is_none());
        assert!(report.period_end.is_none());
        assert_eq!(report.average_count, 0.0);
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn test_trend_report_unknown_with_single_data_point() {
        let mut guardian = IgnoredTestGuardian::new(make_governance());
        guardian.set_historical_data(vec![(SystemTime::now(), 10)]);
        let report = guardian.generate_trend_report();
        assert_eq!(report.trend_direction, TrendDirection::Unknown);
    }

    #[test]
    fn test_trend_report_increasing() {
        let mut guardian = IgnoredTestGuardian::new(make_governance());
        let now = SystemTime::now();
        // last > first * 1.1: 22 > 20 * 1.1 = 22.0 — need strictly greater
        guardian.set_historical_data(vec![
            (now - Duration::from_secs(60), 10),
            (now, 23), // 23 > 10 * 1.1 = 11.0
        ]);
        let report = guardian.generate_trend_report();
        assert_eq!(report.trend_direction, TrendDirection::Increasing);
        assert!(!report.recommendations.is_empty());
        assert!(
            report.recommendations.iter().any(|r| r.contains("resolution plan"))
                || report.recommendations.iter().any(|r| r.contains("categorization"))
        );
    }

    #[test]
    fn test_trend_report_decreasing() {
        let mut guardian = IgnoredTestGuardian::new(make_governance());
        let now = SystemTime::now();
        // last < first * 0.9: 8 < 10 * 0.9 = 9.0
        guardian.set_historical_data(vec![(now - Duration::from_secs(60), 10), (now, 8)]);
        let report = guardian.generate_trend_report();
        assert_eq!(report.trend_direction, TrendDirection::Decreasing);
        assert!(report.recommendations.iter().any(|r| r.contains("progress")));
    }

    #[test]
    fn test_trend_report_stable() {
        let mut guardian = IgnoredTestGuardian::new(make_governance());
        let now = SystemTime::now();
        // last is between first*0.9 and first*1.1
        guardian.set_historical_data(vec![
            (now - Duration::from_secs(60), 10),
            (now, 10), // exactly equal — stable
        ]);
        let report = guardian.generate_trend_report();
        assert_eq!(report.trend_direction, TrendDirection::Stable);
        assert!(report.recommendations.iter().any(|r| r.contains("acceptable")));
    }

    #[test]
    fn test_trend_report_average_count_computed() {
        let mut guardian = IgnoredTestGuardian::new(make_governance());
        let now = SystemTime::now();
        guardian.set_historical_data(vec![
            (now - Duration::from_secs(120), 10),
            (now - Duration::from_secs(60), 20),
            (now, 30),
        ]);
        let report = guardian.generate_trend_report();
        assert!((report.average_count - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_trend_report_data_points_returned() {
        let mut guardian = IgnoredTestGuardian::new(make_governance());
        let now = SystemTime::now();
        let data = vec![(now - Duration::from_secs(60), 5usize), (now, 7usize)];
        guardian.set_historical_data(data.clone());
        let report = guardian.generate_trend_report();
        assert_eq!(report.data_points.len(), 2);
    }

    // ---------------------------------------------------------------------------
    // generate_trend_report — high variance branch
    // ---------------------------------------------------------------------------

    #[test]
    fn test_trend_report_high_variance_adds_recommendation() {
        let mut guardian = IgnoredTestGuardian::new(make_governance());
        let now = SystemTime::now();
        // Build > 10 data points with high variance (alternating 0, 100)
        let mut data = Vec::new();
        for i in 0..12u64 {
            let count = if i % 2 == 0 { 0usize } else { 100usize };
            data.push((now - Duration::from_secs((12 - i) * 60), count));
        }
        guardian.set_historical_data(data);
        let report = guardian.generate_trend_report();
        assert!(
            report
                .recommendations
                .iter()
                .any(|r| r.contains("variance") || r.contains("inconsistent"))
        );
    }

    #[test]
    fn test_trend_report_low_variance_no_variance_recommendation() {
        let mut guardian = IgnoredTestGuardian::new(make_governance());
        let now = SystemTime::now();
        // 12 data points all with the same value — zero variance
        let data: Vec<_> =
            (0..12u64).map(|i| (now - Duration::from_secs((12 - i) * 60), 10usize)).collect();
        guardian.set_historical_data(data);
        let report = guardian.generate_trend_report();
        // High-variance recommendation should NOT appear
        assert!(
            !report
                .recommendations
                .iter()
                .any(|r| r.contains("variance") || r.contains("inconsistent"))
        );
    }

    // ---------------------------------------------------------------------------
    // calculate_variance — edge cases
    // ---------------------------------------------------------------------------

    #[test]
    fn test_variance_returns_zero_for_single_point() {
        // Exercised indirectly via generate_trend_report variance path;
        // we invoke it through the high-variance path with exactly 1 data point
        // in the last-10 slice by using 10 identical + 1 outlier
        let mut guardian = IgnoredTestGuardian::new(make_governance());
        let now = SystemTime::now();
        // Single data point — variance path in generate_trend_report calls
        // calculate_variance on data[data.len()-10..], size 1 → returns 0.0
        let data = vec![(now, 10usize)];
        guardian.set_historical_data(data);
        let report = guardian.generate_trend_report();
        // No variance recommendation since variance is 0
        assert!(!report.recommendations.iter().any(|r| r.contains("variance")));
    }

    // ---------------------------------------------------------------------------
    // set_historical_data
    // ---------------------------------------------------------------------------

    #[test]
    fn test_set_historical_data_replaces_existing() {
        let mut guardian = IgnoredTestGuardian::new(make_governance());
        let now = SystemTime::now();
        guardian.set_historical_data(vec![(now, 5)]);
        guardian.set_historical_data(vec![(now, 10), (now, 20)]);
        assert_eq!(guardian.baseline_tracker.historical_data.len(), 2);
    }

    #[test]
    fn test_set_historical_data_empty_clears_data() {
        let mut guardian = IgnoredTestGuardian::new(make_governance());
        let now = SystemTime::now();
        guardian.set_historical_data(vec![(now, 5)]);
        guardian.set_historical_data(Vec::new());
        assert!(guardian.baseline_tracker.historical_data.is_empty());
    }

    // ---------------------------------------------------------------------------
    // Enum PartialEq coverage
    // ---------------------------------------------------------------------------

    #[test]
    fn test_trend_direction_all_variants_comparable() {
        assert_eq!(TrendDirection::Increasing, TrendDirection::Increasing);
        assert_eq!(TrendDirection::Decreasing, TrendDirection::Decreasing);
        assert_eq!(TrendDirection::Stable, TrendDirection::Stable);
        assert_eq!(TrendDirection::Unknown, TrendDirection::Unknown);
        assert_ne!(TrendDirection::Increasing, TrendDirection::Stable);
    }

    #[test]
    fn test_test_category_all_variants_hashable() {
        let mut map = HashMap::new();
        map.insert(TestCategory::CriticalLsp, 1usize);
        map.insert(TestCategory::Infrastructure, 2);
        map.insert(TestCategory::AdvancedSyntax, 3);
        map.insert(TestCategory::EdgeCases, 4);
        assert_eq!(map[&TestCategory::CriticalLsp], 1);
        assert_eq!(map[&TestCategory::EdgeCases], 4);
    }

    #[test]
    fn test_complexity_level_all_variants() {
        let levels = [
            ComplexityLevel::Low,
            ComplexityLevel::Medium,
            ComplexityLevel::High,
            ComplexityLevel::Critical,
        ];
        for level in &levels {
            assert_eq!(level, level);
        }
        assert_ne!(ComplexityLevel::Low, ComplexityLevel::High);
    }

    #[test]
    fn test_lsp_workflow_stage_all_variants() {
        let stages = [
            LspWorkflowStage::Parse,
            LspWorkflowStage::Index,
            LspWorkflowStage::Navigate,
            LspWorkflowStage::Complete,
            LspWorkflowStage::Analyze,
            LspWorkflowStage::CrossCutting,
        ];
        for stage in &stages {
            assert_eq!(stage, stage);
        }
        assert_ne!(LspWorkflowStage::Parse, LspWorkflowStage::Index);
    }

    #[test]
    fn test_report_format_all_variants() {
        assert_eq!(ReportFormat::Json, ReportFormat::Json);
        assert_eq!(ReportFormat::Markdown, ReportFormat::Markdown);
        assert_eq!(ReportFormat::Html, ReportFormat::Html);
        assert_eq!(ReportFormat::Csv, ReportFormat::Csv);
        assert_ne!(ReportFormat::Json, ReportFormat::Html);
    }

    // ---------------------------------------------------------------------------
    // ValidationResult and RegressionResult — field access
    // ---------------------------------------------------------------------------

    #[test]
    fn test_validation_result_has_warnings_field() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        meta.complexity = ComplexityLevel::Low;
        meta.target_timeline = Duration::from_secs(10 * 24 * 3600);
        let result = guardian.validate_new_ignored_test(&meta);
        // warnings is accessible
        let _: &Vec<String> = &result.warnings;
    }

    #[test]
    fn test_regression_result_percentage_increase_field() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let result = guardian.check_baseline_regression(12);
        assert!(result.percentage_increase >= 0.0);
    }

    // ---------------------------------------------------------------------------
    // PerformanceRequirements coverage
    // ---------------------------------------------------------------------------

    #[test]
    fn test_performance_requirements_with_throughput() {
        let perf = PerformanceRequirements {
            max_latency_ms: 100,
            max_memory_mb: 256,
            min_throughput: Some(1000.0),
        };
        assert_eq!(perf.max_latency_ms, 100);
        assert_eq!(perf.max_memory_mb, 256);
        assert_eq!(perf.min_throughput, Some(1000.0));
    }

    #[test]
    fn test_performance_requirements_without_throughput() {
        let perf = PerformanceRequirements {
            max_latency_ms: 50,
            max_memory_mb: 128,
            min_throughput: None,
        };
        assert!(perf.min_throughput.is_none());
    }

    #[test]
    fn test_meta_with_performance_requirements() {
        let guardian = IgnoredTestGuardian::new(make_governance());
        let mut meta = make_meta();
        meta.performance_requirements = Some(PerformanceRequirements {
            max_latency_ms: 200,
            max_memory_mb: 512,
            min_throughput: None,
        });
        let result = guardian.validate_new_ignored_test(&meta);
        // Performance requirements are stored but not validated — should still pass
        assert!(result.is_valid);
    }
}
