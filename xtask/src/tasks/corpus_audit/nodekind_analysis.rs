//! NodeKind reachability analysis
//!
//! This module analyzes corpus files to determine which NodeKinds are
//! being exercised and identifies gaps in coverage.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
        if let Some(_duration) = outcome.duration_ms() {
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
    let coverage_percentage =
        if total_count > 0 { (covered_count as f64 / total_count as f64) * 100.0 } else { 0.0 };

    // Find never-seen NodeKinds
    let never_seen: Vec<String> =
        all_nodekinds.iter().filter(|nk| !nodekind_counts.contains_key(*nk)).cloned().collect();

    // Find at-risk NodeKinds (low coverage)
    let at_risk: Vec<AtRiskNodeKind> = nodekind_counts
        .iter()
        .filter(|(_, count)| **count < 5)
        .map(|(name, count)| {
            let count = *count;
            let risk_level = if count == 0 {
                RiskLevel::Critical
            } else if count <= 2 {
                RiskLevel::High
            } else {
                RiskLevel::Medium
            };

            AtRiskNodeKind { name: name.clone(), count, risk_level }
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
/// This implementation parses the file and traverses the AST to collect
/// all unique NodeKind names.
fn extract_nodekinds_from_content(path: &PathBuf) -> Vec<String> {
    use perl_parser::Parser;
    use std::fs;

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut parser = Parser::new(&content);
    let mut nodekinds = HashSet::new();

    if let Ok(ast) = parser.parse() {
        collect_nodekinds_recursive(&ast, &mut nodekinds);
    } else {
        eprintln!("   Warning: Failed to parse {}", path.display());
    }

    let result: Vec<String> = nodekinds.into_iter().collect();
    if path.to_string_lossy().contains("basic_constructs") {
        eprintln!("   Debug: NodeKinds in basic_constructs.pl: {:?}", result);
    }
    result
}

fn collect_nodekinds_recursive(node: &perl_parser::ast::Node, out: &mut HashSet<String>) {
    out.insert(node.kind.kind_name().to_string());

    // Traverse children using robust API
    node.for_each_child(|child| {
        collect_nodekinds_recursive(child, out);
    });
}

/// Get all NodeKinds from the parser
///
/// This is a placeholder. In a real implementation,
/// this would use reflection or a predefined list from the parser.
fn get_all_nodekinds() -> HashSet<String> {
    // Comprehensive list of NodeKinds from crates/perl-ast/src/ast.rs
    let nodekinds = vec![
        "Program",
        "ExpressionStatement",
        "VariableDeclaration",
        "VariableListDeclaration",
        "Variable",
        "VariableWithAttributes",
        "Assignment",
        "Binary",
        "Ternary",
        "Unary",
        "Diamond",
        "Ellipsis",
        "Undef",
        "Readline",
        "Glob",
        "Typeglob",
        "Number",
        "String",
        "Heredoc",
        "ArrayLiteral",
        "HashLiteral",
        "Block",
        "Eval",
        "Do",
        "Try",
        "If",
        "LabeledStatement",
        "While",
        "Tie",
        "Untie",
        "For",
        "Foreach",
        "Given",
        "When",
        "Default",
        "StatementModifier",
        "Subroutine",
        "Prototype",
        "Signature",
        "MandatoryParameter",
        "OptionalParameter",
        "SlurpyParameter",
        "NamedParameter",
        "Method",
        "Return",
        "LoopControl",
        "MethodCall",
        "FunctionCall",
        "IndirectCall",
        "Regex",
        "Match",
        "Substitution",
        "Transliteration",
        "Package",
        "Use",
        "No",
        "PhaseBlock",
        "DataSection",
        "Class",
        "Format",
        "Identifier",
        "Error",
        "HeredocDepthLimit",
        "MissingExpression",
        "MissingStatement",
        "MissingIdentifier",
        "MissingBlock",
        "UnknownRest",
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
        assert!(nodekinds.contains("ExpressionStatement"));
        assert!(nodekinds.contains("Binary"));
        assert!(nodekinds.contains("Subroutine"));
    }

    #[test]
    #[ignore = "corpus audit: test file 'test.pl' may not exist"]
    fn test_extract_nodekinds_from_content() {
        let path = PathBuf::from("test.pl");
        let nodekinds = extract_nodekinds_from_content(&path);
        assert!(!nodekinds.is_empty());
    }
}
