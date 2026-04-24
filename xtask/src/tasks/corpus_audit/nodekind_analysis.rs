//! NodeKind reachability analysis
//!
//! This module analyzes corpus files to determine which NodeKinds are
//! being exercised and identifies gaps in coverage.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// A NodeKind intentionally excluded from deterministic clean-corpus coverage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistedNodeKind {
    /// NodeKind variant name.
    pub name: String,
    /// Why this variant is intentionally unreachable in clean corpus parsing.
    pub rationale: String,
}

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
    /// NodeKinds that are never expected in deterministic clean corpus parsing.
    #[serde(default)]
    pub allowlisted_never_seen: Vec<AllowlistedNodeKind>,
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

    // Get all NodeKinds from the parser and split allowlisted unreachable variants.
    let all_nodekinds = get_all_nodekinds();
    let allowlist = nodekind_allowlist();
    let allowlisted_names: HashSet<&str> =
        allowlist.iter().map(|entry| entry.name.as_str()).collect();
    let total_count = all_nodekinds.len();
    let raw_covered_count = nodekind_counts.len();
    let covered_count = raw_covered_count + allowlist.len();
    let coverage_percentage =
        if total_count > 0 { (covered_count as f64 / total_count as f64) * 100.0 } else { 0.0 };

    // Find never-seen NodeKinds
    let never_seen: Vec<String> = all_nodekinds
        .iter()
        .filter(|nk| !nodekind_counts.contains_key(*nk) && !allowlisted_names.contains(nk.as_str()))
        .cloned()
        .collect();

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
        allowlisted_never_seen: allowlist,
        at_risk,
        frequency: nodekind_counts,
    }
}

/// NodeKinds intentionally absent from deterministic clean-corpus parsing.
fn nodekind_allowlist() -> Vec<AllowlistedNodeKind> {
    vec![
        AllowlistedNodeKind {
            name: "MissingStatement".to_string(),
            rationale: "Recovery sentinel inserted only when a statement is missing due to syntax errors; clean corpus fixtures avoid malformed statement boundaries.".to_string(),
        },
        AllowlistedNodeKind {
            name: "MissingIdentifier".to_string(),
            rationale: "Recovery sentinel emitted only when identifier tokens are absent at required grammar sites; deterministic clean corpus contains valid identifiers.".to_string(),
        },
        AllowlistedNodeKind {
            name: "MissingBlock".to_string(),
            rationale: "Recovery sentinel used when required block delimiters are missing; clean corpus excludes those malformed constructs.".to_string(),
        },
        AllowlistedNodeKind {
            name: "UnknownRest".to_string(),
            rationale: "Catch-all recovery sentinel for unknown trailing parser state, only reachable through invalid or incomplete syntax not present in clean corpus.".to_string(),
        },
    ]
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

    nodekinds.into_iter().collect()
}

fn collect_nodekinds_recursive(node: &perl_parser::ast::Node, out: &mut HashSet<String>) {
    out.insert(node.kind.kind_name().to_string());

    // Traverse children using robust API
    node.for_each_child(|child| {
        collect_nodekinds_recursive(child, out);
    });
}

/// Get all NodeKinds from the parser's canonical list.
fn get_all_nodekinds() -> HashSet<String> {
    perl_parser::ast::NodeKind::ALL_KIND_NAMES.iter().map(|s| (*s).to_string()).collect()
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
    fn test_extract_nodekinds_from_content() -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new()?;
        writeln!(tmp, "my $x = 1;\nprint $x;\nsub foo {{ return 42; }}")?;
        let path = PathBuf::from(tmp.path());
        let nodekinds = extract_nodekinds_from_content(&path);
        assert!(!nodekinds.is_empty());
        Ok(())
    }

    #[test]
    fn test_nodekind_allowlist_entries_are_canonical() {
        let all_nodekinds = get_all_nodekinds();
        let allowlist = nodekind_allowlist();
        assert_eq!(allowlist.len(), 4);
        for entry in &allowlist {
            assert!(
                all_nodekinds.contains(entry.name.as_str()),
                "allowlisted NodeKind {} must exist in canonical list",
                entry.name
            );
            assert!(
                !entry.rationale.is_empty(),
                "allowlisted NodeKind {} must include rationale",
                entry.name
            );
        }
    }
}
