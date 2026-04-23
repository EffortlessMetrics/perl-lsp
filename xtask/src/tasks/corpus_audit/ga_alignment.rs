//! GA (General Availability) feature-to-fixture alignment
//!
//! This module checks alignment between GA features and corpus fixtures,
//! identifying gaps in feature coverage.

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

use super::corpus::CorpusFile;

/// A GA feature that should be tested in the corpus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAFeature {
    /// Feature identifier
    pub id: String,
    /// Feature name
    pub name: String,
    /// Feature priority (P0 = critical, P1 = high, P2 = medium)
    pub priority: FeaturePriority,
    /// Expected NodeKinds for this feature
    pub expected_nodekinds: Vec<String>,
    /// Description of the feature
    pub description: String,
}

/// Priority level for GA features
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeaturePriority {
    /// P0 - Critical feature, must be covered
    P0,
    /// P1 - High priority feature
    P1,
    /// P2 - Medium priority feature
    P2,
}

impl FeaturePriority {
    /// Get priority as a numeric value (0 = highest priority)
    #[allow(dead_code)]
    pub fn as_numeric(&self) -> usize {
        match self {
            FeaturePriority::P0 => 0,
            FeaturePriority::P1 => 1,
            FeaturePriority::P2 => 2,
        }
    }
}

/// Coverage status for a GA feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureCoverage {
    /// Feature being checked
    pub feature: GAFeature,
    /// Whether the feature is covered
    pub covered: bool,
    /// Files that cover this feature
    pub covering_files: Vec<String>,
    /// Coverage percentage (0-100)
    pub coverage_percentage: f64,
}

/// Overall GA feature coverage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAFeatureCoverage {
    /// Total number of GA features
    pub total_count: usize,
    /// Number of features with coverage
    pub covered_count: usize,
    /// Coverage percentage
    pub coverage_percentage: f64,
    /// Coverage status for each feature
    pub features: Vec<FeatureCoverage>,
    /// Features with no coverage (P0 and P1)
    pub uncovered_critical: Vec<String>,
    /// Features with partial coverage
    pub uncovered_partial: Vec<String>,
}

/// Check GA feature alignment with corpus files
///
/// This function analyzes corpus files to determine which GA features
/// are covered and identifies gaps.
pub fn check_ga_feature_alignment(files: &[CorpusFile]) -> Result<GAFeatureCoverage> {
    // Define GA features to check
    let ga_features = define_ga_features();

    // Check coverage for each feature
    let features: Vec<FeatureCoverage> =
        ga_features.iter().map(|feature| check_feature_coverage(feature, files)).collect();

    // Calculate overall statistics
    let total_count = features.len();
    let covered_count = features.iter().filter(|f| f.covered).count();
    let coverage_percentage =
        if total_count > 0 { (covered_count as f64 / total_count as f64) * 100.0 } else { 0.0 };

    // Identify uncovered critical features (P0 and P1)
    let uncovered_critical: Vec<String> = features
        .iter()
        .filter(|f| {
            !f.covered && matches!(f.feature.priority, FeaturePriority::P0 | FeaturePriority::P1)
        })
        .map(|f| f.feature.id.clone())
        .collect();

    // Identify features with partial coverage
    let uncovered_partial: Vec<String> = features
        .iter()
        .filter(|f| f.covered && f.coverage_percentage < 50.0)
        .map(|f| f.feature.id.clone())
        .collect();

    Ok(GAFeatureCoverage {
        total_count,
        covered_count,
        coverage_percentage,
        features,
        uncovered_critical,
        uncovered_partial,
    })
}

/// Check coverage for a single GA feature
fn check_feature_coverage(feature: &GAFeature, files: &[CorpusFile]) -> FeatureCoverage {
    let mut covering_files = Vec::new();

    for file in files {
        // Check if file covers this feature
        // For now, use a simple heuristic based on content
        if file_covers_feature(file, feature) {
            covering_files.push(file.path.display().to_string());
        }
    }

    let covered = !covering_files.is_empty();
    let coverage_percentage = if covered {
        // Simple heuristic: if covered, assume 100%
        100.0
    } else {
        0.0
    };

    FeatureCoverage { feature: feature.clone(), covered, covering_files, coverage_percentage }
}

/// Check if a file covers a given GA feature
fn file_covers_feature(file: &CorpusFile, feature: &GAFeature) -> bool {
    // For now, use simple heuristics based on content
    // In production, this would parse the file and check for specific patterns

    let content = &file.content;

    // Check for expected NodeKinds
    for nodekind in &feature.expected_nodekinds {
        // Simple heuristic: check if content contains patterns related to nodekind
        if content_matches_nodekind(content, nodekind) {
            return true;
        }
    }

    false
}

