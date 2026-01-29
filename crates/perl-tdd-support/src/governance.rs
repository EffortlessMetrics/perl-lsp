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

        score.max(0.0).min(100.0)
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
                recommendations.push(
                    "Evaluate whether current ignored test count is acceptable".to_string(),
                );
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
        let variance = data.iter().map(|(_, count)| (*count as f64 - mean).powi(2)).sum::<f64>()
            / data.len() as f64;

        variance
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