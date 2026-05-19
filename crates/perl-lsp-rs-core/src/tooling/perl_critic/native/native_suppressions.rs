//! Native critic suppression directive parsing.

use super::native_contract::CriticFinding;
use serde::{Deserialize, Serialize};

/// Scope covered by a native critic suppression directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriticSuppressionScope {
    /// Suppression applies to the whole file.
    File,
}

/// Parsed native critic suppression directive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticSuppression {
    /// Suppressed rule ID.
    pub rule_id: String,
    /// Scope covered by this directive.
    pub scope: CriticSuppressionScope,
    /// Zero-based line where the directive appears.
    pub line: usize,
    /// Optional human reason after `--`.
    pub reason: Option<String>,
}

/// Parsed native critic suppressions for a source file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticSuppressionMap {
    suppressions: Vec<CriticSuppression>,
}

impl CriticSuppressionMap {
    /// Parse native critic suppression directives from source text.
    #[must_use]
    pub fn from_source(source: &str) -> Self {
        let suppressions = source
            .lines()
            .enumerate()
            .flat_map(|(line, text)| parse_suppression_line(line, text))
            .collect();

        Self { suppressions }
    }

    /// Parsed suppression records.
    #[must_use]
    pub fn suppressions(&self) -> &[CriticSuppression] {
        &self.suppressions
    }

    /// Whether this map suppresses a native critic finding.
    #[must_use]
    pub fn suppresses(&self, finding: &CriticFinding) -> bool {
        self.suppressions.iter().any(|suppression| {
            suppression.rule_id == finding.rule_id || suppression.rule_id == finding.suppression_key
        })
    }
}

fn parse_suppression_line(line: usize, text: &str) -> Vec<CriticSuppression> {
    const NO_CRITIC: &str = "## no critic ";
    const NO_NATIVE_CRITIC: &str = "## no perl-lsp-critic ";

    let trimmed = text.trim_start();
    let Some(rest) =
        trimmed.strip_prefix(NO_CRITIC).or_else(|| trimmed.strip_prefix(NO_NATIVE_CRITIC))
    else {
        return Vec::new();
    };

    let (rules, reason) = rest.split_once("--").map_or((rest, None), |(rules, reason)| {
        let reason = reason.trim();
        (rules, (!reason.is_empty()).then(|| reason.to_string()))
    });

    rules
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter_map(|rule_id| {
            let rule_id = rule_id.trim();
            (!rule_id.is_empty()).then(|| CriticSuppression {
                rule_id: rule_id.to_string(),
                scope: CriticSuppressionScope::File,
                line,
                reason: reason.clone(),
            })
        })
        .collect()
}
