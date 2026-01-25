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
    use perl_parser::{Parser, ast::Node};
    use std::fs;

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut parser = Parser::new(&content);
    let mut nodekinds = HashSet::new();

    if let Ok(ast) = parser.parse() {
        collect_nodekinds_recursive(&ast, &mut nodekinds);
    }

    nodekinds.into_iter().collect()
}

fn collect_nodekinds_recursive(node: &perl_parser::ast::Node, out: &mut HashSet<String>) {
    out.insert(node.kind.kind_name().to_string());

    // Traverse children
    match &node.kind {
        perl_parser::ast::NodeKind::Program { statements }
        | perl_parser::ast::NodeKind::Block { statements } => {
            for stmt in statements {
                collect_nodekinds_recursive(stmt, out);
            }
        }
        perl_parser::ast::NodeKind::VariableDeclaration { initializer, .. } => {
            if let Some(init) = initializer {
                collect_nodekinds_recursive(init, out);
            }
        }
        perl_parser::ast::NodeKind::VariableListDeclaration { variables, initializer, .. } => {
            for var in variables {
                collect_nodekinds_recursive(var, out);
            }
            if let Some(init) = initializer {
                collect_nodekinds_recursive(init, out);
            }
        }
        perl_parser::ast::NodeKind::VariableWithAttributes { variable, .. } => {
            collect_nodekinds_recursive(variable, out);
        }
        perl_parser::ast::NodeKind::Assignment { lhs, rhs, .. } => {
            collect_nodekinds_recursive(lhs, out);
            collect_nodekinds_recursive(rhs, out);
        }
        perl_parser::ast::NodeKind::Binary { left, right, .. } => {
            collect_nodekinds_recursive(left, out);
            collect_nodekinds_recursive(right, out);
        }
        perl_parser::ast::NodeKind::Ternary { condition, then_expr, else_expr } => {
            collect_nodekinds_recursive(condition, out);
            collect_nodekinds_recursive(then_expr, out);
            collect_nodekinds_recursive(else_expr, out);
        }
        perl_parser::ast::NodeKind::Unary { operand, .. } => {
            collect_nodekinds_recursive(operand, out);
        }
        perl_parser::ast::NodeKind::ArrayLiteral { elements } => {
            for el in elements {
                collect_nodekinds_recursive(el, out);
            }
        }
        perl_parser::ast::NodeKind::HashLiteral { pairs } => {
            for (k, v) in pairs {
                collect_nodekinds_recursive(k, out);
                collect_nodekinds_recursive(v, out);
            }
        }
        perl_parser::ast::NodeKind::Eval { block } => {
            collect_nodekinds_recursive(block, out);
        }
        perl_parser::ast::NodeKind::Do { block } => {
            collect_nodekinds_recursive(block, out);
        }
        perl_parser::ast::NodeKind::Try { body, catch_blocks, finally_block } => {
            collect_nodekinds_recursive(body, out);
            for (_, block) in catch_blocks {
                collect_nodekinds_recursive(block, out);
            }
            if let Some(finally) = finally_block {
                collect_nodekinds_recursive(finally, out);
            }
        }
        perl_parser::ast::NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            collect_nodekinds_recursive(condition, out);
            collect_nodekinds_recursive(then_branch, out);
            for (cond, block) in elsif_branches {
                collect_nodekinds_recursive(cond, out);
                collect_nodekinds_recursive(block, out);
            }
            if let Some(branch) = else_branch {
                collect_nodekinds_recursive(branch, out);
            }
        }
        perl_parser::ast::NodeKind::LabeledStatement { statement, .. } => {
            collect_nodekinds_recursive(statement, out);
        }
        perl_parser::ast::NodeKind::While { condition, body, continue_block } => {
            collect_nodekinds_recursive(condition, out);
            collect_nodekinds_recursive(body, out);
            if let Some(cont) = continue_block {
                collect_nodekinds_recursive(cont, out);
            }
        }
        perl_parser::ast::NodeKind::Tie { variable, package, args } => {
            collect_nodekinds_recursive(variable, out);
            collect_nodekinds_recursive(package, out);
            for arg in args {
                collect_nodekinds_recursive(arg, out);
            }
        }
        perl_parser::ast::NodeKind::Untie { variable } => {
            collect_nodekinds_recursive(variable, out);
        }
        perl_parser::ast::NodeKind::For { init, condition, update, body, continue_block } => {
            if let Some(i) = init { collect_nodekinds_recursive(i, out); }
            if let Some(c) = condition { collect_nodekinds_recursive(c, out); }
            if let Some(u) = update { collect_nodekinds_recursive(u, out); }
            collect_nodekinds_recursive(body, out);
            if let Some(cont) = continue_block {
                collect_nodekinds_recursive(cont, out);
            }
        }
        perl_parser::ast::NodeKind::Foreach { variable, list, body } => {
            collect_nodekinds_recursive(variable, out);
            collect_nodekinds_recursive(list, out);
            collect_nodekinds_recursive(body, out);
        }
        perl_parser::ast::NodeKind::Given { expr, body } => {
            collect_nodekinds_recursive(expr, out);
            collect_nodekinds_recursive(body, out);
        }
        perl_parser::ast::NodeKind::When { condition, body } => {
            collect_nodekinds_recursive(condition, out);
            collect_nodekinds_recursive(body, out);
        }
        perl_parser::ast::NodeKind::Default { body } => {
            collect_nodekinds_recursive(body, out);
        }
        perl_parser::ast::NodeKind::StatementModifier { statement, condition, .. } => {
            collect_nodekinds_recursive(statement, out);
            collect_nodekinds_recursive(condition, out);
        }
        perl_parser::ast::NodeKind::Subroutine { prototype, signature, body, .. } => {
            if let Some(p) = prototype { collect_nodekinds_recursive(p, out); }
            if let Some(s) = signature { collect_nodekinds_recursive(s, out); }
            collect_nodekinds_recursive(body, out);
        }
        perl_parser::ast::NodeKind::Signature { parameters } => {
            for p in parameters {
                collect_nodekinds_recursive(p, out);
            }
        }
        perl_parser::ast::NodeKind::MandatoryParameter { variable } => {
            collect_nodekinds_recursive(variable, out);
        }
        perl_parser::ast::NodeKind::OptionalParameter { variable, default_value } => {
            collect_nodekinds_recursive(variable, out);
            collect_nodekinds_recursive(default_value, out);
        }
        perl_parser::ast::NodeKind::SlurpyParameter { variable } => {
            collect_nodekinds_recursive(variable, out);
        }
        perl_parser::ast::NodeKind::NamedParameter { variable } => {
            collect_nodekinds_recursive(variable, out);
        }
        perl_parser::ast::NodeKind::Method { signature, body, .. } => {
            if let Some(s) = signature { collect_nodekinds_recursive(s, out); }
            collect_nodekinds_recursive(body, out);
        }
        perl_parser::ast::NodeKind::Return { value } => {
            if let Some(v) = value { collect_nodekinds_recursive(v, out); }
        }
        perl_parser::ast::NodeKind::MethodCall { object, args, .. } => {
            collect_nodekinds_recursive(object, out);
            for arg in args {
                collect_nodekinds_recursive(arg, out);
            }
        }
        perl_parser::ast::NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                collect_nodekinds_recursive(arg, out);
            }
        }
        perl_parser::ast::NodeKind::IndirectCall { object, args, .. } => {
            collect_nodekinds_recursive(object, out);
            for arg in args {
                collect_nodekinds_recursive(arg, out);
            }
        }
        perl_parser::ast::NodeKind::Match { expr, .. } => {
            collect_nodekinds_recursive(expr, out);
        }
        perl_parser::ast::NodeKind::Substitution { expr, .. } => {
            collect_nodekinds_recursive(expr, out);
        }
        perl_parser::ast::NodeKind::Transliteration { expr, .. } => {
            collect_nodekinds_recursive(expr, out);
        }
        perl_parser::ast::NodeKind::Package { block, .. } => {
            if let Some(b) = block { collect_nodekinds_recursive(b, out); }
        }
        perl_parser::ast::NodeKind::PhaseBlock { block, .. } => {
            collect_nodekinds_recursive(block, out);
        }
        perl_parser::ast::NodeKind::Class { body, .. } => {
            collect_nodekinds_recursive(body, out);
        }
        _ => {} // Leaf nodes or simple variants
    }
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
        assert!(!nodekinds.is_empty());
    }
}
