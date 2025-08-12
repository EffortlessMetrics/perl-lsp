//! Integrated edge case handler for Perl heredocs
//!
//! This module combines phase-aware parsing, dynamic delimiter recovery,
//! and other edge case detection systems into a unified interface.

use crate::anti_pattern_detector::{AntiPatternDetector, Diagnostic};
use crate::dynamic_delimiter_recovery::{DynamicDelimiterRecovery, ParseContext, RecoveryMode};
use crate::partial_parse_ast::ExtendedAstNode;
use crate::phase_aware_parser::{PerlPhase, PhaseAwareParser};
use crate::understanding_parser::UnderstandingParser;

pub struct EdgeCaseHandler {
    anti_pattern_detector: AntiPatternDetector,
    phase_parser: PhaseAwareParser,
    delimiter_recovery: DynamicDelimiterRecovery,
    config: EdgeCaseConfig,
}

#[derive(Debug, Clone)]
pub struct EdgeCaseConfig {
    pub recovery_mode: RecoveryMode,
    pub enable_sandbox: bool,
    pub interactive_mode: bool,
    pub strict_mode: bool,
}

impl Default for EdgeCaseConfig {
    fn default() -> Self {
        Self {
            recovery_mode: RecoveryMode::BestGuess,
            enable_sandbox: false,
            interactive_mode: false,
            strict_mode: false,
        }
    }
}

#[derive(Debug)]
pub struct EdgeCaseAnalysis {
    pub ast: ExtendedAstNode,
    pub diagnostics: Vec<Diagnostic>,
    pub phase_warnings: Vec<String>,
    pub delimiter_resolutions: Vec<DelimiterResolution>,
    pub recommended_actions: Vec<RecommendedAction>,
}

#[derive(Debug)]
pub struct DelimiterResolution {
    pub expression: String,
    pub resolved_to: Option<String>,
    pub confidence: f32,
    pub method: String,
}

#[derive(Debug)]
pub enum RecommendedAction {
    RefactorCode {
        reason: String,
        suggestion: String,
    },
    EnableFeature {
        feature: String,
        risk_level: RiskLevel,
    },
    ManualReview {
        reason: String,
    },
    RunInSandbox {
        command: String,
    },
}

