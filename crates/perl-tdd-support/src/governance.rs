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
                && test_info.target_timeline > Duration::from_hours(168)
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
            if duration > Duration::from_hours(2160) {
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

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    fn minimal_governance(
        baseline_count: usize,
        max_deviation: usize,
        deviation_threshold_percent: f64,
        require_issue_reference: bool,
        require_timeline: bool,
        require_success_criteria: bool,
        require_complexity_assessment: bool,
    ) -> IgnoredTestGovernance {
        IgnoredTestGovernance {
            inventory: IgnoredTestInventory {
                total_count: 0,
                by_category: HashMap::new(),
                by_crate: HashMap::new(),
                by_priority: HashMap::new(),
                last_updated: SystemTime::now(),
            },
            baseline_management: BaselineManagement {
                baseline_count,
                max_deviation,
                deviation_threshold_percent,
                baseline_date: SystemTime::now(),
                next_review_date: SystemTime::now(),
            },
            quality_gates: QualityGates {
                pre_commit: PreCommitValidation {
                    require_justification: false,
                    max_new_ignored_per_commit: 1,
                    documentation_requirements: DocumentationRequirements {
                        require_issue_reference,
                        require_timeline,
                        require_success_criteria,
                        require_complexity_assessment,
                    },
                },
                ci_validation: CiValidation {
                    block_on_count_increase: false,
                    max_ignored_per_crate: HashMap::new(),
                    min_quality_score: 0.0,
                },
                metrics_tracking: MetricsTracking {
                    track_trend: false,
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

    fn minimal_metadata(
        ignore_reason: &str,
        target_timeline_secs: u64,
        success_criteria: Vec<String>,
        complexity: ComplexityLevel,
        last_assessed: SystemTime,
    ) -> IgnoredTestMetadata {
        IgnoredTestMetadata {
            test_id: "test::example".to_string(),
            file_path: PathBuf::from("src/lib.rs"),
            test_name: "test_example".to_string(),
            category: TestCategory::EdgeCases,
            priority: 2,
            ignore_reason: ignore_reason.to_string(),
            complexity,
            target_timeline: Duration::from_secs(target_timeline_secs),
            dependencies: vec![],
            success_criteria,
            workflow_integration: LspWorkflowStage::Parse,
            performance_requirements: None,
            last_assessed,
        }
    }

    // ---------------------------------------------------------------------------
    // check_baseline_regression
    // ---------------------------------------------------------------------------

    #[test]
    fn test_regression_at_baseline_is_not_regression() -> Result<(), Box<dyn std::error::Error>> {
        let gov = minimal_governance(10, 2, 20.0, false, false, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let result = guardian.check_baseline_regression(10);
        assert!(!result.is_regression);
        assert_eq!(result.absolute_increase, 0);
        assert_eq!(result.baseline_count, 10);
        assert!(result.threshold_exceeded.is_none());
        Ok(())
    }

    #[test]
    fn test_regression_below_baseline_is_not_regression() -> Result<(), Box<dyn std::error::Error>>
    {
        let gov = minimal_governance(10, 2, 20.0, false, false, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let result = guardian.check_baseline_regression(5);
        assert!(!result.is_regression);
        assert_eq!(result.absolute_increase, 0); // saturating_sub
        assert!(result.threshold_exceeded.is_none());
        Ok(())
    }

    #[test]
    fn test_regression_absolute_threshold_exceeded() -> Result<(), Box<dyn std::error::Error>> {
        // baseline=10, max_deviation=2 — adding 3 triggers absolute regression
        let gov = minimal_governance(10, 2, 50.0, false, false, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let result = guardian.check_baseline_regression(13);
        assert!(result.is_regression);
        assert_eq!(result.absolute_increase, 3);
        let msg = result.threshold_exceeded.as_deref().unwrap_or("");
        assert!(msg.contains("Absolute increase"), "expected absolute message, got: {msg}");
        Ok(())
    }

    #[test]
    fn test_regression_percentage_threshold_exceeded() -> Result<(), Box<dyn std::error::Error>> {
        // baseline=100, max_deviation=50 (won't trip), deviation_threshold=10%
        // adding 15 → 15 % > 10 %
        let gov = minimal_governance(100, 50, 10.0, false, false, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let result = guardian.check_baseline_regression(115);
        assert!(result.is_regression);
        let msg = result.threshold_exceeded.as_deref().unwrap_or("");
        assert!(msg.contains("Percentage increase"), "expected percentage message, got: {msg}");
        Ok(())
    }

    #[test]
    fn test_regression_zero_baseline_no_percentage_increase()
    -> Result<(), Box<dyn std::error::Error>> {
        // baseline=0 → percentage_increase stays 0.0 even when count grows
        let gov = minimal_governance(0, 0, 5.0, false, false, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        // absolute_increase=5 > max_deviation=0 so is_regression is true,
        // but percentage_increase must be 0.0 (not NaN / Inf)
        let result = guardian.check_baseline_regression(5);
        assert_eq!(result.percentage_increase, 0.0);
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // validate_new_ignored_test — issue reference gate
    // ---------------------------------------------------------------------------

    #[test]
    fn test_validate_issue_reference_required_missing() -> Result<(), Box<dyn std::error::Error>> {
        let gov = minimal_governance(0, 5, 20.0, true, false, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let meta = minimal_metadata(
            "no reference here",
            3600,
            vec!["criterion".to_string()],
            ComplexityLevel::Low,
            SystemTime::now(),
        );
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("issue")));
        Ok(())
    }

    #[test]
    fn test_validate_issue_reference_satisfied_by_hash() -> Result<(), Box<dyn std::error::Error>> {
        let gov = minimal_governance(0, 5, 20.0, true, false, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let meta = minimal_metadata(
            "Blocked by #1234",
            3600,
            vec!["criterion".to_string()],
            ComplexityLevel::Low,
            SystemTime::now(),
        );
        let result = guardian.validate_new_ignored_test(&meta);
        // no issue-reference error expected
        assert!(!result.errors.iter().any(|e| e.contains("issue")));
        Ok(())
    }

    #[test]
    fn test_validate_issue_reference_satisfied_by_word_issue()
    -> Result<(), Box<dyn std::error::Error>> {
        let gov = minimal_governance(0, 5, 20.0, true, false, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let meta = minimal_metadata(
            "See issue tracker for details",
            3600,
            vec!["criterion".to_string()],
            ComplexityLevel::Low,
            SystemTime::now(),
        );
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.errors.iter().any(|e| e.contains("issue")));
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // validate_new_ignored_test — timeline gate
    // ---------------------------------------------------------------------------

    #[test]
    fn test_validate_timeline_required_zero_secs_fails() -> Result<(), Box<dyn std::error::Error>> {
        let gov = minimal_governance(0, 5, 20.0, false, true, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let meta = minimal_metadata(
            "short",
            0,
            vec!["ok".to_string()],
            ComplexityLevel::Low,
            SystemTime::now(),
        );
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("timeline")));
        Ok(())
    }

    #[test]
    fn test_validate_timeline_required_nonzero_passes() -> Result<(), Box<dyn std::error::Error>> {
        let gov = minimal_governance(0, 5, 20.0, false, true, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let meta = minimal_metadata(
            "short",
            3600,
            vec!["ok".to_string()],
            ComplexityLevel::Low,
            SystemTime::now(),
        );
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.errors.iter().any(|e| e.contains("timeline")));
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // validate_new_ignored_test — success criteria gate
    // ---------------------------------------------------------------------------

    #[test]
    fn test_validate_success_criteria_required_empty_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let gov = minimal_governance(0, 5, 20.0, false, false, true, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let meta = minimal_metadata(
            "a long enough reason here",
            3600,
            vec![],
            ComplexityLevel::Low,
            SystemTime::now(),
        );
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("success criteria")));
        Ok(())
    }

    #[test]
    fn test_validate_success_criteria_required_provided_passes()
    -> Result<(), Box<dyn std::error::Error>> {
        let gov = minimal_governance(0, 5, 20.0, false, false, true, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let meta = minimal_metadata(
            "a long enough reason here",
            3600,
            vec!["parses correctly".to_string()],
            ComplexityLevel::Low,
            SystemTime::now(),
        );
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.errors.iter().any(|e| e.contains("success criteria")));
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // validate_new_ignored_test — complexity assessment warning
    // ---------------------------------------------------------------------------

    #[test]
    fn test_validate_complexity_low_long_timeline_warns() -> Result<(), Box<dyn std::error::Error>>
    {
        let gov = minimal_governance(0, 5, 20.0, false, false, false, true);
        let guardian = IgnoredTestGuardian::new(gov);
        // 8 days > 7-day threshold for Low complexity
        let eight_days = 8 * 24 * 3600;
        let meta = minimal_metadata(
            "a reasonable explanation",
            eight_days,
            vec!["criterion".to_string()],
            ComplexityLevel::Low,
            SystemTime::now(),
        );
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(result.warnings.iter().any(|w| w.contains("shorter timeline")));
        Ok(())
    }

    #[test]
    fn test_validate_complexity_high_long_timeline_no_warning()
    -> Result<(), Box<dyn std::error::Error>> {
        // High complexity: even a long timeline should NOT trigger the low-complexity warning
        let gov = minimal_governance(0, 5, 20.0, false, false, false, true);
        let guardian = IgnoredTestGuardian::new(gov);
        let eight_days = 8 * 24 * 3600;
        let meta = minimal_metadata(
            "a reasonable explanation",
            eight_days,
            vec!["criterion".to_string()],
            ComplexityLevel::High,
            SystemTime::now(),
        );
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(!result.warnings.iter().any(|w| w.contains("shorter timeline")));
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // quality score
    // ---------------------------------------------------------------------------

    #[test]
    fn test_quality_score_clamped_to_100() -> Result<(), Box<dyn std::error::Error>> {
        // Perfect metadata: long reason, ≥3 success criteria, recent assessment
        let gov = minimal_governance(0, 5, 20.0, false, false, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        let meta = minimal_metadata(
            "This is a detailed ignore reason that is long enough",
            3600,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            ComplexityLevel::Low,
            SystemTime::now(),
        );
        let result = guardian.validate_new_ignored_test(&meta);
        assert!(result.quality_score <= 100.0);
        assert!(result.quality_score >= 0.0);
        Ok(())
    }

    #[test]
    fn test_quality_score_reduced_for_short_reason() -> Result<(), Box<dyn std::error::Error>> {
        let gov = minimal_governance(0, 5, 20.0, false, false, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        // Short reason (<20 chars) and no success criteria → score deductions
        let meta = minimal_metadata("short", 3600, vec![], ComplexityLevel::Low, SystemTime::now());
        let result = guardian.validate_new_ignored_test(&meta);
        // 100 - 20 (short reason) - 30 (no success criteria) = 50
        assert!(result.quality_score < 70.0);
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // trend report — direction classification
    // ---------------------------------------------------------------------------

    fn make_history(counts: &[usize]) -> Vec<(SystemTime, usize)> {
        // counts[0] is oldest, counts[last] is most recent.
        // Timestamps: oldest = len*60 seconds ago, most recent = 60 seconds ago.
        // All within the reporting window (< 30 days).
        let n = counts.len();
        counts
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let ago = Duration::from_secs((n - i) as u64 * 60);
                (SystemTime::now() - ago, c)
            })
            .collect()
    }

    fn guardian_with_monthly(monthly: bool) -> IgnoredTestGuardian {
        let mut gov = minimal_governance(10, 5, 20.0, false, false, false, false);
        gov.reporting.monthly_summaries = monthly;
        IgnoredTestGuardian::new(gov)
    }

    #[test]
    fn test_trend_report_unknown_with_no_data() -> Result<(), Box<dyn std::error::Error>> {
        let guardian = guardian_with_monthly(true);
        let report = guardian.generate_trend_report();
        assert_eq!(report.trend_direction, TrendDirection::Unknown);
        assert_eq!(report.average_count, 0.0);
        assert!(report.period_start.is_none());
        assert!(report.period_end.is_none());
        Ok(())
    }

    #[test]
    fn test_trend_report_unknown_with_one_data_point() -> Result<(), Box<dyn std::error::Error>> {
        let mut guardian = guardian_with_monthly(true);
        guardian.set_historical_data(make_history(&[5]));
        let report = guardian.generate_trend_report();
        assert_eq!(report.trend_direction, TrendDirection::Unknown);
        Ok(())
    }

    #[test]
    fn test_trend_report_increasing_direction() -> Result<(), Box<dyn std::error::Error>> {
        let mut guardian = guardian_with_monthly(true);
        // last is > first * 1.1
        guardian.set_historical_data(make_history(&[10, 15, 20]));
        let report = guardian.generate_trend_report();
        assert_eq!(report.trend_direction, TrendDirection::Increasing);
        assert!(report.average_count > 0.0);
        Ok(())
    }

    #[test]
    fn test_trend_report_decreasing_direction() -> Result<(), Box<dyn std::error::Error>> {
        let mut guardian = guardian_with_monthly(true);
        // last is < first * 0.9
        guardian.set_historical_data(make_history(&[20, 15, 10]));
        let report = guardian.generate_trend_report();
        assert_eq!(report.trend_direction, TrendDirection::Decreasing);
        Ok(())
    }

    #[test]
    fn test_trend_report_stable_direction() -> Result<(), Box<dyn std::error::Error>> {
        let mut guardian = guardian_with_monthly(true);
        // last == first → within ±10 %
        guardian.set_historical_data(make_history(&[10, 10, 10]));
        let report = guardian.generate_trend_report();
        assert_eq!(report.trend_direction, TrendDirection::Stable);
        Ok(())
    }

    #[test]
    fn test_trend_report_recommendations_not_empty() -> Result<(), Box<dyn std::error::Error>> {
        let mut guardian = guardian_with_monthly(true);
        guardian.set_historical_data(make_history(&[10, 20, 30]));
        let report = guardian.generate_trend_report();
        assert!(!report.recommendations.is_empty());
        Ok(())
    }

    #[test]
    fn test_trend_report_window_excludes_old_data_when_monthly_disabled()
    -> Result<(), Box<dyn std::error::Error>> {
        // monthly_summaries=false → window = 0 seconds → all historical data is excluded
        let mut guardian = guardian_with_monthly(false);
        guardian.set_historical_data(make_history(&[10, 20]));
        let report = guardian.generate_trend_report();
        // All data is outside the 0-second window
        assert_eq!(report.trend_direction, TrendDirection::Unknown);
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // IgnoredTestGuardian::new — baseline initialised from governance
    // ---------------------------------------------------------------------------

    #[test]
    fn test_new_guardian_initialises_baseline_from_governance()
    -> Result<(), Box<dyn std::error::Error>> {
        let gov = minimal_governance(42, 5, 10.0, false, false, false, false);
        let guardian = IgnoredTestGuardian::new(gov);
        assert_eq!(guardian.baseline_tracker.current_baseline, 42);
        assert!(guardian.baseline_tracker.historical_data.is_empty());
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // set_historical_data round-trip
    // ---------------------------------------------------------------------------

    #[test]
    fn test_set_historical_data_replaces_previous() -> Result<(), Box<dyn std::error::Error>> {
        let gov = minimal_governance(0, 5, 10.0, false, false, false, false);
        let mut guardian = IgnoredTestGuardian::new(gov);
        guardian.set_historical_data(vec![(SystemTime::now(), 1)]);
        assert_eq!(guardian.baseline_tracker.historical_data.len(), 1);
        guardian.set_historical_data(vec![]);
        assert!(guardian.baseline_tracker.historical_data.is_empty());
        Ok(())
    }
}
