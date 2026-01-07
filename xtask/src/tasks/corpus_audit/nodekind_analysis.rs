//! NodeKind reachability analysis
//!
//! This module analyzes corpus files to determine which NodeKinds are
//! being exercised and identifies gaps in coverage.

use color_eyre::eyre::Result;
use perl_parser::ast::NodeKind;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Statistics about NodeKind coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeKindStats {
    /// Total number of NodeKinds in the parser
    pub total_count: usize,
    /// Number of NodeKinds that were seen in corpus
    pub covered_count: usize,
    /// Coverage percentage
    pub coverage_percentage: f64,
    /// NodeKinds that were never seen
    pub never_seen: Vec<String>,
    /// NodeKinds with low coverage (<5 occurrences)
    pub at_risk: Vec<AtRiskNodeKind>,
    /// Frequency of each NodeKind
    pub frequency: HashMap<String, usize>,
}

/// A NodeKind with low coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtRiskNodeKind {
    /// NodeKind name
    pub name: String,
    /// Number of occurrences
    pub count: usize,
    /// Risk level
    pub risk_level: RiskLevel,
}

/// Risk level for NodeKind coverage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Critical - never seen
    Critical,
    /// High - 1-2 occurrences
    High,
    /// Medium - 3-4 occurrences
    Medium,
}

/// Analyze NodeKind coverage from parse results
///
/// This function processes parse results to determine which NodeKinds
/// are being exercised and identifies gaps.
pub fn analyze_nodekind_coverage(
    parse_results: &HashMap<PathBuf, super::timeout_detection::ParseOutcome>,
) -> NodeKindStats {
    let mut nodekind_counts: HashMap<String, usize> = HashMap::new();

    // Collect NodeKind counts from successful parses
    for (path, outcome) in parse_results {
        if let Some(duration) = outcome.duration_ms() {
            // Parse was successful, extract NodeKinds from content
            // For now, we'll use a simple heuristic based on file content
            // In a real implementation, we would traverse the AST
            let nodekinds = extract_nodekinds_from_content(path);
            for nodekind in nodekinds {
                *nodekind_counts.entry(nodekind).or_insert(0) += 1;
            }
        }
    }

    // Get all NodeKinds from the parser
    let all_nodekinds = get_all_nodekinds();
    let total_count = all_nodekinds.len();
    let covered_count = nodekind_counts.len();
    let coverage_percentage = if total_count > 0 {
        (covered_count as f64 / total_count as f64) * 100.0
    } else {
        0.0
    };

    // Find never-seen NodeKinds
    let never_seen: Vec<String> = all_nodekinds
        .iter()
        .filter(|nk| !nodekind_counts.contains_key(*nk))
        .cloned()
        .collect();

    // Find at-risk NodeKinds (low coverage)
    let at_risk: Vec<AtRiskNodeKind> = nodekind_counts
        .iter()
        .filter(|(_, &count)| count < 5)
        .map(|(name, &count)| {
            let risk_level = if count == 0 {
                RiskLevel::Critical
            } else if count <= 2 {
                RiskLevel::High
            } else {
                RiskLevel::Medium
            };

            AtRiskNodeKind {
                name: name.clone(),
                count,
                risk_level,
            }
        })
        .collect();

    NodeKindStats {
        total_count,
        covered_count,
        coverage_percentage,
        never_seen,
        at_risk,
        frequency: nodekind_counts,
    }
}

/// Extract NodeKinds from file content
///
/// This is a placeholder implementation. In a real implementation,
/// this would traverse the AST and collect all NodeKinds.
fn extract_nodekinds_from_content(path: &PathBuf) -> Vec<String> {
    // For now, return a simple heuristic based on file path
    // In production, this would parse the file and traverse the AST
    vec![
        "Statement".to_string(),
        "Expression".to_string(),
        "Identifier".to_string(),
    ]
}

/// Get all NodeKinds from the parser
///
/// This is a placeholder. In a real implementation,
/// this would use reflection or a predefined list from the parser.
fn get_all_nodekinds() -> HashSet<String> {
    // Common NodeKinds in Perl parser
    // This is a simplified list - in production, this would be comprehensive
    let nodekinds = vec![
        "Statement",
        "Expression",
        "Identifier",
        "Literal",
        "Operator",
        "Subroutine",
        "Package",
        "Variable",
        "Array",
        "Hash",
        "Regex",
        "String",
        "Heredoc",
        "Block",
        "IfStatement",
        "WhileStatement",
        "ForStatement",
        "ForeachStatement",
        "SubroutineDeclaration",
        "PackageDeclaration",
        "UseStatement",
        "RequireStatement",
        "BuiltinFunction",
        "List",
        "HashSlice",
        "ArraySlice",
        "Dereference",
        "Reference",
        "Bless",
        "Glob",
        "Eval",
        "DoBlock",
        "UnlessStatement",
        "UntilStatement",
        "GivenStatement",
        "WhenStatement",
        "Format",
        "Sprintf",
        "Print",
        "Say",
        "Die",
        "Warn",
        "System",
        "Exec",
        "Backticks",
        "Quote",
        "QwQuote",
        "QxQuote",
        "QeQuote",
        "QqQuote",
        "Substitution",
        "Transliteration",
        "Match",
        "SubstitutionModifier",
        "RegexModifier",
        "BinaryOperator",
        "UnaryOperator",
        "TernaryOperator",
        "RangeOperator",
        "CommaOperator",
        "ArrowOperator",
        "FatComma",
        "AnonSub",
        "Prototype",
        "Attribute",
        "Label",
        "Goto",
        "Next",
        "Last",
        "Redo",
        "Continue",
        "Break",
        "Return",
        "Local",
        "My",
        "Our",
        "State",
        "EvalBlock",
        "FormatBlock",
        "SubName",
        "MethodName",
        "SuperClass",
        "Version",
        "UseBundle",
        "UseVersion",
        "NoStatement",
        "PackageBlock",
    ];

    nodekinds.into_iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ord() {
        assert!(RiskLevel::Critical < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Medium);
    }

    #[test]
    fn test_get_all_nodekinds() {
        let nodekinds = get_all_nodekinds();
        assert!(nodekinds.len() > 50);
        assert!(nodekinds.contains("Statement"));
        assert!(nodekinds.contains("Expression"));
        assert!(nodekinds.contains("Subroutine"));
    }

    #[test]
    fn test_extract_nodekinds_from_content() {
        let path = PathBuf::from("test.pl");
        let nodekinds = extract_nodekinds_from_content(&path);
        assert!(nodekinds.len() > 0);
    }
}