#[derive(Debug)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl EdgeCaseHandler {
    pub fn new(config: EdgeCaseConfig) -> Self {
        Self {
            anti_pattern_detector: AntiPatternDetector::new(),
            phase_parser: PhaseAwareParser::new(),
            delimiter_recovery: DynamicDelimiterRecovery::new(config.recovery_mode.clone()),
            config,
        }
    }

    /// Analyze code for all edge cases
    pub fn analyze(&mut self, code: &str) -> EdgeCaseAnalysis {
        let mut diagnostics = Vec::new();
        let mut phase_warnings = Vec::new();
        let delimiter_resolutions = Vec::new();
        let mut recommended_actions = Vec::new();

        // Phase 1: Anti-pattern detection
        let anti_patterns = self.anti_pattern_detector.detect_all(code);
        diagnostics.extend(anti_patterns);

        // Phase 2: Phase analysis
        let phase_transitions = self.phase_parser.analyze_phases(code);
        for transition in phase_transitions {
            if matches!(transition.to, PerlPhase::Begin) {
                phase_warnings.push(format!(
                    "Entering {} block at line {} - heredocs may have compile-time effects",
                    match transition.to {
                        PerlPhase::Begin => "BEGIN",
                        PerlPhase::Check => "CHECK",
                        _ => "phase",
                    },
                    transition.line
                ));
            }
        }

        // Phase 3: Dynamic delimiter analysis
        self.delimiter_recovery.scan_for_assignments(code);

        // Phase 4: Integrated parsing with understanding
        let mut understanding_parser = UnderstandingParser::new();
        let parse_result = understanding_parser
            .parse_with_understanding(code)
            .unwrap_or_else(|e| panic!("Parse failed: {}", e));

        // Phase 5: Generate recommendations
        self.generate_recommendations(&mut recommended_actions, &diagnostics);

        // Add phase diagnostics
        diagnostics.extend(self.phase_parser.generate_phase_diagnostics());

        EdgeCaseAnalysis {
            ast: parse_result.ast,
            diagnostics,
            phase_warnings,
            delimiter_resolutions,
            recommended_actions,
        }
    }

    /// Handle a specific dynamic delimiter case
    pub fn handle_dynamic_delimiter(
        &self,
        expression: &str,
        context: &ParseContext,
    ) -> DelimiterResolution {
        let analysis = self
            .delimiter_recovery
            .analyze_dynamic_delimiter(expression, context);

        DelimiterResolution {
            expression: expression.to_string(),
            resolved_to: analysis.delimiter,
            confidence: analysis.confidence,
            method: analysis.recovery_strategy,
        }
    }

    /// Generate recommendations based on detected issues
    fn generate_recommendations(
        &self,
        actions: &mut Vec<RecommendedAction>,
        diagnostics: &[Diagnostic],
    ) {
        let mut has_dynamic_delimiters = false;
        let mut has_begin_heredocs = false;
        let mut has_source_filters = false;

        for diag in diagnostics {
            match &diag.pattern {
                crate::anti_pattern_detector::AntiPattern::DynamicHeredocDelimiter { .. } => {
                    has_dynamic_delimiters = true;
                }
                crate::anti_pattern_detector::AntiPattern::BeginTimeHeredoc { .. } => {
                    has_begin_heredocs = true;
                }
                crate::anti_pattern_detector::AntiPattern::SourceFilterHeredoc { .. } => {
                    has_source_filters = true;
                }
                _ => {}
            }
        }

        if has_dynamic_delimiters {
            actions.push(RecommendedAction::RefactorCode {
                reason: "Dynamic heredoc delimiters prevent static analysis".to_string(),
                suggestion:
                    "Use static delimiters with variable interpolation inside the heredoc body"
                        .to_string(),
            });

            if self.config.enable_sandbox {
                actions.push(RecommendedAction::RunInSandbox {
                    command: "perl-sandbox --resolve-delimiters".to_string(),
                });
            }
        }

        if has_begin_heredocs {
            actions.push(RecommendedAction::RefactorCode {
                reason: "BEGIN-time heredocs can modify compile-time state".to_string(),
                suggestion: "Move heredoc initialization to INIT block or runtime".to_string(),
            });
        }

        if has_source_filters {
            actions.push(RecommendedAction::ManualReview {
                reason: "Source filters can arbitrarily transform code".to_string(),
            });

            actions.push(RecommendedAction::EnableFeature {
                feature: "source-filter-simulation".to_string(),
                risk_level: RiskLevel::High,
            });
        }
    }

    /// Generate a comprehensive report
    pub fn generate_report(&self, analysis: &EdgeCaseAnalysis) -> String {
        let mut report = String::new();

        report.push_str("=== Perl Heredoc Edge Case Analysis ===\n\n");

        // Summary
        report.push_str(&format!("Total Issues: {}\n", analysis.diagnostics.len()));
        report.push_str(&format!(
            "Phase Warnings: {}\n",
            analysis.phase_warnings.len()
        ));
        report.push_str(&format!(
            "Dynamic Delimiters: {}\n",
            analysis.delimiter_resolutions.len()
        ));

        // Phase warnings
        if !analysis.phase_warnings.is_empty() {
            report.push_str("\n## Phase-Related Warnings\n");
            for warning in &analysis.phase_warnings {
                report.push_str(&format!("- {}\n", warning));
            }
        }

        // Delimiter resolutions
        if !analysis.delimiter_resolutions.is_empty() {
            report.push_str("\n## Dynamic Delimiter Analysis\n");
            for resolution in &analysis.delimiter_resolutions {
                report.push_str(&format!(
                    "- Expression '{}' {} (confidence: {:.0}%)\n",
                    resolution.expression,
                    if let Some(ref delim) = resolution.resolved_to {
                        format!("resolved to '{}'", delim)
                    } else {
                        "could not be resolved".to_string()
                    },
                    resolution.confidence * 100.0
                ));
            }
        }

        // Recommendations
        if !analysis.recommended_actions.is_empty() {
            report.push_str("\n## Recommended Actions\n");
            for (i, action) in analysis.recommended_actions.iter().enumerate() {
                report.push_str(&format!("{}. {}\n", i + 1, self.format_action(action)));
            }
        }

        // Anti-pattern details
        if !analysis.diagnostics.is_empty() {
            report.push_str("\n## Detailed Diagnostics\n");
            report.push_str(
                &self
                    .anti_pattern_detector
                    .format_report(&analysis.diagnostics),
            );
        }

        report
    }

    fn format_action(&self, action: &RecommendedAction) -> String {
        match action {
            RecommendedAction::RefactorCode { reason, suggestion } => {
                format!("Refactor: {} - {}", reason, suggestion)
            }
            RecommendedAction::EnableFeature {
                feature,
                risk_level,
            } => {
                format!("Enable Feature: {} (Risk: {:?})", feature, risk_level)
            }
            RecommendedAction::ManualReview { reason } => {
                format!("Manual Review Required: {}", reason)
            }
            RecommendedAction::RunInSandbox { command } => {
                format!("Run in Sandbox: `{}`", command)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integrated_analysis() {
        let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
        let code = r#"
BEGIN {
    my $delimiter = "EOF";
    $config = <<$delimiter;
    Dynamic content in BEGIN
EOF
}

format REPORT =
<<'END'
Format heredoc
END
.
"#;

        let analysis = handler.analyze(code);
        assert!(!analysis.diagnostics.is_empty());
        assert!(!analysis.phase_warnings.is_empty());
        assert!(!analysis.recommended_actions.is_empty());
    }
}