/// Check if content matches a NodeKind pattern
fn content_matches_nodekind(content: &str, nodekind: &str) -> bool {
    // Simple heuristics for common NodeKinds
    match nodekind {
        "If" => content.contains("if ") || content.contains("unless "),
        "Unless" => content.contains("unless "),
        "While" => content.contains("while "),
        "For" => content.contains("for "),
        "Foreach" => content.contains("foreach "),
        "Subroutine" => content.contains("sub "),
        "Package" => content.contains("package "),
        "Use" => content.contains("use "),
        "Regex" => content.contains("m/") || content.contains("qr/"),
        "Substitution" => content.contains("s/"),
        "Heredoc" => content.contains("<<"),
        "HashLiteral" => content.contains("%") && (content.contains('{') || content.contains('(')),
        "ArrayLiteral" => content.contains("@") && (content.contains('(') || content.contains('[')),
        "FunctionCall" => {
            let builtins =
                ["map ", "grep ", "sort ", "push ", "pop ", "shift ", "print ", "say ", "sprintf "];
            builtins.iter().any(|b| content.contains(b))
        }
        "Given" => content.contains("given "),
        "When" => content.contains("when "),
        "Default" => content.contains("default "),
        "Format" => content.contains("format "),
        _ => false,
    }
}

/// Define GA features to check
///
/// This is a placeholder list. In production, this would be
/// loaded from a configuration file or derived from the parser.
fn define_ga_features() -> Vec<GAFeature> {
    vec![
        GAFeature {
            id: "control-flow-if".to_string(),
            name: "If/Unless Statements".to_string(),
            priority: FeaturePriority::P0,
            expected_nodekinds: vec!["If".to_string(), "Unless".to_string()],
            description: "Conditional control flow with if/unless".to_string(),
        },
        GAFeature {
            id: "control-flow-loops".to_string(),
            name: "Loop Statements".to_string(),
            priority: FeaturePriority::P0,
            expected_nodekinds: vec!["While".to_string(), "For".to_string(), "Foreach".to_string()],
            description: "Loop control flow with while/for/foreach".to_string(),
        },
        GAFeature {
            id: "subroutines".to_string(),
            name: "Subroutine Declarations".to_string(),
            priority: FeaturePriority::P0,
            expected_nodekinds: vec!["Subroutine".to_string()],
            description: "Named subroutine declarations".to_string(),
        },
        GAFeature {
            id: "packages".to_string(),
            name: "Package Declarations".to_string(),
            priority: FeaturePriority::P0,
            expected_nodekinds: vec!["Package".to_string()],
            description: "Package namespace declarations".to_string(),
        },
        GAFeature {
            id: "regex".to_string(),
            name: "Regular Expressions".to_string(),
            priority: FeaturePriority::P0,
            expected_nodekinds: vec!["Regex".to_string()],
            description: "Pattern matching with regular expressions".to_string(),
        },
        GAFeature {
            id: "substitution".to_string(),
            name: "Substitution Operators".to_string(),
            priority: FeaturePriority::P0,
            expected_nodekinds: vec!["Substitution".to_string()],
            description: "String substitution with s/// operator".to_string(),
        },
        GAFeature {
            id: "heredocs".to_string(),
            name: "Heredocs".to_string(),
            priority: FeaturePriority::P1,
            expected_nodekinds: vec!["Heredoc".to_string()],
            description: "Multi-line string literals with heredocs".to_string(),
        },
        GAFeature {
            id: "hashes".to_string(),
            name: "Hash Data Structures".to_string(),
            priority: FeaturePriority::P1,
            expected_nodekinds: vec!["HashLiteral".to_string()],
            description: "Associative arrays (hashes)".to_string(),
        },
        GAFeature {
            id: "arrays".to_string(),
            name: "Array Data Structures".to_string(),
            priority: FeaturePriority::P1,
            expected_nodekinds: vec!["ArrayLiteral".to_string()],
            description: "Indexed arrays".to_string(),
        },
        GAFeature {
            id: "builtin-functions".to_string(),
            name: "Builtin Functions".to_string(),
            priority: FeaturePriority::P0,
            expected_nodekinds: vec!["FunctionCall".to_string()],
            description: "Common builtin functions (map, grep, sort, etc.)".to_string(),
        },
        GAFeature {
            id: "match-given".to_string(),
            name: "Match/Given Statements".to_string(),
            priority: FeaturePriority::P1,
            expected_nodekinds: vec![
                "Given".to_string(),
                "When".to_string(),
                "Default".to_string(),
            ],
            description: "Pattern matching with match/given/when".to_string(),
        },
        GAFeature {
            id: "format-sprintf".to_string(),
            name: "Format/Sprintf".to_string(),
            priority: FeaturePriority::P2,
            expected_nodekinds: vec!["Format".to_string(), "FunctionCall".to_string()],
            description: "String formatting functions".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_priority_ord() {
        assert!(FeaturePriority::P0.as_numeric() < FeaturePriority::P1.as_numeric());
        assert!(FeaturePriority::P1.as_numeric() < FeaturePriority::P2.as_numeric());
    }

    #[test]
    fn test_define_ga_features() {
        let features = define_ga_features();
        assert!(features.len() > 10);
        assert!(features.iter().any(|f| f.id == "control-flow-if"));
        assert!(features.iter().any(|f| f.id == "regex"));
    }

    #[test]
    fn test_content_matches_nodekind() {
        assert!(content_matches_nodekind("if ($x) { print $x; }", "If"));
        assert!(content_matches_nodekind("while (1) { print; }", "While"));
        assert!(content_matches_nodekind("sub foo { print; }", "Subroutine"));
        assert!(content_matches_nodekind("package Foo;", "Package"));
        assert!(content_matches_nodekind("m/pattern/", "Regex"));
        assert!(content_matches_nodekind("s/foo/bar/", "Substitution"));
        assert!(content_matches_nodekind("map { $_ } @list", "FunctionCall"));
    }
}
